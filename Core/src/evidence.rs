//! Evidence database model.
//!
//! The classes are kept separate on purpose. A statement in a source is not a verified result,
//! a result on another device is not a result on this one, and a static observation is not a
//! running feature. Only `TargetCompatibleResult` and `IndependentReproduction` records whose
//! scope matches the exact device and build can support a positive compatibility state.

use crate::device_profile::DeviceProfile;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceClass {
    ConfirmedDeviceMetadata,
    PublicSourceStatement,
    OfficialAppleDocumentation,
    StaticBinaryObservation,
    ReportedRuntimeResult,
    IndependentReproduction,
    TargetCompatibleResult,
    Unverified,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    High,
    Medium,
    Low,
}

/// What an evidence record covers. Missing fields mean "not stated", never "any".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceScope {
    pub product_type: Option<String>,
    pub product_version: Option<String>,
    pub build_version: Option<String>,
    pub soc_family: Option<String>,
}

impl EvidenceScope {
    /// Exact-device match: product type, version and build must all be stated and equal.
    pub fn matches_exact(&self, profile: &DeviceProfile) -> bool {
        self.product_type.as_deref() == Some(profile.product_type.as_str())
            && self.product_version.as_deref() == Some(profile.product_version.as_str())
            && self.build_version.as_deref() == Some(profile.build_version.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    pub id: String,
    pub tracker_claim_id: Option<String>,
    pub title: String,
    pub class: EvidenceClass,
    pub scope: EvidenceScope,
    pub source_urls: Vec<String>,
    pub publication_date: Option<String>,
    pub access_date: Option<String>,
    pub confidence: Confidence,
    pub limitations: String,
    pub contradicting_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceFile {
    pub schema_version: u32,
    pub tracker_repository: String,
    pub tracker_commit: String,
    pub snapshot_date: String,
    pub note: String,
    pub evidence: Vec<Evidence>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EvidenceStore {
    pub items: Vec<Evidence>,
}

impl EvidenceStore {
    pub fn new(items: Vec<Evidence>) -> Self {
        Self { items }
    }

    pub fn get(&self, id: &str) -> Option<&Evidence> {
        self.items.iter().find(|e| e.id == id)
    }

    /// Does any of `ids` name a usable record of `class` that covers exactly this device and build?
    ///
    /// A record counts only when its confidence is not low and nothing contradicts it.
    pub fn has_exact_evidence(
        &self,
        ids: &[String],
        class: EvidenceClass,
        profile: &DeviceProfile,
    ) -> bool {
        ids.iter().filter_map(|id| self.get(id)).any(|e| {
            e.class == class
                && e.scope.matches_exact(profile)
                && e.confidence != Confidence::Low
                && e.contradicting_ids.is_empty()
        })
    }
}
