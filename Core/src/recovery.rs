//! Recovery and data-protection information.
//!
//! This module explains limits. It does not automate restore, recovery or any device operation,
//! and it does not promise that a backup or a saved signing ticket allows restoring a build.

use crate::compatibility::{CompatibilityStatus, Decision};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum TopicStatus {
    /// Stated plainly by public documentation.
    Documented,
    /// Depends on facts HOMEI cannot establish.
    Uncertain,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Topic {
    pub id: &'static str,
    pub title: &'static str,
    pub status: TopicStatus,
    pub body: &'static str,
}

pub fn topics() -> Vec<Topic> {
    vec![
        Topic {
            id: "firmware_signing",
            title: "Firmware signing",
            status: TopicStatus::Uncertain,
            body: "Apple decides which builds can be installed, and that can change without notice. Any signing status shown here is a dated third-party observation, not an Apple statement.",
        },
        Topic {
            id: "restore_limits",
            title: "Restore limitations",
            status: TopicStatus::Documented,
            body: "A restore installs a build that Apple is currently signing. If the installed build is no longer signed, it cannot be reinstalled after an erase.",
        },
        Topic {
            id: "backup_limits",
            title: "Backup limitations",
            status: TopicStatus::Documented,
            body: "A backup holds data and settings, not the operating system. A backup made on a newer iOS version generally cannot be restored to an older one. A backup does not guarantee a way back to an unsigned build.",
        },
        Topic {
            id: "data_loss",
            title: "Data loss",
            status: TopicStatus::Documented,
            body: "Erasing or restoring a device removes its data. Anything not backed up separately can be lost.",
        },
        Topic {
            id: "app_compatibility",
            title: "App compatibility",
            status: TopicStatus::Uncertain,
            body: "Apps, banking and payment features, and enterprise management may react to a modified system. HOMEI has no data about the effect on any specific app.",
        },
        Topic {
            id: "firmware_preservation",
            title: "Firmware preservation",
            status: TopicStatus::Uncertain,
            body: "Saved signing tickets (SHSH blobs) do not by themselves guarantee a restore. Whether a saved ticket can be used depends on the device, the build and tooling HOMEI does not evaluate.",
        },
        Topic {
            id: "recovery_uncertainty",
            title: "Device recovery uncertainty",
            status: TopicStatus::Uncertain,
            body: "No recovery path has been verified for the research target. HOMEI automates none.",
        },
    ]
}

/// A dated, sourced observation about signing. Not a verdict.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct SigningObservation {
    pub build: &'static str,
    pub observation: &'static str,
    pub source: &'static str,
    pub date: &'static str,
    pub verified: bool,
}

/// Signing observations for the research target, from the research tracker (claim C-021).
pub fn signing_observations() -> Vec<SigningObservation> {
    vec![
        SigningObservation {
            build: "24A446",
            observation: "Listed as not signed by a third-party firmware index.",
            source: "ipsw.me device index (tracker source S-014)",
            date: "2026-11-12",
            verified: false,
        },
        SigningObservation {
            build: "24A446",
            observation: "Reported as still signed by a community repository. Not verified.",
            source: "Community reference repository (tracker source S-026)",
            date: "2026-11-02",
            verified: false,
        },
    ]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum GoNoGo {
    NoGo,
}

#[derive(Debug, Clone, Serialize)]
pub struct Guidance {
    pub verdict: GoNoGo,
    pub reasons: Vec<String>,
}

/// GO / NO-GO guidance. In this release the answer is always NO-GO: HOMEI contains no backend,
/// so there is nothing it could responsibly start.
pub fn guidance(decision: &Decision) -> Guidance {
    let mut reasons = vec![
        "This release contains no operational backend. There is nothing to run.".to_string(),
        "No recovery path has been verified for this device and build.".to_string(),
    ];
    if decision.status != CompatibilityStatus::Supported {
        reasons.push(format!(
            "Compatibility is {}. It needs exact-device, exact-build evidence.",
            decision.status.label()
        ));
    }
    Guidance {
        verdict: GoNoGo::NoGo,
        reasons,
    }
}
