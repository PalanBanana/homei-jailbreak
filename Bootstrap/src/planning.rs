//! Dependency planning: a deterministic install order for requested components.

use crate::models::{Component, Manifest};
use std::collections::BTreeSet;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanError {
    UnknownComponent(String),
    Cycle(String),
}

impl fmt::Display for PlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlanError::UnknownComponent(id) => write!(f, "unknown component {id}"),
            PlanError::Cycle(id) => write!(f, "dependency cycle through {id}"),
        }
    }
}

impl std::error::Error for PlanError {}

/// Ordered components, dependencies first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub steps: Vec<Component>,
}

/// Plan the requested components and everything they depend on.
pub fn plan(manifest: &Manifest, requested: &[&str]) -> Result<Plan, PlanError> {
    let mut done: BTreeSet<String> = BTreeSet::new();
    let mut visiting: BTreeSet<String> = BTreeSet::new();
    let mut steps = Vec::new();
    for id in requested {
        visit(manifest, id, &mut done, &mut visiting, &mut steps)?;
    }
    Ok(Plan { steps })
}

fn visit(
    manifest: &Manifest,
    id: &str,
    done: &mut BTreeSet<String>,
    visiting: &mut BTreeSet<String>,
    steps: &mut Vec<Component>,
) -> Result<(), PlanError> {
    if done.contains(id) {
        return Ok(());
    }
    if !visiting.insert(id.to_string()) {
        return Err(PlanError::Cycle(id.to_string()));
    }
    let component = manifest
        .get(id)
        .ok_or_else(|| PlanError::UnknownComponent(id.to_string()))?;
    for dep in &component.depends_on {
        visit(manifest, dep, done, visiting, steps)?;
    }
    visiting.remove(id);
    done.insert(id.to_string());
    steps.push(component.clone());
    Ok(())
}
