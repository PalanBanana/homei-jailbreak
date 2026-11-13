//! Bundled research data, embedded at build time so the application is offline-first.

use crate::compatibility::{CompatibilityDb, CompatibilityFile};
use crate::device_profile::DeviceProfile;
use crate::evidence::{EvidenceFile, EvidenceStore};
use crate::research::ResearchIndex;
use serde::{Deserialize, Serialize};
use std::fmt;

const EVIDENCE_JSON: &str = include_str!("../../Data/evidence.json");
const COMPATIBILITY_JSON: &str = include_str!("../../Data/compatibility.json");
const RESEARCH_JSON: &str = include_str!("../../Data/research-index.json");
const FIXTURES_JSON: &str = include_str!("../../Data/device-fixtures.json");

/// A named test record. Fixtures are never connected devices.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fixture {
    pub id: String,
    pub label: String,
    pub profile: DeviceProfile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixturesFile {
    pub schema_version: u32,
    pub note: String,
    pub fixtures: Vec<Fixture>,
}

#[derive(Debug)]
pub struct DataError {
    pub file: &'static str,
    pub message: String,
}

impl fmt::Display for DataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.file, self.message)
    }
}

impl std::error::Error for DataError {}

fn parse<T: for<'de> Deserialize<'de>>(file: &'static str, json: &str) -> Result<T, DataError> {
    serde_json::from_str(json).map_err(|e| DataError {
        file,
        message: e.to_string(),
    })
}

/// Everything the application knows, loaded from the bundled files.
#[derive(Debug, Clone)]
pub struct Bundle {
    pub evidence: EvidenceStore,
    pub compatibility: CompatibilityDb,
    pub research: ResearchIndex,
    pub fixtures: Vec<Fixture>,
    pub research_snapshot_date: String,
    pub tracker_commit: String,
}

impl Bundle {
    pub fn fixture(&self, id: &str) -> Option<&Fixture> {
        self.fixtures.iter().find(|f| f.id.eq_ignore_ascii_case(id))
    }
}

/// Load the bundled data.
pub fn bundled() -> Result<Bundle, DataError> {
    let evidence: EvidenceFile = parse("Data/evidence.json", EVIDENCE_JSON)?;
    let compat: CompatibilityFile = parse("Data/compatibility.json", COMPATIBILITY_JSON)?;
    let research: ResearchIndex = parse("Data/research-index.json", RESEARCH_JSON)?;
    let fixtures: FixturesFile = parse("Data/device-fixtures.json", FIXTURES_JSON)?;
    Ok(Bundle {
        evidence: EvidenceStore::new(evidence.evidence),
        compatibility: CompatibilityDb {
            declarations: compat.declarations,
        },
        research_snapshot_date: research.snapshot_date.clone(),
        tracker_commit: research.tracker_commit.clone(),
        research,
        fixtures: fixtures.fixtures,
    })
}
