//! The lab state exported by the host controller and read by the iOS application.
//!
//! The iOS app never runs a virtual machine, Ghidra or a package manager. The host controller does
//! those things and writes this sanitized snapshot. The type has no absolute-path field, so a
//! local path cannot be serialised into it by accident, and unknown JSON keys are rejected.

use crate::experiment::{Correlation, ExperimentRecord};
use crate::fidelity::FidelityReport;
use crate::target::{TargetManifest, VmStatus};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostSummary {
    pub distribution: String,
    pub architecture: String,
    pub qemu_version: Option<String>,
    pub qemu_accelerators: Vec<String>,
    /// `/dev/kvm` exists. It says nothing about whether an arm64 guest could use it.
    pub kvm_device_present: bool,
    pub ghidra_version: Option<String>,
    pub memory_total_gib: u32,
    pub memory_available_gib: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackendStatus {
    pub name: String,
    pub available: bool,
    pub optional: bool,
    pub vm_status: VmStatus,
    pub blockers: Vec<String>,
    pub upstream_commit: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FirmwareArtifact {
    pub name: String,
    pub bytes: u64,
    pub sha256: String,
    pub format: String,
    pub source_build: String,
    pub role: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GhidraSummary {
    pub ghidra_version: Option<String>,
    pub project_name: String,
    pub project_present: bool,
    pub program: String,
    pub firmware_sha256: Option<String>,
    pub architecture: String,
    pub import_status: String,
    pub function_count: u64,
    pub fileset_entries: u64,
    pub fileset_entries_covered: u64,
    /// Symbol match rate in basis points (hundredths of a percent), to avoid floating-point comparison.
    pub symbol_match_basis_points: Option<u32>,
    pub import_errors: Vec<String>,
    pub last_analysis_date: Option<String>,
    pub source_reference: String,
    pub confidence: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GuestSummary {
    /// Always a guest filesystem, never a device.
    pub filesystem_scope: String,
    pub overlay_present: bool,
    pub base_image_sha256: Option<String>,
    pub snapshots: u32,
    pub vm_running: bool,
    pub bootstrap_version: Option<String>,
    pub bootstrap_state: String,
    pub packages: Vec<InstalledPackage>,
    pub transactions: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstalledPackage {
    pub id: String,
    pub version: String,
    pub provenance: String,
    pub signature: String,
    pub files: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceItem {
    pub id: String,
    pub category: String,
    pub title: String,
    pub reference: String,
    pub limitation: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LabState {
    pub schema_version: u32,
    pub generated_on: String,
    pub target: TargetManifest,
    pub host: HostSummary,
    pub backends: Vec<BackendStatus>,
    pub firmware: Vec<FirmwareArtifact>,
    pub ghidra: GhidraSummary,
    pub guest: GuestSummary,
    pub fidelity: FidelityReport,
    pub experiments: Vec<ExperimentRecord>,
    pub correlations: Vec<Correlation>,
    pub evidence: Vec<EvidenceItem>,
    pub unresolved: Vec<String>,
}

impl LabState {
    /// Checks that must hold before a snapshot is published: a valid canonical target, a VM status
    /// that agrees with the fidelity report, and no text that looks like a local path.
    pub fn validate(&self) -> Result<(), String> {
        self.target.validate().map_err(|e| e.to_string())?;
        if self.fidelity.target_id != self.target.target_id {
            return Err("fidelity report is for a different target".into());
        }
        let json = serde_json::to_string(self).map_err(|e| e.to_string())?;
        for marker in ["/home/", "/Users/", "/tmp/", "C:\\"] {
            if json.contains(marker) {
                return Err(format!(
                    "the snapshot contains a local path marker {marker:?}"
                ));
            }
        }
        Ok(())
    }
}
