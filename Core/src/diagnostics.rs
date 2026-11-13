//! Local diagnostics: build metadata, data validation and compatibility explanations.
//!
//! Diagnostics only describe HOMEI itself and its bundled data. They never read the operating
//! system, other applications, logs or credentials, and an export is checked so it cannot carry
//! identifier-like values.

use crate::compatibility::{CompatibilityStatus, Decision};
use crate::data::Bundle;
use crate::device_profile::{looks_like_identifier, Provenance};
use crate::research::is_cve_id;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Check {
    pub name: String,
    pub passed: bool,
    pub detail: String,
}

fn check(name: &str, problems: Vec<String>) -> Check {
    Check {
        name: name.to_string(),
        passed: problems.is_empty(),
        detail: if problems.is_empty() {
            "ok".to_string()
        } else {
            problems.join("; ")
        },
    }
}

/// Validate the bundled data: reference integrity, fixture hygiene and the target rule.
pub fn validate_bundle(bundle: &Bundle) -> Vec<Check> {
    let mut checks = Vec::new();

    let mut refs = Vec::new();
    for d in &bundle.compatibility.declarations {
        for id in &d.evidence_ids {
            if bundle.evidence.get(id).is_none() {
                refs.push(format!("{} cites missing evidence {id}", d.id));
            }
        }
    }
    for f in &bundle.research.findings {
        for id in &f.evidence_ids {
            if bundle.evidence.get(id).is_none() {
                refs.push(format!("{} cites missing evidence {id}", f.id));
            }
        }
    }
    checks.push(check("evidence references resolve", refs));

    let mut fx = Vec::new();
    for f in &bundle.fixtures {
        if let Err(e) = f.profile.validate() {
            fx.push(format!("{}: {e}", f.id));
        }
        if f.profile.provenance != Provenance::TestFixture {
            fx.push(format!("{} is not marked as a test fixture", f.id));
        }
    }
    checks.push(check("fixtures are valid test fixtures", fx));

    let cves: Vec<String> = bundle
        .research
        .findings
        .iter()
        .flat_map(|f| f.cve_ids.iter())
        .filter(|c| !is_cve_id(c))
        .map(|c| format!("malformed CVE id {c}"))
        .collect();
    checks.push(check("CVE identifiers are well formed", cves));

    let mut target = Vec::new();
    if let Some(fix) = bundle.fixture("FIX-A") {
        let d =
            crate::compatibility::evaluate(&fix.profile, &bundle.compatibility, &bundle.evidence);
        if d.status != CompatibilityStatus::NotEstablished {
            target.push(format!("target fixture evaluates to {}", d.status.label()));
        }
    } else {
        target.push("target fixture FIX-A is missing".to_string());
    }
    checks.push(check("target fixture is NOT ESTABLISHED", target));

    checks
}

/// A sanitized, user-controlled diagnostic report.
#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub product: &'static str,
    pub version: &'static str,
    pub research_snapshot_date: String,
    pub tracker_commit: String,
    pub profile_provenance: String,
    pub compatibility: &'static str,
    pub explanation: Vec<String>,
    pub checks: Vec<Check>,
}

/// Build a report from a decision and the validation results.
pub fn build_report(bundle: &Bundle, decision: &Decision) -> Report {
    Report {
        product: "HOMEI",
        version: env!("CARGO_PKG_VERSION"),
        research_snapshot_date: bundle.research_snapshot_date.clone(),
        tracker_commit: bundle.tracker_commit.clone(),
        profile_provenance: decision.profile_provenance.label().to_string(),
        compatibility: decision.status.label(),
        explanation: decision.explanation.clone(),
        checks: validate_bundle(bundle),
    }
}

/// Export a report as JSON. Refuses to export if any token looks like an identifier.
pub fn export(report: &Report) -> Result<String, String> {
    let json = serde_json::to_string_pretty(report).map_err(|e| e.to_string())?;
    if contains_identifier_like_token(&json) {
        return Err("export blocked: an identifier-like value was found".to_string());
    }
    Ok(json)
}

/// True if any ASCII-alphanumeric token (hyphens allowed) of 14 bytes or more looks like an
/// identifier. The Swift kit implements the same rule, and shared test vectors compare them.
pub fn contains_identifier_like_token(text: &str) -> bool {
    text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
        .any(|token| token.len() >= 14 && looks_like_identifier(token))
}
