//! Evidence confidence tracker.
//!
//! Every statement in an analysis is tagged with the strongest evidence behind it. The tags
//! follow the project convention: [T] read from a target artifact, [M] read from the signed
//! manifest, [I] inferred from target artifacts, [U] unknown. Only [T] and [M] may be stated as
//! facts. An inference must say what it rests on.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    /// Read directly from a target artifact (a file, a payload or a DeviceTree property).
    Target,
    /// Stated by the signed build manifest for the target build.
    Manifest,
    /// Derived from target artifacts by stated arithmetic or comparison. Not directly observed.
    Inferred,
    /// Not established by any artifact in this analysis.
    Unknown,
}

impl Confidence {
    pub fn tag(self) -> &'static str {
        match self {
            Confidence::Target => "T",
            Confidence::Manifest => "M",
            Confidence::Inferred => "I",
            Confidence::Unknown => "U",
        }
    }

    /// Whether the statement may be presented as a fact.
    pub fn is_fact(self) -> bool {
        matches!(self, Confidence::Target | Confidence::Manifest)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Claim {
    pub id: String,
    pub statement: String,
    pub confidence: Confidence,
    /// The artifact or procedure the claim rests on, named without local paths.
    pub source: String,
    pub limitation: String,
}

/// Issues a claim list. Ids are sequential and stable for a given run order.
#[derive(Debug, Default)]
pub struct ClaimTracker {
    claims: Vec<Claim>,
}

impl ClaimTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(
        &mut self,
        statement: impl Into<String>,
        confidence: Confidence,
        source: impl Into<String>,
        limitation: impl Into<String>,
    ) -> String {
        let id = format!("FA-{:03}", self.claims.len() + 1);
        self.claims.push(Claim {
            id: id.clone(),
            statement: statement.into(),
            confidence,
            source: source.into(),
            limitation: limitation.into(),
        });
        id
    }

    pub fn claims(&self) -> &[Claim] {
        &self.claims
    }

    pub fn into_claims(self) -> Vec<Claim> {
        self.claims
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_target_and_manifest_are_facts() {
        assert!(Confidence::Target.is_fact());
        assert!(Confidence::Manifest.is_fact());
        assert!(!Confidence::Inferred.is_fact());
        assert!(!Confidence::Unknown.is_fact());
        assert_eq!(Confidence::Inferred.tag(), "I");
    }

    #[test]
    fn claim_ids_are_sequential() {
        let mut t = ClaimTracker::new();
        let a = t.add("a", Confidence::Target, "s", "l");
        let b = t.add("b", Confidence::Unknown, "s", "l");
        assert_eq!((a.as_str(), b.as_str()), ("FA-001", "FA-002"));
        assert_eq!(t.claims().len(), 2);
    }
}
