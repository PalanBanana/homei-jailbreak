//! HOMEI offline package catalog.
//!
//! The catalog holds package metadata only: identifiers, versions, descriptions, source
//! attribution, licence, compatibility declarations, integrity metadata and dependency
//! summaries. No package can be downloaded, installed or injected, and every install attempt
//! returns an explicit unsupported result.

use homei_core::compatibility::{CompatibilityStatus, Decision};
use homei_core::safety::{self, Unsupported};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

const PACKAGES_JSON: &str = include_str!("../Packages/catalog.json");
const SOURCES_JSON: &str = include_str!("../Sources/sources.json");

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageCompat {
    pub product_type: String,
    pub product_version: String,
    pub status: CompatibilityStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Integrity {
    pub algorithm: String,
    pub digest: String,
    pub verified: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Package {
    pub id: String,
    pub name: String,
    pub version: String,
    pub summary: String,
    pub source_id: String,
    pub license: String,
    pub compatibility: Vec<PackageCompat>,
    pub integrity: Integrity,
    pub dependencies: Vec<String>,
    pub fixture: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub id: String,
    pub name: String,
    pub url: Option<String>,
    pub trust: String,
    pub attribution: String,
    pub license: String,
}

#[derive(Deserialize)]
struct PackagesFile {
    packages: Vec<Package>,
}

#[derive(Deserialize)]
struct SourcesFile {
    sources: Vec<Source>,
}

/// What the catalog can do with a package: show it. Nothing more.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Availability {
    pub installable: bool,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct Catalog {
    pub packages: Vec<Package>,
    pub sources: Vec<Source>,
}

impl Catalog {
    /// Load the bundled offline catalog.
    pub fn bundled() -> Result<Catalog, String> {
        let p: PackagesFile =
            serde_json::from_str(PACKAGES_JSON).map_err(|e| format!("catalog.json: {e}"))?;
        let s: SourcesFile =
            serde_json::from_str(SOURCES_JSON).map_err(|e| format!("sources.json: {e}"))?;
        Ok(Catalog {
            packages: p.packages,
            sources: s.sources,
        })
    }

    pub fn get(&self, id: &str) -> Option<&Package> {
        self.packages.iter().find(|p| p.id == id)
    }

    /// Problems found in the catalog; empty means valid.
    pub fn validate(&self) -> Vec<String> {
        let mut problems = Vec::new();
        for p in &self.packages {
            let id_ok = !p.id.is_empty()
                && p.id
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '-');
            if !id_ok {
                problems.push(format!("{}: invalid package id", p.id));
            }
            if p.license.trim().is_empty() {
                problems.push(format!("{}: licence is required", p.id));
            }
            if !self.sources.iter().any(|s| s.id == p.source_id) {
                problems.push(format!("{}: unknown source {}", p.id, p.source_id));
            }
            let digest_ok = p.integrity.digest.len() == 64
                && p.integrity.digest.chars().all(|c| c.is_ascii_hexdigit());
            if p.integrity.algorithm != "sha256" || !digest_ok {
                problems.push(format!("{}: integrity must be a sha256 hex digest", p.id));
            }
            for dep in &p.dependencies {
                if self.get(dep).is_none() {
                    problems.push(format!("{}: unknown dependency {dep}", p.id));
                }
            }
            if !p.fixture && !p.integrity.verified {
                problems.push(format!(
                    "{}: a non-fixture package needs verified integrity",
                    p.id
                ));
            }
        }
        problems
    }

    /// Transitive dependency summary, deduplicated and sorted. Cycles do not loop forever.
    pub fn dependency_summary(&self, id: &str) -> Vec<String> {
        let mut seen = BTreeSet::new();
        let mut stack: Vec<&str> = self
            .get(id)
            .map(|p| p.dependencies.iter().map(String::as_str).collect())
            .unwrap_or_default();
        while let Some(next) = stack.pop() {
            if seen.insert(next.to_string()) {
                if let Some(p) = self.get(next) {
                    stack.extend(p.dependencies.iter().map(String::as_str));
                }
            }
        }
        seen.into_iter().collect()
    }

    /// What can be done with a package. The answer is always metadata only.
    pub fn availability(&self, package: &Package, decision: &Decision) -> Availability {
        let declared = package
            .compatibility
            .iter()
            .map(|c| c.status.label())
            .collect::<Vec<_>>()
            .join(", ");
        Availability {
            installable: false,
            reason: format!(
                "Metadata only. Device compatibility is {}; the package declares {}. This build cannot install anything.",
                decision.status.label(),
                if declared.is_empty() { "nothing".to_string() } else { declared }
            ),
        }
    }

    /// Install a package. Always unsupported.
    pub fn install(&self, _id: &str) -> Result<(), Unsupported> {
        safety::invoke_capability("Package installation")
    }
}
