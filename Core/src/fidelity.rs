//! VM fidelity assessment.
//!
//! Three questions are answered separately and never merged:
//!
//! - A. Firmware: does the analysis refer to the exact OS build?
//! - B. Hardware: does the virtual machine reproduce the intended hardware target?
//! - C. Functional: which hardware and OS features are actually present in the guest?
//!
//! No positive result is derived from a similar CPU generation, a nearby build, or a guest that
//! merely produced a shell. Guest root is never physical-device root.

use crate::target::{HardwareFidelity, TargetManifest, VmStatus};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FidelityStatus {
    Exact,
    Partial,
    Approximate,
    Unsupported,
    Unverified,
    NotTested,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FirmwareIdentity {
    ExactReferenceMatch,
    BuildMismatch,
    ModelMismatch,
    NotEstablished,
}

/// What kind of access a guest account gives. It never implies access to a physical device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PrivilegeScope {
    GuestRoot,
    PhysicalDeviceRoot,
}

impl PrivilegeScope {
    /// Guest root never implies physical device root.
    pub fn implies(self, other: PrivilegeScope) -> bool {
        self == other
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BootOutcome {
    NotAttempted,
    Booted,
    Timeout,
    KernelPanic,
    Interrupted,
    ProcessFailed,
    IncompleteOutput,
}

/// A virtual machine backend as it was actually configured and run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackendIdentity {
    pub name: String,
    pub version: Option<String>,
    pub machine: Option<String>,
    pub cpu_model: Option<String>,
    /// True only if the backend implements a CPU model for the target itself, and not a generic one.
    pub models_target_cpu: bool,
}

/// What a boot attempt actually showed. Every field is observed, none is assumed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BootObservation {
    pub outcome: BootOutcome,
    pub kernel_sha256: Option<String>,
    pub guest_kernel_version: Option<String>,
    pub device_tree_product: Option<String>,
    pub device_tree_board: Option<String>,
    pub device_tree_chip_id: Option<String>,
    pub reached_shell: bool,
}

/// Everything known about the lab at assessment time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LabEvidence {
    pub backend: Option<BackendIdentity>,
    pub qemu_available: bool,
    pub darwin_machine_available: bool,
    pub backend_rejects_target: bool,
    pub firmware_present: bool,
    /// SHA-256 of the kernelcache in the reference manifest for the target build.
    pub reference_kernel_sha256: Option<String>,
    pub boot: Option<BootObservation>,
    /// Features observed to work in a guest, by name. Absent means not tested.
    pub observed_features: BTreeMap<String, bool>,
}

fn eq_ci(a: &Option<String>, b: &str) -> bool {
    a.as_deref()
        .map(|x| x.eq_ignore_ascii_case(b))
        .unwrap_or(false)
}

/// Does the boot observation match the target's identity in every checked respect?
pub fn boot_matches_target(
    target: &TargetManifest,
    reference_kernel_sha256: &Option<String>,
    boot: &BootObservation,
) -> bool {
    let kernel_ok = match (&boot.kernel_sha256, reference_kernel_sha256) {
        (Some(a), Some(b)) => a.eq_ignore_ascii_case(b),
        _ => false,
    };
    kernel_ok
        && boot.device_tree_product.as_deref() == Some(target.product_type.as_str())
        && eq_ci(&boot.device_tree_board, &target.board)
        && eq_ci(&boot.device_tree_chip_id, &target.cpid)
}

/// Classify the VM status from evidence. The order matters: a status is only as strong as the
/// weakest unmet condition above it.
pub fn classify_vm_status(target: &TargetManifest, ev: &LabEvidence) -> VmStatus {
    let Some(backend) = &ev.backend else {
        return VmStatus::ReferenceDeviceOnly;
    };
    if ev.backend_rejects_target {
        return VmStatus::UnsupportedTarget;
    }
    if !ev.qemu_available || !ev.darwin_machine_available {
        return VmStatus::BlockedByMissingQemuFeature;
    }
    if !ev.firmware_present {
        return VmStatus::BlockedByMissingFirmware;
    }
    let Some(boot) = &ev.boot else {
        return VmStatus::NotTested;
    };
    match boot.outcome {
        BootOutcome::NotAttempted => VmStatus::NotTested,
        BootOutcome::Timeout
        | BootOutcome::KernelPanic
        | BootOutcome::Interrupted
        | BootOutcome::ProcessFailed
        | BootOutcome::IncompleteOutput => VmStatus::BootFailure,
        BootOutcome::Booted => {
            // A guest that booted but is not the target's kernel and device tree is a Darwin guest, no more.
            if boot.guest_kernel_version.is_none()
                || !boot_matches_target(target, &ev.reference_kernel_sha256, boot)
            {
                return VmStatus::PartialDarwinGuest;
            }
            // Identity matches. "Exact" additionally needs a virtual hardware model of the target
            // itself. A generic machine with a generic CPU cannot be verified as the A17 Pro.
            if backend.models_target_cpu && boot.reached_shell {
                VmStatus::ExactTargetVerified
            } else {
                VmStatus::TargetKernelBootedHardwareUnverified
            }
        }
    }
}

/// The three-axis report, with each feature stated on its own.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FidelityReport {
    pub target_id: String,
    pub firmware_identity: FirmwareIdentity,
    /// A. Firmware compatibility.
    pub firmware: FidelityStatus,
    /// B. Hardware compatibility.
    pub hardware: FidelityStatus,
    /// C. Functional compatibility, one entry per feature.
    pub functional: BTreeMap<String, FidelityStatus>,
    pub vm_status: VmStatus,
    pub hardware_fidelity: HardwareFidelity,
    pub notes: Vec<String>,
}

/// Features the report always lists, so a missing one is visible as NOT_TESTED or UNSUPPORTED.
pub const FEATURES: &[&str] = &[
    "guest_boot",
    "serial_console",
    "springboard",
    "graphics",
    "wifi",
    "bluetooth",
    "kernel_research",
];

/// Features that upstream tooling states are not present at all.
const UNSUPPORTED_BY_DESIGN: &[&str] = &["springboard", "graphics", "wifi", "bluetooth"];

pub fn assess(target: &TargetManifest, ev: &LabEvidence) -> FidelityReport {
    let vm_status = classify_vm_status(target, ev);
    let mut notes = Vec::new();

    // A. Firmware.
    let (firmware_identity, firmware) = match &ev.boot {
        Some(b) if b.outcome == BootOutcome::Booted => {
            if boot_matches_target(target, &ev.reference_kernel_sha256, b) {
                (FirmwareIdentity::ExactReferenceMatch, FidelityStatus::Exact)
            } else if b
                .device_tree_product
                .as_deref()
                .is_some_and(|p| p != target.product_type)
            {
                (FirmwareIdentity::ModelMismatch, FidelityStatus::Unsupported)
            } else {
                (FirmwareIdentity::BuildMismatch, FidelityStatus::Unsupported)
            }
        }
        // No guest was observed. The reference manifest still names the exact build, but nothing was run.
        _ if ev.reference_kernel_sha256.is_some() => (
            FirmwareIdentity::ExactReferenceMatch,
            FidelityStatus::Unverified,
        ),
        _ => (FirmwareIdentity::NotEstablished, FidelityStatus::Unverified),
    };

    // B. Hardware.
    let hardware = match (&ev.backend, vm_status) {
        (None, _) => FidelityStatus::Unverified,
        (Some(_), VmStatus::UnsupportedTarget) => FidelityStatus::Unsupported,
        (Some(b), VmStatus::ExactTargetVerified) if b.models_target_cpu => FidelityStatus::Exact,
        (Some(b), s)
            if matches!(
                s,
                VmStatus::TargetKernelBootedHardwareUnverified | VmStatus::PartialDarwinGuest
            ) && !b.models_target_cpu =>
        {
            notes.push("The backend uses a generic machine and a generic CPU, so the guest is an approximation of the hardware.".into());
            FidelityStatus::Approximate
        }
        (Some(_), _) => FidelityStatus::Unverified,
    };

    // C. Functional.
    let booted = ev
        .boot
        .as_ref()
        .is_some_and(|b| b.outcome == BootOutcome::Booted);
    let mut functional = BTreeMap::new();
    for feature in FEATURES {
        let status = if UNSUPPORTED_BY_DESIGN.contains(feature) {
            FidelityStatus::Unsupported
        } else {
            match feature {
                &"guest_boot" => {
                    if booted {
                        FidelityStatus::Partial
                    } else {
                        FidelityStatus::NotTested
                    }
                }
                &"serial_console" | &"kernel_research" => {
                    match ev.observed_features.get(*feature) {
                        Some(true) if booted => FidelityStatus::Partial,
                        Some(false) => FidelityStatus::Unsupported,
                        _ => FidelityStatus::NotTested,
                    }
                }
                _ => FidelityStatus::NotTested,
            }
        };
        functional.insert((*feature).to_string(), status);
    }

    if !vm_status.claims_exact_target() {
        notes.push("This is not an exact target virtual machine.".into());
    }
    notes.push("Guest root is not physical device root.".into());

    let hardware_fidelity = match hardware {
        FidelityStatus::Exact => HardwareFidelity::Exact,
        FidelityStatus::Partial => HardwareFidelity::Partial,
        FidelityStatus::Approximate => HardwareFidelity::Approximate,
        FidelityStatus::Unsupported => HardwareFidelity::Unsupported,
        _ => HardwareFidelity::Unverified,
    };

    FidelityReport {
        target_id: target.target_id.clone(),
        firmware_identity,
        firmware,
        hardware,
        functional,
        vm_status,
        hardware_fidelity,
        notes,
    }
}
