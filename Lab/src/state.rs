//! Assemble the sanitized lab state that the iOS app displays. Every field comes from a real input
//! or says it is missing. The result is validated before it is written.

use crate::backend::{backend_status, corellium_status, evidence_from, PreflightReport};
use homei_core::experiment::{Correlation, ExperimentRecord};
use homei_core::fidelity::{assess, BootObservation};
use homei_core::labstate::{
    EvidenceItem, FirmwareArtifact, GhidraSummary, GuestSummary, HostSummary, LabState,
};
use homei_core::target::TargetManifest;

pub struct StateInputs {
    pub generated_on: String,
    pub target: TargetManifest,
    pub host: HostSummary,
    pub preflight: PreflightReport,
    pub reference_kernel_sha256: Option<String>,
    pub boot: Option<BootObservation>,
    pub firmware: Vec<FirmwareArtifact>,
    pub ghidra: GhidraSummary,
    pub guest: GuestSummary,
    pub experiments: Vec<ExperimentRecord>,
    pub correlations: Vec<Correlation>,
}

/// An empty guest summary: no overlay, nothing installed. Used when no workspace was given.
pub fn empty_guest() -> GuestSummary {
    GuestSummary {
        filesystem_scope: "isolated guest overlay (not a device)".to_string(),
        overlay_present: false,
        base_image_sha256: None,
        snapshots: 0,
        vm_running: false,
        bootstrap_version: None,
        bootstrap_state: "not provisioned".to_string(),
        packages: Vec::new(),
        transactions: 0,
    }
}

fn evidence_items() -> Vec<EvidenceItem> {
    let e =
        |id: &str, category: &str, title: &str, reference: &str, limitation: &str| EvidenceItem {
            id: id.to_string(),
            category: category.to_string(),
            title: title.to_string(),
            reference: reference.to_string(),
            limitation: limitation.to_string(),
        };
    vec![
        e("EV-LAB-001", "Feasibility report", "Exact target feasibility", "Docs/EXACT_TARGET_FEASIBILITY.md", "Read-only investigation of public sources and this host on 2026-11-14. Nothing was booted."),
        e("EV-LAB-002", "Target manifest", "Canonical iPhone16,2 / 27.0.1 / 24A446 reference", "Research/Targets/iPhone16,2-iOS27.0.1.json", "Reference metadata. It is not evidence that a VM booted."),
        e("EV-LAB-003", "Firmware manifest", "BuildManifest.plist of the 24A446 package", "tracker source S-043", "Primary Apple artifact. A hash shows identity with the recorded file, not authenticity."),
        e("EV-LAB-004", "Public source", "darwin-vm README, tested configurations and limits", "https://github.com/jprx/darwin-vm", "The A17 Pro is not in the tested table. The tested iOS 27.0 build is 24A437."),
        e("EV-LAB-005", "Public source", "qemu-sptm darwin machine source", "https://github.com/jprx/qemu-sptm", "Pinned commit. The default CPU is the generic max model."),
        e("EV-LAB-006", "Public source", "Corellium release notes and iOS device page", "https://support.corellium.com/releases", "OS support is listed. Exact-model support for the target is not documented in the pages read."),
        e("EV-LAB-007", "Static analysis", "Existing Ghidra project for the target kernelcache", "tracker reports/ghidra-*.md", "Static observation only. Ghidra function names are not vulnerability claims."),
    ]
}

fn unresolved(preflight: &PreflightReport, has_boot: bool) -> Vec<String> {
    let mut v = vec![
        "No exact A17 Pro / 24A446 guest has been booted.".to_string(),
        "Hardware fidelity is unverified: darwin-vm uses a generic machine and QEMU's generic max CPU.".to_string(),
        "The CPU name t8130 is a secondary-source label and the monitor image is named t8122 (tracker claim C-159).".to_string(),
        "The unmodified darwin-vm preparation script does not find an SPTM image for this device (sptm.t8130 is absent, sptm.t8122 is present).".to_string(),
        "Preparing a bootable ramdisk needs macOS. This host is Linux.".to_string(),
        "Exact-model support for iPhone 15 Pro Max on iOS 27.0.1 is not documented for Corellium.".to_string(),
    ];
    for b in &preflight.blockers {
        v.push(format!("Preflight: {b}"));
    }
    if !has_boot {
        v.push("No guest boot has been attempted, so every functional feature is NOT_TESTED or UNSUPPORTED.".to_string());
    }
    v
}

pub fn build_state(i: StateInputs) -> LabState {
    let evidence = evidence_from(
        &i.preflight,
        i.reference_kernel_sha256.clone(),
        i.boot.clone(),
    );
    let fidelity = assess(&i.target, &evidence);
    let backends = vec![
        backend_status(&i.target, &i.preflight, &evidence),
        corellium_status(),
    ];
    let unresolved = unresolved(&i.preflight, i.boot.is_some());
    LabState {
        schema_version: 1,
        generated_on: i.generated_on,
        target: i.target,
        host: i.host,
        backends,
        firmware: i.firmware,
        ghidra: i.ghidra,
        guest: i.guest,
        fidelity,
        experiments: i.experiments,
        correlations: i.correlations,
        evidence: evidence_items(),
        unresolved,
    }
}
