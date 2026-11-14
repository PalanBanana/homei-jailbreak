//! Experiment records and cross-experiment comparison, and the link between a static analysis
//! record and an experiment.
//!
//! A record keeps its own target identity. Two records are only comparable when they describe the
//! same hardware target: a result from an A18 guest stays an A18 result.

use crate::fidelity::BootOutcome;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExperimentStatus {
    Planned,
    Running,
    Completed,
    Failed,
    Interrupted,
    Inconclusive,
    NotReproducible,
}

impl ExperimentStatus {
    pub fn is_terminal(self) -> bool {
        !matches!(self, ExperimentStatus::Planned | ExperimentStatus::Running)
    }

    /// An interrupted, failed or inconclusive experiment is never a success.
    pub fn is_success(self) -> bool {
        self == ExperimentStatus::Completed
    }
}

/// Backend kinds. A mock is a test double and never counts as a real boot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackendKind {
    /// No virtual machine: a host-side workspace operation, a preflight check, or static analysis.
    None,
    Mock,
    Real,
}

/// The hardware target a record is about.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetRef {
    pub target_id: String,
    pub product_type: String,
    pub soc: String,
    pub board: String,
    pub build_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackendRef {
    pub name: String,
    pub kind: BackendKind,
    pub version: Option<String>,
    pub qemu_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactHash {
    pub name: String,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExperimentRecord {
    pub experiment_id: String,
    pub kind: String,
    pub target: TargetRef,
    pub backend: BackendRef,
    /// The guest OS build, only if it was established by observation.
    pub guest_os_build: Option<String>,
    pub firmware_artifacts: Vec<ArtifactHash>,
    pub config_sha256: String,
    pub snapshot_id: Option<String>,
    pub inputs: Vec<String>,
    pub timestamp: String,
    pub status: ExperimentStatus,
    pub boot_outcome: Option<BootOutcome>,
    pub result: String,
    pub log_refs: Vec<ArtifactHash>,
    pub limitations: Vec<String>,
    pub reproducible: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompareError {
    /// The records describe different hardware targets and cannot be compared as if identical.
    IncompatibleHardware {
        field: &'static str,
        left: String,
        right: String,
    },
    SameRecord,
}

impl fmt::Display for CompareError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompareError::IncompatibleHardware { field, left, right } => {
                write!(
                    f,
                    "records differ in {field} ({left} vs {right}) and are not comparable"
                )
            }
            CompareError::SameRecord => write!(f, "a record cannot be compared with itself"),
        }
    }
}

impl std::error::Error for CompareError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Comparison {
    pub same_build: bool,
    pub same_backend: bool,
    pub same_config: bool,
    pub same_firmware: bool,
    pub same_status: bool,
    pub notes: Vec<String>,
}

/// Compare two experiments. Different product types, SoCs or boards are rejected.
pub fn compare(a: &ExperimentRecord, b: &ExperimentRecord) -> Result<Comparison, CompareError> {
    if a.experiment_id == b.experiment_id {
        return Err(CompareError::SameRecord);
    }
    for (field, l, r) in [
        (
            "product_type",
            &a.target.product_type,
            &b.target.product_type,
        ),
        ("soc", &a.target.soc, &b.target.soc),
        ("board", &a.target.board, &b.target.board),
    ] {
        if l != r {
            return Err(CompareError::IncompatibleHardware {
                field,
                left: l.clone(),
                right: r.clone(),
            });
        }
    }
    let mut notes = Vec::new();
    let same_build = a.target.build_version == b.target.build_version;
    if !same_build {
        notes.push("The records are for different builds of the same hardware target.".into());
    }
    if a.backend.kind == BackendKind::Mock || b.backend.kind == BackendKind::Mock {
        notes.push(
            "At least one record used a mock backend and is not evidence of a real boot.".into(),
        );
    }
    Ok(Comparison {
        same_build,
        same_backend: a.backend == b.backend,
        same_config: a.config_sha256 == b.config_sha256,
        same_firmware: a.firmware_artifacts == b.firmware_artifacts,
        same_status: a.status == b.status,
        notes,
    })
}

/// What a statement in the evidence chain is. These are never merged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    StaticObservation,
    DecompiledInterpretation,
    EmulatorObservation,
    ReproducedBehavior,
    Hypothesis,
    Unknown,
}

/// A link between a static analysis record and an experiment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Correlation {
    pub analysis_project: String,
    pub firmware_sha256: String,
    pub build_version: String,
    pub experiment_id: String,
    pub guest_identity: String,
    pub config_sha256: String,
    pub evidence_kind: EvidenceKind,
    pub evidence_source: String,
    pub confidence: String,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CorrelationError {
    BuildMismatch {
        analysis: String,
        experiment: String,
    },
    ExperimentNotCompleted(ExperimentStatus),
    MockBackend,
    /// A guest observation cannot be presented as reproduced behaviour of physical hardware.
    OverclaimedEvidence,
}

impl fmt::Display for CorrelationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CorrelationError::BuildMismatch {
                analysis,
                experiment,
            } => {
                write!(
                    f,
                    "analysis is for build {analysis}, experiment for {experiment}"
                )
            }
            CorrelationError::ExperimentNotCompleted(s) => {
                write!(f, "experiment status is {s:?}, not COMPLETED")
            }
            CorrelationError::MockBackend => {
                write!(f, "a mock backend experiment cannot support a correlation")
            }
            CorrelationError::OverclaimedEvidence => {
                write!(f, "a guest observation cannot be classified as reproduced behaviour of physical hardware")
            }
        }
    }
}

impl std::error::Error for CorrelationError {}

/// Build a correlation. It preserves the firmware hash, the build and the guest identity, and
/// refuses links that would overstate what was observed.
pub fn correlate(
    analysis_project: &str,
    firmware_sha256: &str,
    analysis_build: &str,
    experiment: &ExperimentRecord,
    kind: EvidenceKind,
    confidence: &str,
) -> Result<Correlation, CorrelationError> {
    if analysis_build != experiment.target.build_version {
        return Err(CorrelationError::BuildMismatch {
            analysis: analysis_build.to_string(),
            experiment: experiment.target.build_version.clone(),
        });
    }
    if experiment.status != ExperimentStatus::Completed {
        return Err(CorrelationError::ExperimentNotCompleted(experiment.status));
    }
    if experiment.backend.kind == BackendKind::Mock {
        return Err(CorrelationError::MockBackend);
    }
    if experiment.backend.kind == BackendKind::Real && kind == EvidenceKind::ReproducedBehavior {
        // A guest result is an emulator observation until it is reproduced elsewhere.
        return Err(CorrelationError::OverclaimedEvidence);
    }
    Ok(Correlation {
        analysis_project: analysis_project.to_string(),
        firmware_sha256: firmware_sha256.to_string(),
        build_version: analysis_build.to_string(),
        experiment_id: experiment.experiment_id.clone(),
        guest_identity: experiment
            .guest_os_build
            .clone()
            .unwrap_or_else(|| "not established".to_string()),
        config_sha256: experiment.config_sha256.clone(),
        evidence_kind: kind,
        evidence_source: format!("experiment {}", experiment.experiment_id),
        confidence: confidence.to_string(),
        limitations: vec![
            "Observed in a virtual guest. It is not a statement about physical hardware behaviour."
                .to_string(),
        ],
    })
}
