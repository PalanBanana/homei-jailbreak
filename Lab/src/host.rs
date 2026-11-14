//! Read-only host detection. Nothing here changes the host, installs anything or uses privileges.

use crate::process::{ProcessRunner, RunStatus};
use homei_core::labstate::HostSummary;
use std::time::Duration;

fn gib(kib: u64) -> u32 {
    (kib / 1024 / 1024) as u32
}

/// Parse `/etc/os-release` for the distribution name.
pub fn parse_distribution(os_release: &str) -> String {
    for line in os_release.lines() {
        if let Some(v) = line.strip_prefix("PRETTY_NAME=") {
            return v.trim().trim_matches('"').to_string();
        }
    }
    "unknown".to_string()
}

/// Parse `/proc/meminfo` for total and available memory in whole GiB.
pub fn parse_memory(meminfo: &str) -> (u32, u32) {
    let field = |name: &str| {
        meminfo
            .lines()
            .find_map(|l| l.strip_prefix(name))
            .and_then(|r| r.trim().trim_end_matches("kB").trim().parse::<u64>().ok())
            .unwrap_or(0)
    };
    (gib(field("MemTotal:")), gib(field("MemAvailable:")))
}

/// First line of `qemu-system-aarch64 --version`, reduced to the version number.
pub fn parse_qemu_version(output: &str) -> Option<String> {
    let first = output.lines().next()?;
    let rest = first.split("version ").nth(1)?;
    Some(rest.split_whitespace().next()?.to_string())
}

/// Accelerator names from `-accel help`.
pub fn parse_accelerators(output: &str) -> Vec<String> {
    output
        .lines()
        .skip_while(|l| !l.to_lowercase().contains("accelerators supported"))
        .skip(1)
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

/// Does the `-M help` listing name a machine called `name`?
pub fn has_machine(machine_help: &str, name: &str) -> bool {
    machine_help
        .lines()
        .any(|l| l.split_whitespace().next() == Some(name))
}

pub struct HostInputs<'a> {
    pub os_release: &'a str,
    pub meminfo: &'a str,
    pub architecture: &'a str,
    pub kvm_device_present: bool,
    pub ghidra_version: Option<String>,
}

/// Build the host summary from inputs and a runner. Pure with respect to the host, so it can be tested.
pub fn detect_with(inputs: &HostInputs<'_>, qemu: &str, runner: &dyn ProcessRunner) -> HostSummary {
    let version = runner.run(qemu, &["--version".to_string()], Duration::from_secs(10));
    let (qemu_version, accelerators) = if version.status == RunStatus::Exited(0) {
        let accel = runner.run(
            qemu,
            &["-accel".to_string(), "help".to_string()],
            Duration::from_secs(10),
        );
        (
            parse_qemu_version(&version.stdout),
            parse_accelerators(&accel.stdout),
        )
    } else {
        (None, Vec::new())
    };
    let (total, available) = parse_memory(inputs.meminfo);
    HostSummary {
        distribution: parse_distribution(inputs.os_release),
        architecture: inputs.architecture.to_string(),
        qemu_version,
        qemu_accelerators: accelerators,
        kvm_device_present: inputs.kvm_device_present,
        ghidra_version: inputs.ghidra_version.clone(),
        memory_total_gib: total,
        memory_available_gib: available,
    }
}

/// Read the Ghidra version from its `application.properties`.
pub fn parse_ghidra_version(properties: &str) -> Option<String> {
    properties
        .lines()
        .find_map(|l| l.strip_prefix("application.version="))
        .map(|v| v.trim().to_string())
}

/// Detect the real host. Reads a few files and asks QEMU for its version. It starts no VM.
pub fn detect(
    qemu: &str,
    runner: &dyn ProcessRunner,
    ghidra_dir: Option<&std::path::Path>,
) -> HostSummary {
    let os_release = std::fs::read_to_string("/etc/os-release").unwrap_or_default();
    let meminfo = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let ghidra_version = ghidra_dir
        .and_then(|d| std::fs::read_to_string(d.join("Ghidra/application.properties")).ok())
        .and_then(|p| parse_ghidra_version(&p));
    detect_with(
        &HostInputs {
            os_release: &os_release,
            meminfo: &meminfo,
            architecture: std::env::consts::ARCH,
            kvm_device_present: std::path::Path::new("/dev/kvm").exists(),
            ghidra_version,
        },
        qemu,
        runner,
    )
}
