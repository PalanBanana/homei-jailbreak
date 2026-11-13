//! Offline research index: sanitized public findings that point at the research tracker.
//!
//! A finding never carries an "exploitable" label. The status vocabulary only describes how
//! well a statement is supported. Absence of a CVE entry is recorded as a limit, not as proof
//! of safety.

use crate::device_profile::DeviceProfile;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingStatus {
    Verified,
    PartiallyVerified,
    Unverified,
    NotEstablished,
    Contradicted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Finding {
    pub id: String,
    pub title: String,
    pub status: FindingStatus,
    pub evidence_ids: Vec<String>,
    pub cve_ids: Vec<String>,
    pub hardware_scope: String,
    pub firmware_scope: String,
    pub tracker_claim_id: String,
    pub last_checked: String,
    pub confidence: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResearchIndex {
    pub schema_version: u32,
    pub tracker_repository: String,
    pub tracker_commit: String,
    pub snapshot_date: String,
    pub note: String,
    pub findings: Vec<Finding>,
}

/// True for strings shaped like `CVE-2026-12345`.
pub fn is_cve_id(s: &str) -> bool {
    let mut parts = s.split('-');
    let (Some("CVE"), Some(year), Some(num), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return false;
    };
    year.len() == 4
        && year.chars().all(|c| c.is_ascii_digit())
        && num.len() >= 4
        && num.chars().all(|c| c.is_ascii_digit())
}

impl ResearchIndex {
    /// Findings whose firmware scope mentions the profile's version or build.
    pub fn findings_for<'a>(
        &'a self,
        profile: &'a DeviceProfile,
    ) -> impl Iterator<Item = &'a Finding> {
        self.findings.iter().filter(move |f| {
            f.firmware_scope.contains(&profile.product_version)
                || f.firmware_scope.contains(&profile.build_version)
        })
    }
}
