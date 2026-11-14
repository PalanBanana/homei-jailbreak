//! VM backend adapter for the `darwin-vm` / `qemu-sptm` project.
//!
//! HOMEI does not vendor or modify upstream code. It pins upstream commits, detects whether the
//! pieces are present, builds a launch plan, and classifies what a boot attempt showed. Nothing
//! starts a VM unless the caller explicitly asks for it, and a launch refuses to run unless the
//! preflight passed.

use crate::hash::sha256_file;
use crate::host::{has_machine, parse_accelerators, parse_qemu_version};
use crate::process::{ProcessRunner, RunStatus};
use homei_core::fidelity::{BackendIdentity, BootObservation, BootOutcome, LabEvidence};
use homei_core::labstate::BackendStatus;
use homei_core::target::{TargetManifest, VmStatus};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Pinned upstream references. Reading them does not make them trusted: they are what was inspected.
pub const DARWIN_VM_COMMIT: &str = "edfc19433da8";
pub const QEMU_SPTM_COMMIT: &str = "bad6336fb242";
/// The machine type the fork provides. Stock QEMU does not have it.
pub const DARWIN_MACHINE: &str = "darwin";
/// The upstream boot arguments. Kept as the documented default, never extended with network options.
pub const DEFAULT_BOOT_ARGS: &str = "rd=md0 serial=3 -v -noprogress wdt=-1 wlan-olyhal-abort";
pub const DEFAULT_MEMORY_GIB: u32 = 8;

/// Firmware files the backend needs. The monitor images are required for an SPTM kernel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FirmwareFile {
    pub name: &'static str,
    pub required: bool,
    pub role: &'static str,
}

pub const FIRMWARE_FILES: &[FirmwareFile] = &[
    FirmwareFile {
        name: "bootkc",
        required: true,
        role: "kernelcache",
    },
    FirmwareFile {
        name: "dtree",
        required: true,
        role: "device tree",
    },
    FirmwareFile {
        name: "ramdisk.dmg",
        required: true,
        role: "root filesystem ramdisk",
    },
    FirmwareFile {
        name: "ramdisk.tc",
        required: true,
        role: "trust cache for the ramdisk",
    },
    FirmwareFile {
        name: "sptm",
        required: true,
        role: "secure page table monitor (SPTM kernels)",
    },
    FirmwareFile {
        name: "txm",
        required: true,
        role: "trusted execution monitor (SPTM kernels)",
    },
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreflightFile {
    pub name: String,
    pub role: String,
    pub present: bool,
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreflightReport {
    pub qemu_found: bool,
    pub qemu_version: Option<String>,
    pub accelerators: Vec<String>,
    pub darwin_machine_available: bool,
    pub firmware: Vec<PreflightFile>,
    pub memory_available_gib: u32,
    pub memory_requested_gib: u32,
    pub blockers: Vec<String>,
    pub uses_mock_runner: bool,
}

impl PreflightReport {
    pub fn firmware_complete(&self) -> bool {
        self.firmware.iter().all(|f| f.present)
    }

    /// Everything needed to start a guest is present. It does not say the guest will boot.
    pub fn can_launch(&self) -> bool {
        self.qemu_found
            && self.darwin_machine_available
            && self.firmware_complete()
            && self.memory_available_gib >= self.memory_requested_gib
    }
}

/// Inspect the environment without starting a VM. Runs `--version`, `-M help` and `-accel help` only.
pub fn preflight(
    runner: &dyn ProcessRunner,
    qemu: &str,
    firmware_dir: &Path,
    memory_available_gib: u32,
    memory_requested_gib: u32,
    hash_files: bool,
) -> PreflightReport {
    let mut blockers = Vec::new();
    let version = runner.run(qemu, &["--version".to_string()], Duration::from_secs(10));
    let qemu_found = version.status == RunStatus::Exited(0);
    let (mut qemu_version, mut accelerators, mut machine) = (None, Vec::new(), false);
    if qemu_found {
        qemu_version = parse_qemu_version(&version.stdout);
        accelerators = parse_accelerators(
            &runner
                .run(
                    qemu,
                    &["-accel".to_string(), "help".to_string()],
                    Duration::from_secs(10),
                )
                .stdout,
        );
        machine = has_machine(
            &runner
                .run(
                    qemu,
                    &["-M".to_string(), "help".to_string()],
                    Duration::from_secs(10),
                )
                .stdout,
            DARWIN_MACHINE,
        );
        if !machine {
            blockers.push(format!(
                "this QEMU has no '{DARWIN_MACHINE}' machine type; the qemu-sptm fork is required"
            ));
        }
    } else {
        blockers.push("qemu-system-aarch64 was not found or did not run".to_string());
    }
    let firmware: Vec<PreflightFile> = FIRMWARE_FILES
        .iter()
        .map(|f| {
            let p: PathBuf = firmware_dir.join(f.name);
            let present = p.is_file();
            let sha256 = if present && hash_files {
                sha256_file(&p).ok()
            } else {
                None
            };
            PreflightFile {
                name: f.name.to_string(),
                role: f.role.to_string(),
                present,
                sha256,
            }
        })
        .collect();
    let missing: Vec<&str> = firmware
        .iter()
        .filter(|f| !f.present)
        .map(|f| f.name.as_str())
        .collect();
    if !missing.is_empty() {
        blockers.push(format!("missing firmware inputs: {}", missing.join(", ")));
    }
    if memory_available_gib < memory_requested_gib {
        blockers.push(format!(
            "{memory_available_gib} GiB available, {memory_requested_gib} GiB requested"
        ));
    }
    if !accelerators.iter().any(|a| a == "tcg") && qemu_found {
        blockers.push("QEMU reports no TCG accelerator".to_string());
    }
    PreflightReport {
        qemu_found,
        qemu_version,
        accelerators,
        darwin_machine_available: machine,
        firmware,
        memory_available_gib,
        memory_requested_gib,
        blockers,
        uses_mock_runner: runner.is_mock(),
    }
}

/// A launch command, as a list of arguments. Building it starts nothing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaunchPlan {
    pub program: String,
    pub args: Vec<String>,
    pub memory_gib: u32,
    pub timeout_secs: u64,
    pub notes: Vec<String>,
}

impl LaunchPlan {
    /// The command line a person can read. This is the dry run.
    pub fn command_line(&self) -> String {
        std::iter::once(self.program.clone())
            .chain(self.args.iter().map(|a| {
                if a.contains(' ') {
                    format!("'{a}'")
                } else {
                    a.clone()
                }
            }))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Build the launch plan. No network device, no host directory, no USB and no device passthrough is
/// ever added. The console is the serial port, captured to a log.
pub fn plan_launch(
    qemu: &str,
    firmware_dir: &str,
    memory_gib: u32,
    timeout_secs: u64,
) -> LaunchPlan {
    let f = |n: &str| format!("{firmware_dir}/{n}");
    let args = vec![
        "-M".to_string(),
        DARWIN_MACHINE.to_string(),
        "-bootkc".to_string(),
        f("bootkc"),
        "-dtree".to_string(),
        f("dtree"),
        "-tc".to_string(),
        f("ramdisk.tc"),
        "-ramdisk".to_string(),
        f("ramdisk.dmg"),
        "-sptm".to_string(),
        f("sptm"),
        "-txm".to_string(),
        f("txm"),
        "-args".to_string(),
        DEFAULT_BOOT_ARGS.to_string(),
        "-nographic".to_string(),
        "-serial".to_string(),
        "mon:stdio".to_string(),
        "-m".to_string(),
        format!("{memory_gib}G"),
    ];
    LaunchPlan {
        program: qemu.to_string(),
        args,
        memory_gib,
        timeout_secs,
        notes: vec![
            "No network device, host directory, USB or device passthrough is configured.".to_string(),
            "The guest root filesystem is the ramdisk image. Changes made in the guest are not written back to it.".to_string(),
        ],
    }
}

/// What a captured serial log shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogAnalysis {
    pub kernel_banner: Option<String>,
    pub reached_shell: bool,
    pub panicked: bool,
    pub lines: usize,
}

pub fn analyze_log(log: &str) -> LogAnalysis {
    let mut banner = None;
    let mut shell = false;
    let mut panicked = false;
    for line in log.lines() {
        if banner.is_none() && line.contains("Darwin Kernel Version") {
            banner = Some(line.trim().to_string());
        }
        if line.contains("bash-3.2#") || line.contains("bash-5") && line.contains('#') {
            shell = true;
        }
        let l = line.to_lowercase();
        if l.contains("panic(cpu")
            || l.contains("kernel panic")
            || l.starts_with("panic:")
            || l.contains("panicked")
        {
            panicked = true;
        }
    }
    LogAnalysis {
        kernel_banner: banner,
        reached_shell: shell,
        panicked,
        lines: log.lines().count(),
    }
}

/// Turn a process result and a log into a boot observation. A timeout, a panic or a log with no
/// kernel banner is never a success.
pub fn observe_boot(
    status: &RunStatus,
    log: &str,
    kernel_sha256: Option<String>,
    dtree: &BTreeMap<String, String>,
) -> BootObservation {
    let a = analyze_log(log);
    let outcome = match status {
        RunStatus::Timeout if a.kernel_banner.is_some() && !a.panicked => BootOutcome::Timeout,
        RunStatus::Timeout => BootOutcome::Timeout,
        RunStatus::NotFound | RunStatus::NotAllowed | RunStatus::Failed(_) => {
            BootOutcome::ProcessFailed
        }
        RunStatus::Exited(_) if a.panicked => BootOutcome::KernelPanic,
        RunStatus::Exited(code) if *code != 0 && a.kernel_banner.is_none() => {
            BootOutcome::ProcessFailed
        }
        RunStatus::Exited(_) if a.kernel_banner.is_none() => BootOutcome::IncompleteOutput,
        RunStatus::Exited(_) => BootOutcome::Booted,
    };
    // A guest that is still running when the timeout hits and has shown a shell counts as booted.
    let outcome = if outcome == BootOutcome::Timeout && a.reached_shell && !a.panicked {
        BootOutcome::Booted
    } else {
        outcome
    };
    BootObservation {
        outcome,
        kernel_sha256,
        guest_kernel_version: a.kernel_banner,
        device_tree_product: dtree.get("product").cloned(),
        device_tree_board: dtree.get("board").cloned(),
        device_tree_chip_id: dtree.get("chip_id").cloned(),
        reached_shell: a.reached_shell,
    }
}

/// The backend as the evidence engine sees it. The darwin-vm machine is generic and its default CPU is
/// QEMU's `max`, so it never models the target CPU.
pub fn identity(qemu_version: Option<String>) -> BackendIdentity {
    BackendIdentity {
        name: "darwin-vm (qemu-sptm)".to_string(),
        version: Some(format!(
            "darwin-vm {DARWIN_VM_COMMIT}, qemu-sptm {}",
            &QEMU_SPTM_COMMIT[..7]
        )),
        machine: Some(DARWIN_MACHINE.to_string()),
        cpu_model: Some("max".to_string()),
        models_target_cpu: false,
    }
    .with_qemu(qemu_version)
}

trait WithQemu {
    fn with_qemu(self, v: Option<String>) -> Self;
}

impl WithQemu for BackendIdentity {
    fn with_qemu(mut self, v: Option<String>) -> Self {
        if let Some(v) = v {
            self.version = self.version.map(|s| format!("{s}, qemu {v}"));
        }
        self
    }
}

/// Assemble the evidence from a preflight and an optional boot observation, then let the core engine decide.
pub fn evidence_from(
    preflight: &PreflightReport,
    reference_kernel_sha256: Option<String>,
    boot: Option<BootObservation>,
) -> LabEvidence {
    LabEvidence {
        backend: Some(identity(preflight.qemu_version.clone())),
        qemu_available: preflight.qemu_found,
        darwin_machine_available: preflight.darwin_machine_available,
        backend_rejects_target: false,
        firmware_present: preflight.firmware_complete(),
        reference_kernel_sha256,
        boot,
        observed_features: BTreeMap::new(),
    }
}

pub fn backend_status(
    target: &TargetManifest,
    preflight: &PreflightReport,
    evidence: &LabEvidence,
) -> BackendStatus {
    let status: VmStatus = homei_core::fidelity::classify_vm_status(target, evidence);
    BackendStatus {
        name: "darwin-vm (qemu-sptm)".to_string(),
        available: preflight.can_launch(),
        optional: false,
        vm_status: status,
        blockers: preflight.blockers.clone(),
        upstream_commit: Some(format!(
            "darwin-vm {DARWIN_VM_COMMIT}; qemu-sptm {}",
            &QEMU_SPTM_COMMIT[..7]
        )),
    }
}

/// Corellium is an optional backend and is not integrated. Its entry records that, without a claim.
pub fn corellium_status() -> BackendStatus {
    BackendStatus {
        name: "Corellium (optional, commercial)".to_string(),
        available: false,
        optional: true,
        vm_status: VmStatus::NotTested,
        blockers: vec![
            "no integration in this milestone".to_string(),
            "exact model support for iPhone 15 Pro Max on iOS 27.0.1 is not documented in the public pages read".to_string(),
        ],
        upstream_commit: None,
    }
}

/// The result of an explicit launch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchResult {
    pub status: RunStatus,
    pub log: String,
    pub truncated: bool,
    pub observation: BootObservation,
}

/// Run a launch plan once, with a timeout, capture the serial output and classify it.
///
/// The caller decides to launch; nothing here starts a VM on its own. It refuses unless the
/// preflight says a launch is possible, so a missing machine type or missing firmware never turns
/// into a half-started process.
pub fn launch(
    runner: &dyn ProcessRunner,
    plan: &LaunchPlan,
    preflight: &PreflightReport,
    kernel_sha256: Option<String>,
    dtree: &BTreeMap<String, String>,
) -> Result<LaunchResult, String> {
    if !preflight.can_launch() {
        return Err(format!(
            "preflight does not allow a launch: {}",
            preflight.blockers.join("; ")
        ));
    }
    let r = runner.run(
        &plan.program,
        &plan.args,
        Duration::from_secs(plan.timeout_secs),
    );
    let observation = observe_boot(&r.status, &r.stdout, kernel_sha256, dtree);
    Ok(LaunchResult {
        status: r.status,
        log: r.stdout,
        truncated: r.truncated,
        observation,
    })
}
