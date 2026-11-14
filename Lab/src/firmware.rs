//! Firmware artifact metadata. It reads hashes and sizes from a manifest and can verify a local
//! file against them. It never modifies a firmware file. A matching checksum shows the file is the
//! one that was recorded. It does not prove authenticity.

use crate::hash::{is_sha256_hex, sha256_file};
use homei_core::labstate::FirmwareArtifact;
use serde::Deserialize;
use std::fmt;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ArtifactRecord {
    pub artifact: String,
    /// Absent for artifacts that are not stored as a file below the analysis tree, such as the package itself.
    pub path_relative: Option<String>,
    pub bytes: u64,
    pub sha256: String,
    pub extraction_method: String,
    pub source_firmware: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FirmwareError {
    Parse(String),
    BuildMismatch {
        expected: String,
        found: String,
    },
    MissingBinary(String),
    HashMismatch {
        name: String,
        expected: String,
        found: String,
    },
    BadDigest(String),
}

impl fmt::Display for FirmwareError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for FirmwareError {}

/// Parse an artifact hash manifest and keep only the records for one build.
pub fn parse_artifact_hashes(
    json: &str,
    build: &str,
) -> Result<Vec<ArtifactRecord>, FirmwareError> {
    let all: Vec<ArtifactRecord> =
        serde_json::from_str(json).map_err(|e| FirmwareError::Parse(e.to_string()))?;
    for r in &all {
        if !is_sha256_hex(&r.sha256) {
            return Err(FirmwareError::BadDigest(r.artifact.clone()));
        }
    }
    Ok(all
        .into_iter()
        .filter(|r| r.source_firmware.contains(build))
        .collect())
}

/// Fail if a record is for a different build than the target.
pub fn require_build(r: &ArtifactRecord, build: &str) -> Result<(), FirmwareError> {
    if r.source_firmware.contains(build) {
        Ok(())
    } else {
        Err(FirmwareError::BuildMismatch {
            expected: build.to_string(),
            found: r.source_firmware.clone(),
        })
    }
}

/// Verify a local file against a record. Reads the file and nothing else.
pub fn verify_local(path: &Path, r: &ArtifactRecord) -> Result<(), FirmwareError> {
    if !path.is_file() {
        return Err(FirmwareError::MissingBinary(r.artifact.clone()));
    }
    let found = sha256_file(path).map_err(|_| FirmwareError::MissingBinary(r.artifact.clone()))?;
    if found == r.sha256 {
        Ok(())
    } else {
        Err(FirmwareError::HashMismatch {
            name: r.artifact.clone(),
            expected: r.sha256.clone(),
            found,
        })
    }
}

fn role_of(name: &str) -> &'static str {
    let n = name.to_lowercase();
    if n.contains("buildmanifest") {
        "build manifest"
    } else if n.contains("ipsw") {
        "full firmware package"
    } else if n.contains("kext") {
        "kernel extension"
    } else if n.contains("kernelcache") || n.contains("decompressed payload") {
        "kernelcache"
    } else if n.contains("sptm") {
        "secure page table monitor"
    } else if n.contains("txm") {
        "trusted execution monitor"
    } else {
        "firmware component"
    }
}

/// A coarse, honest format label taken from the artifact's own description, never guessed from bytes.
fn format_of(name: &str) -> &'static str {
    let n = name.to_lowercase();
    if n.contains("buildmanifest") {
        "property list"
    } else if n.contains("ipsw") {
        "zip package"
    } else if n.contains("im4p container") {
        "IM4P container"
    } else if n.contains("decompressed payload") || n.contains("mh_fileset") {
        "Mach-O fileset payload"
    } else if n.contains("kext") {
        "Mach-O extracted from the kernelcache"
    } else {
        "not classified"
    }
}

/// Public metadata for the app: a name, size, hash and role. No path and no local file name.
pub fn to_public(r: &ArtifactRecord, build: &str) -> FirmwareArtifact {
    FirmwareArtifact {
        name: r.artifact.clone(),
        bytes: r.bytes,
        sha256: r.sha256.clone(),
        format: format_of(&r.artifact).to_string(),
        source_build: build.to_string(),
        role: role_of(&r.artifact).to_string(),
    }
}
