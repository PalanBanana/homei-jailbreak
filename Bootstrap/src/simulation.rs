//! Simulated installation and mock status output. Nothing is written anywhere.

use crate::planning::Plan;
use homei_core::compatibility::Decision;
use serde::Serialize;

pub const BANNER: &str = "SIMULATION ONLY. Nothing was installed, written or changed.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum StepOutcome {
    /// The step was computed on paper.
    Simulated,
    /// The step is refused because compatibility is not positive.
    BlockedByCompatibility,
}

#[derive(Debug, Clone, Serialize)]
pub struct SimulatedStep {
    pub component: String,
    pub version: String,
    pub outcome: StepOutcome,
}

#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub banner: &'static str,
    pub steps: Vec<SimulatedStep>,
    /// Always zero: this crate has no code path that writes.
    pub writes_performed: u32,
    pub system_changes: bool,
    pub installable: bool,
    pub status_lines: Vec<String>,
}

/// Simulate a plan against a compatibility decision.
pub fn simulate(plan: &Plan, decision: &Decision) -> Report {
    let allowed = decision.status.is_positive();
    let outcome = if allowed {
        StepOutcome::Simulated
    } else {
        StepOutcome::BlockedByCompatibility
    };
    let steps: Vec<SimulatedStep> = plan
        .steps
        .iter()
        .map(|c| SimulatedStep {
            component: c.id.clone(),
            version: c.version.clone(),
            outcome,
        })
        .collect();
    let mut status_lines = vec![BANNER.to_string()];
    status_lines.push(format!("Compatibility: {}", decision.status.label()));
    for s in &steps {
        let tag = match s.outcome {
            StepOutcome::Simulated => "simulated",
            StepOutcome::BlockedByCompatibility => "blocked",
        };
        status_lines.push(format!("[{tag}] {} {}", s.component, s.version));
    }
    Report {
        banner: BANNER,
        steps,
        writes_performed: 0,
        system_changes: false,
        // Even a positive simulation is not an installation.
        installable: false,
        status_lines,
    }
}
