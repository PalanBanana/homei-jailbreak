//! Evidence-driven compatibility engine.
//!
//! Matching is deterministic and exact: product type, version and build must all match a
//! record. A version-only or SoC-only match never produces a positive state. A record that
//! declares `supported` or `partially_supported` is downgraded to `not_established` unless the
//! evidence it cites contains exact-scope results of the required class.
//!
//! The engine only reads data. It never contacts a device and never runs anything privileged.

use crate::device_profile::{DeviceProfile, Provenance, SocFamily};
use crate::evidence::{EvidenceClass, EvidenceStore};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompatibilityStatus {
    Supported,
    Unsupported,
    PartiallySupported,
    NotEstablished,
    Unknown,
}

impl CompatibilityStatus {
    /// Upper-case label for the interface.
    pub fn label(self) -> &'static str {
        match self {
            CompatibilityStatus::Supported => "SUPPORTED",
            CompatibilityStatus::Unsupported => "UNSUPPORTED",
            CompatibilityStatus::PartiallySupported => "PARTIALLY SUPPORTED",
            CompatibilityStatus::NotEstablished => "NOT ESTABLISHED",
            CompatibilityStatus::Unknown => "UNKNOWN",
        }
    }

    /// Whether any positive claim is being made.
    pub fn is_positive(self) -> bool {
        matches!(
            self,
            CompatibilityStatus::Supported | CompatibilityStatus::PartiallySupported
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Declaration {
    pub id: String,
    pub product_type: String,
    pub soc_family: String,
    pub product_version: String,
    pub build_version: String,
    pub architecture: Option<String>,
    pub status: CompatibilityStatus,
    pub evidence_ids: Vec<String>,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityFile {
    pub schema_version: u32,
    pub note: String,
    pub declarations: Vec<Declaration>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CompatibilityDb {
    pub declarations: Vec<Declaration>,
}

/// The engine's answer, with the reasoning needed to explain it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Decision {
    pub status: CompatibilityStatus,
    pub soc_family: Option<SocFamily>,
    pub matched_declaration: Option<String>,
    /// True when a declared positive state was reduced because exact evidence was missing.
    pub downgraded: bool,
    pub profile_provenance: Provenance,
    pub explanation: Vec<String>,
}

impl Decision {
    fn new(profile: &DeviceProfile) -> Self {
        Decision {
            status: CompatibilityStatus::Unknown,
            soc_family: profile.soc_family(),
            matched_declaration: None,
            downgraded: false,
            profile_provenance: profile.provenance,
            explanation: Vec::new(),
        }
    }

    fn finish(mut self, status: CompatibilityStatus, why: impl Into<String>) -> Self {
        self.status = status;
        self.explanation.push(why.into());
        self
    }
}

fn architectures_agree(decl: &Declaration, profile: &DeviceProfile) -> bool {
    match (&decl.architecture, &profile.cpu_architecture) {
        (Some(a), Some(b)) => a == b,
        _ => true,
    }
}

/// Evaluate a profile against the compatibility records and the evidence they cite.
pub fn evaluate(
    profile: &DeviceProfile,
    db: &CompatibilityDb,
    evidence: &EvidenceStore,
) -> Decision {
    let d = Decision::new(profile);

    if let Err(e) = profile.validate() {
        return d.finish(
            CompatibilityStatus::Unknown,
            format!("Profile rejected, so nothing is evaluated: {e}."),
        );
    }
    if d.soc_family.is_none() {
        return d.finish(
            CompatibilityStatus::Unknown,
            "No sourced SoC mapping exists for this product type.",
        );
    }

    let same_triple = |x: &&Declaration| {
        x.product_type == profile.product_type
            && x.product_version == profile.product_version
            && x.build_version == profile.build_version
    };
    let exact = db
        .declarations
        .iter()
        .filter(same_triple)
        .find(|x| architectures_agree(x, profile));

    let Some(decl) = exact else {
        if db.declarations.iter().any(|x| same_triple(&x)) {
            return d.finish(
                CompatibilityStatus::NotEstablished,
                "A record exists for this device and build, but its architecture differs from the profile.",
            );
        }
        let known_device = db
            .declarations
            .iter()
            .any(|x| x.product_type == profile.product_type);
        return if known_device {
            d.finish(
                CompatibilityStatus::NotEstablished,
                format!(
                    "Records exist for {} but none for version {} build {}. Results for other versions or other devices do not transfer.",
                    profile.product_type, profile.product_version, profile.build_version
                ),
            )
        } else {
            d.finish(
                CompatibilityStatus::Unknown,
                "No record exists for this product type.",
            )
        };
    };

    let mut d = d;
    d.matched_declaration = Some(decl.id.clone());
    let target_result = evidence.has_exact_evidence(
        &decl.evidence_ids,
        EvidenceClass::TargetCompatibleResult,
        profile,
    );
    let reproduced = evidence.has_exact_evidence(
        &decl.evidence_ids,
        EvidenceClass::IndependentReproduction,
        profile,
    );

    let (status, why) = match decl.status {
        CompatibilityStatus::Supported if target_result && reproduced => (
            CompatibilityStatus::Supported,
            "Exact-scope target result and independent reproduction are both on record."
                .to_string(),
        ),
        CompatibilityStatus::PartiallySupported if target_result || reproduced => (
            CompatibilityStatus::PartiallySupported,
            "Exact-scope evidence is on record for part of the capability set.".to_string(),
        ),
        CompatibilityStatus::Supported | CompatibilityStatus::PartiallySupported => {
            d.downgraded = true;
            (
                CompatibilityStatus::NotEstablished,
                format!(
                    "Record {} declares {} but cites no exact-scope evidence of the required class, so the state is reduced.",
                    decl.id,
                    decl.status.label()
                ),
            )
        }
        other => (other, format!("Record {}: {}", decl.id, decl.note)),
    };
    d.finish(status, why)
}

/// Whether this build offers a jailbreak action. It does not: no verified backend exists.
pub fn jailbreak_action_available() -> bool {
    false
}

/// Label used in place of a jailbreak button.
pub const RESEARCH_ONLY_LABEL: &str = "Research only. No verified jailbreak backend exists.";
