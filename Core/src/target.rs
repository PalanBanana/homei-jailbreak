//! Canonical research target manifest.
//!
//! The manifest is reference metadata: what the target device and firmware are, and where each fact
//! comes from. It is not evidence that anything booted. Observations obtained inside a virtual
//! machine live in `labstate` and `experiment`, never here.
//!
//! The exact tuple is fixed. A manifest that describes a different model, SoC, board, chip or build
//! is rejected, so another device can never be substituted silently.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

/// The canonical target. Every field must match exactly.
pub const CANONICAL_PRODUCT_TYPE: &str = "iPhone16,2";
pub const CANONICAL_SOC: &str = "Apple A17 Pro";
pub const CANONICAL_CPU: &str = "t8130";
pub const CANONICAL_BOARD: &str = "D84AP";
pub const CANONICAL_CPID: &str = "0x8130";
pub const CANONICAL_BDID: &str = "0x06";
pub const CANONICAL_ARCHITECTURE: &str = "arm64e";
pub const CANONICAL_OS_VERSION: &str = "27.0.1";
pub const CANONICAL_BUILD: &str = "24A446";
pub const CANONICAL_MODEL: &str = "iPhone 15 Pro Max";

/// Fields that must each carry provenance.
pub const PROVENANCE_FIELDS: &[&str] = &[
    "commercial_model",
    "product_type",
    "soc",
    "cpu",
    "board",
    "cpid",
    "bdid",
    "architecture",
    "os_version",
    "build_version",
];

/// Whether a virtual machine for the target exists and what is known about it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VmStatus {
    ExactTargetVerified,
    TargetKernelBootedHardwareUnverified,
    PartialDarwinGuest,
    ReferenceDeviceOnly,
    UnsupportedTarget,
    BlockedByMissingFirmware,
    BlockedByMissingQemuFeature,
    BootFailure,
    NotTested,
}

impl VmStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            VmStatus::ExactTargetVerified => "EXACT_TARGET_VERIFIED",
            VmStatus::TargetKernelBootedHardwareUnverified => {
                "TARGET_KERNEL_BOOTED_HARDWARE_UNVERIFIED"
            }
            VmStatus::PartialDarwinGuest => "PARTIAL_DARWIN_GUEST",
            VmStatus::ReferenceDeviceOnly => "REFERENCE_DEVICE_ONLY",
            VmStatus::UnsupportedTarget => "UNSUPPORTED_TARGET",
            VmStatus::BlockedByMissingFirmware => "BLOCKED_BY_MISSING_FIRMWARE",
            VmStatus::BlockedByMissingQemuFeature => "BLOCKED_BY_MISSING_QEMU_FEATURE",
            VmStatus::BootFailure => "BOOT_FAILURE",
            VmStatus::NotTested => "NOT_TESTED",
        }
    }

    /// True only for the single status that claims an exact target VM.
    pub fn claims_exact_target(self) -> bool {
        self == VmStatus::ExactTargetVerified
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HardwareFidelity {
    Exact,
    Partial,
    Approximate,
    Unsupported,
    Unverified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    /// Apple's own artifact or statement.
    Primary,
    /// A tool database, index or community source.
    Secondary,
    /// A value supplied by the project as the reference target, not independently sourced.
    Reference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    Verified,
    PartiallyVerified,
    Unverified,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldProvenance {
    pub source: String,
    pub kind: SourceKind,
    pub confidence: Confidence,
    pub tracker_claim_ids: Vec<String>,
    pub note: String,
}

/// A recorded disagreement between sources. It is kept, not resolved by choosing one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Discrepancy {
    pub id: String,
    pub summary: String,
    pub tracker_claim_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetManifest {
    pub schema_version: u32,
    pub target_id: String,
    pub commercial_model: String,
    pub product_type: String,
    pub soc: String,
    pub cpu: String,
    pub board: String,
    pub cpid: String,
    pub bdid: String,
    pub architecture: String,
    pub os_version: String,
    pub build_version: String,
    /// Reference status only. It is `NOT_TESTED` until an observation says otherwise.
    pub vm_status: VmStatus,
    pub hardware_fidelity: HardwareFidelity,
    pub provenance: BTreeMap<String, FieldProvenance>,
    pub discrepancies: Vec<Discrepancy>,
    pub checked_on: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetError {
    UnsupportedSchema(u32),
    /// A field differs from the canonical target. A different model cannot be substituted.
    FieldMismatch {
        field: &'static str,
        expected: &'static str,
        found: String,
    },
    MissingProvenance(&'static str),
    EmptyProvenanceSource(&'static str),
    /// A positive VM status in reference metadata would be read as evidence of a boot.
    ReferenceClaimsObservation(VmStatus),
}

impl fmt::Display for TargetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TargetError::UnsupportedSchema(v) => write!(f, "unsupported schema version {v}"),
            TargetError::FieldMismatch {
                field,
                expected,
                found,
            } => {
                write!(
                    f,
                    "{field} is {found:?}, but the target is fixed to {expected:?}"
                )
            }
            TargetError::MissingProvenance(field) => write!(f, "field {field} has no provenance"),
            TargetError::EmptyProvenanceSource(field) => {
                write!(f, "field {field} has an empty provenance source")
            }
            TargetError::ReferenceClaimsObservation(s) => {
                write!(
                    f,
                    "reference metadata may not carry VM status {}",
                    s.as_str()
                )
            }
        }
    }
}

impl std::error::Error for TargetError {}

impl TargetManifest {
    /// Validate against the canonical tuple. Checks every field and every provenance entry.
    pub fn validate(&self) -> Result<(), TargetError> {
        if self.schema_version != 1 {
            return Err(TargetError::UnsupportedSchema(self.schema_version));
        }
        let pairs: [(&'static str, &'static str, &String); 10] = [
            ("commercial_model", CANONICAL_MODEL, &self.commercial_model),
            ("product_type", CANONICAL_PRODUCT_TYPE, &self.product_type),
            ("soc", CANONICAL_SOC, &self.soc),
            ("cpu", CANONICAL_CPU, &self.cpu),
            ("board", CANONICAL_BOARD, &self.board),
            ("cpid", CANONICAL_CPID, &self.cpid),
            ("bdid", CANONICAL_BDID, &self.bdid),
            ("architecture", CANONICAL_ARCHITECTURE, &self.architecture),
            ("os_version", CANONICAL_OS_VERSION, &self.os_version),
            ("build_version", CANONICAL_BUILD, &self.build_version),
        ];
        for (field, expected, found) in pairs {
            if found != expected {
                return Err(TargetError::FieldMismatch {
                    field,
                    expected,
                    found: found.clone(),
                });
            }
        }
        for field in PROVENANCE_FIELDS {
            match self.provenance.get(*field) {
                None => return Err(TargetError::MissingProvenance(field)),
                Some(p) if p.source.trim().is_empty() => {
                    return Err(TargetError::EmptyProvenanceSource(field))
                }
                Some(_) => {}
            }
        }
        // Reference data describes the target. It cannot say a VM booted.
        if !matches!(
            self.vm_status,
            VmStatus::NotTested | VmStatus::ReferenceDeviceOnly
        ) {
            return Err(TargetError::ReferenceClaimsObservation(self.vm_status));
        }
        Ok(())
    }

    /// True when the manifest describes exactly the reference target.
    pub fn is_canonical(&self) -> bool {
        self.validate().is_ok()
    }
}
