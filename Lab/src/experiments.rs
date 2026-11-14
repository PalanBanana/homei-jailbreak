//! Reproducible experiment records, stored in the lab workspace.
//!
//! The state machine is strict: a record is planned, then running, then exactly one terminal state.
//! A run that was still marked running when the store is opened was interrupted, and is recorded as
//! interrupted. It is never marked successful.

use crate::guard::{GuardError, GuestRoot};
use crate::hash::sha256_hex;
use homei_core::experiment::{
    ArtifactHash, BackendRef, ExperimentRecord, ExperimentStatus, TargetRef,
};
use homei_core::fidelity::BootOutcome;
use std::collections::BTreeMap;
use std::fmt;

pub const MAX_LOG_BYTES: usize = 1 << 20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExperimentError {
    NotFound(String),
    BadTransition {
        id: String,
        from: ExperimentStatus,
        to: ExperimentStatus,
    },
    Guard(String),
    Parse(String),
}

impl fmt::Display for ExperimentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for ExperimentError {}

impl From<GuardError> for ExperimentError {
    fn from(e: GuardError) -> Self {
        ExperimentError::Guard(e.to_string())
    }
}

pub struct ExperimentSpec {
    pub kind: String,
    pub target: TargetRef,
    pub backend: BackendRef,
    pub firmware_artifacts: Vec<ArtifactHash>,
    pub config: BTreeMap<String, String>,
    pub inputs: Vec<String>,
    pub limitations: Vec<String>,
    pub snapshot_id: Option<String>,
}

/// Hash of a configuration, independent of insertion order.
pub fn config_hash(config: &BTreeMap<String, String>) -> String {
    sha256_hex(serde_json::to_string(config).unwrap_or_default().as_bytes())
}

pub struct ExperimentStore {
    root: GuestRoot,
    /// Experiments that were still running when the store was opened, now marked interrupted.
    pub interrupted_on_open: Vec<String>,
}

impl ExperimentStore {
    pub fn open(root: GuestRoot) -> Result<ExperimentStore, ExperimentError> {
        root.mkdir_all("experiments")?;
        let mut store = ExperimentStore {
            root,
            interrupted_on_open: Vec::new(),
        };
        for r in store.list()? {
            if r.status == ExperimentStatus::Running {
                let mut r = r;
                r.status = ExperimentStatus::Interrupted;
                r.result =
                    "interrupted: the run was still marked running when the store was reopened"
                        .to_string();
                store.save(&r)?;
                store.interrupted_on_open.push(r.experiment_id.clone());
            }
        }
        Ok(store)
    }

    fn save(&self, r: &ExperimentRecord) -> Result<(), ExperimentError> {
        let json =
            serde_json::to_vec_pretty(r).map_err(|e| ExperimentError::Parse(e.to_string()))?;
        self.root
            .write(&format!("experiments/{}.json", r.experiment_id), &json)?;
        Ok(())
    }

    pub fn list(&self) -> Result<Vec<ExperimentRecord>, ExperimentError> {
        let mut out = Vec::new();
        for rel in self.root.list_files("experiments")? {
            if rel.ends_with(".json") {
                out.push(
                    serde_json::from_slice(&self.root.read(&rel)?)
                        .map_err(|e| ExperimentError::Parse(e.to_string()))?,
                );
            }
        }
        out.sort_by(|a: &ExperimentRecord, b| a.experiment_id.cmp(&b.experiment_id));
        Ok(out)
    }

    pub fn get(&self, id: &str) -> Result<ExperimentRecord, ExperimentError> {
        let rel = format!("experiments/{id}.json");
        if !self.root.exists(&rel) {
            return Err(ExperimentError::NotFound(id.to_string()));
        }
        serde_json::from_slice(&self.root.read(&rel)?)
            .map_err(|e| ExperimentError::Parse(e.to_string()))
    }

    pub fn plan(
        &self,
        spec: ExperimentSpec,
        now: &str,
    ) -> Result<ExperimentRecord, ExperimentError> {
        let id = format!("exp-{:04}", self.list()?.len() + 1);
        let r = ExperimentRecord {
            experiment_id: id,
            kind: spec.kind,
            target: spec.target,
            backend: spec.backend,
            guest_os_build: None,
            firmware_artifacts: spec.firmware_artifacts,
            config_sha256: config_hash(&spec.config),
            snapshot_id: spec.snapshot_id,
            inputs: spec.inputs,
            timestamp: now.to_string(),
            status: ExperimentStatus::Planned,
            boot_outcome: None,
            result: "planned".to_string(),
            log_refs: Vec::new(),
            limitations: spec.limitations,
            reproducible: None,
        };
        self.save(&r)?;
        Ok(r)
    }

    fn transition(
        &self,
        id: &str,
        allowed_from: &[ExperimentStatus],
        to: ExperimentStatus,
    ) -> Result<ExperimentRecord, ExperimentError> {
        let mut r = self.get(id)?;
        if !allowed_from.contains(&r.status) {
            return Err(ExperimentError::BadTransition {
                id: id.to_string(),
                from: r.status,
                to,
            });
        }
        r.status = to;
        Ok(r)
    }

    pub fn start(&self, id: &str) -> Result<ExperimentRecord, ExperimentError> {
        let mut r = self.transition(id, &[ExperimentStatus::Planned], ExperimentStatus::Running)?;
        r.result = "running".to_string();
        self.save(&r)?;
        Ok(r)
    }

    /// Finish a running experiment with a terminal status. `Completed` is the only success.
    pub fn finish(
        &self,
        id: &str,
        status: ExperimentStatus,
        boot_outcome: Option<BootOutcome>,
        result: &str,
        guest_os_build: Option<String>,
    ) -> Result<ExperimentRecord, ExperimentError> {
        if !status.is_terminal() || status == ExperimentStatus::NotReproducible {
            return Err(ExperimentError::BadTransition {
                id: id.to_string(),
                from: ExperimentStatus::Running,
                to: status,
            });
        }
        let mut r = self.transition(id, &[ExperimentStatus::Running], status)?;
        r.boot_outcome = boot_outcome;
        r.result = result.to_string();
        r.guest_os_build = guest_os_build;
        self.save(&r)?;
        Ok(r)
    }

    /// Attach a log. It is capped, hashed, and the cap is recorded as a limitation.
    pub fn attach_log(
        &self,
        id: &str,
        name: &str,
        bytes: &[u8],
    ) -> Result<ExperimentRecord, ExperimentError> {
        let mut r = self.get(id)?;
        let capped = &bytes[..bytes.len().min(MAX_LOG_BYTES)];
        let rel = format!("logs/{id}-{name}");
        self.root.write(&rel, capped)?;
        r.log_refs.push(ArtifactHash {
            name: rel,
            sha256: sha256_hex(capped),
        });
        if capped.len() < bytes.len() {
            r.limitations
                .push(format!("log {name} was truncated to {MAX_LOG_BYTES} bytes"));
        }
        self.save(&r)?;
        Ok(r)
    }

    /// Recompute the hash of each attached log. Returns the names that no longer match.
    pub fn verify_logs(&self, r: &ExperimentRecord) -> Vec<String> {
        r.log_refs
            .iter()
            .filter(|l| {
                self.root
                    .read(&l.name)
                    .map(|d| sha256_hex(&d) != l.sha256)
                    .unwrap_or(true)
            })
            .map(|l| l.name.clone())
            .collect()
    }

    /// Compare a repeat run with a recorded one. Only matching configuration, firmware and result
    /// count as reproduced. A mismatch marks the original `NOT_REPRODUCIBLE`.
    pub fn record_reproduction(
        &self,
        original_id: &str,
        repeat_id: &str,
    ) -> Result<bool, ExperimentError> {
        let (a, b) = (self.get(original_id)?, self.get(repeat_id)?);
        let same = a.config_sha256 == b.config_sha256
            && a.firmware_artifacts == b.firmware_artifacts
            && a.status == b.status
            && a.result == b.result
            && a.target == b.target;
        let mut a = a;
        a.reproducible = Some(same);
        if !same && a.status == ExperimentStatus::Completed {
            a.status = ExperimentStatus::NotReproducible;
        }
        self.save(&a)?;
        Ok(same)
    }
}
