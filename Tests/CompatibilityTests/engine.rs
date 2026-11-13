//! Compatibility engine tests. Synthetic records are used to prove that the engine never turns
//! missing or mismatched evidence into a positive state, and that it can still return one when the
//! exact evidence exists.

use homei_core::compatibility::{self, CompatibilityDb, CompatibilityStatus as S, Declaration};
use homei_core::data;
use homei_core::device_profile::DeviceProfile;
use homei_core::evidence::{
    Confidence, Evidence, EvidenceClass as C, EvidenceScope, EvidenceStore,
};

fn target() -> DeviceProfile {
    serde_json::from_str(
        r#"{"product_type":"iPhone16,2","product_version":"27.0.1","build_version":"24A446","device_class":null,"hardware_model":null,"cpu_architecture":"arm64e","provenance":"test_fixture"}"#,
    )
    .unwrap()
}

fn scope(pt: &str, v: &str, b: &str) -> EvidenceScope {
    EvidenceScope {
        product_type: Some(pt.into()),
        product_version: Some(v.into()),
        build_version: Some(b.into()),
        soc_family: None,
    }
}

fn ev(
    id: &str,
    class: C,
    scope: EvidenceScope,
    confidence: Confidence,
    contradicted: bool,
) -> Evidence {
    Evidence {
        id: id.into(),
        tracker_claim_id: None,
        title: "synthetic".into(),
        class,
        scope,
        source_urls: vec![],
        publication_date: None,
        access_date: None,
        confidence,
        limitations: "synthetic test record".into(),
        contradicting_ids: if contradicted {
            vec!["X".into()]
        } else {
            vec![]
        },
    }
}

fn decl(status: S, ids: &[&str]) -> Declaration {
    Declaration {
        id: "SYN-1".into(),
        product_type: "iPhone16,2".into(),
        soc_family: "A17 Pro".into(),
        product_version: "27.0.1".into(),
        build_version: "24A446".into(),
        architecture: Some("arm64e".into()),
        status,
        evidence_ids: ids.iter().map(|s| s.to_string()).collect(),
        note: "synthetic".into(),
    }
}

fn run(d: Declaration, evidence: Vec<Evidence>) -> compatibility::Decision {
    compatibility::evaluate(
        &target(),
        &CompatibilityDb {
            declarations: vec![d],
        },
        &EvidenceStore::new(evidence),
    )
}

fn exact() -> EvidenceScope {
    scope("iPhone16,2", "27.0.1", "24A446")
}

#[test]
fn supported_requires_exact_target_result_and_independent_reproduction() {
    let d = run(
        decl(S::Supported, &["T", "R"]),
        vec![
            ev(
                "T",
                C::TargetCompatibleResult,
                exact(),
                Confidence::Medium,
                false,
            ),
            ev(
                "R",
                C::IndependentReproduction,
                exact(),
                Confidence::Medium,
                false,
            ),
        ],
    );
    assert_eq!(d.status, S::Supported);
    assert!(!d.downgraded);
}

#[test]
fn supported_without_reproduction_is_downgraded() {
    let d = run(
        decl(S::Supported, &["T"]),
        vec![ev(
            "T",
            C::TargetCompatibleResult,
            exact(),
            Confidence::High,
            false,
        )],
    );
    assert_eq!(d.status, S::NotEstablished);
    assert!(d.downgraded);
}

#[test]
fn partially_supported_needs_at_least_one_exact_result() {
    let ok = run(
        decl(S::PartiallySupported, &["R"]),
        vec![ev(
            "R",
            C::IndependentReproduction,
            exact(),
            Confidence::Medium,
            false,
        )],
    );
    assert_eq!(ok.status, S::PartiallySupported);
    let none = run(decl(S::PartiallySupported, &[]), vec![]);
    assert_eq!(none.status, S::NotEstablished);
    assert!(none.downgraded);
}

#[test]
fn positive_states_are_downgraded_for_every_kind_of_inexact_evidence() {
    let wrong_build = scope("iPhone16,2", "27.0.1", "24A447");
    let wrong_version = scope("iPhone16,2", "27.2", "24A446");
    let wrong_device = scope("iPhone18,1", "27.0.1", "24A446");
    let soc_only = EvidenceScope {
        product_type: None,
        product_version: None,
        build_version: None,
        soc_family: Some("A17 Pro".into()),
    };
    let version_only = EvidenceScope {
        product_type: None,
        product_version: Some("27.0.1".into()),
        build_version: None,
        soc_family: None,
    };
    for (name, s) in [
        ("build", wrong_build),
        ("version", wrong_version),
        ("device", wrong_device),
        ("soc only", soc_only),
        ("version only", version_only),
    ] {
        let d = run(
            decl(S::Supported, &["T", "R"]),
            vec![
                ev(
                    "T",
                    C::TargetCompatibleResult,
                    s.clone(),
                    Confidence::High,
                    false,
                ),
                ev("R", C::IndependentReproduction, s, Confidence::High, false),
            ],
        );
        assert_eq!(d.status, S::NotEstablished, "{name}");
        assert!(d.downgraded, "{name}");
    }
}

#[test]
fn low_confidence_contradicted_or_wrong_class_evidence_does_not_count() {
    let cases = [
        (
            "low confidence",
            ev(
                "T",
                C::TargetCompatibleResult,
                exact(),
                Confidence::Low,
                false,
            ),
        ),
        (
            "contradicted",
            ev(
                "T",
                C::TargetCompatibleResult,
                exact(),
                Confidence::High,
                true,
            ),
        ),
        (
            "press-style statement",
            ev(
                "T",
                C::PublicSourceStatement,
                exact(),
                Confidence::High,
                false,
            ),
        ),
        (
            "reported runtime result",
            ev(
                "T",
                C::ReportedRuntimeResult,
                exact(),
                Confidence::High,
                false,
            ),
        ),
        (
            "static observation",
            ev(
                "T",
                C::StaticBinaryObservation,
                exact(),
                Confidence::High,
                false,
            ),
        ),
        (
            "unverified",
            ev("T", C::Unverified, exact(), Confidence::High, false),
        ),
    ];
    for (name, e) in cases {
        let d = run(decl(S::PartiallySupported, &["T"]), vec![e]);
        assert_eq!(d.status, S::NotEstablished, "{name}");
    }
}

#[test]
fn evidence_not_cited_by_the_record_is_ignored() {
    let d = run(
        decl(S::Supported, &[]),
        vec![
            ev(
                "T",
                C::TargetCompatibleResult,
                exact(),
                Confidence::High,
                false,
            ),
            ev(
                "R",
                C::IndependentReproduction,
                exact(),
                Confidence::High,
                false,
            ),
        ],
    );
    assert_eq!(d.status, S::NotEstablished);
}

#[test]
fn negative_and_neutral_declarations_pass_through() {
    for s in [S::Unsupported, S::NotEstablished, S::Unknown] {
        assert_eq!(run(decl(s, &[]), vec![]).status, s);
    }
}

#[test]
fn other_versions_builds_or_devices_never_transfer() {
    let db = CompatibilityDb {
        declarations: vec![decl(S::NotEstablished, &[])],
    };
    let store = EvidenceStore::default();
    let mut p = target();
    p.product_version = "27.2".into();
    p.build_version = "24B119".into();
    assert_eq!(
        compatibility::evaluate(&p, &db, &store).status,
        S::NotEstablished
    );

    let mut other = target();
    other.product_type = "iPhone18,1".into();
    assert_eq!(
        compatibility::evaluate(&other, &db, &store).status,
        S::Unknown
    );
}

#[test]
fn architecture_mismatch_is_not_an_exact_match() {
    let mut p = target();
    p.cpu_architecture = Some("arm64".into());
    let db = CompatibilityDb {
        declarations: vec![decl(S::Supported, &[])],
    };
    let d = compatibility::evaluate(&p, &db, &EvidenceStore::default());
    assert_eq!(d.status, S::NotEstablished);
    assert!(d.matched_declaration.is_none());
}

#[test]
fn invalid_profiles_and_unmapped_devices_are_unknown() {
    let db = CompatibilityDb {
        declarations: vec![decl(S::Supported, &[])],
    };
    let store = EvidenceStore::default();
    let mut bad = target();
    bad.hardware_model = Some("0123456789abcdef0123456789abcdef01234567".into());
    assert_eq!(
        compatibility::evaluate(&bad, &db, &store).status,
        S::Unknown
    );
    let mut unmapped = target();
    unmapped.product_type = "iPhone99,9".into();
    assert_eq!(
        compatibility::evaluate(&unmapped, &db, &store).status,
        S::Unknown
    );
}

#[test]
fn bundled_target_is_not_established_and_never_positive() {
    let b = data::bundled().unwrap();
    for f in &b.fixtures {
        let d = compatibility::evaluate(&f.profile, &b.compatibility, &b.evidence);
        assert!(!d.status.is_positive(), "{} must not be positive", f.id);
        assert_eq!(d.status, S::NotEstablished, "{}", f.id);
        assert!(!d.downgraded);
    }
    let a = b.fixture("FIX-A").unwrap();
    let d = compatibility::evaluate(&a.profile, &b.compatibility, &b.evidence);
    assert_eq!(d.matched_declaration.as_deref(), Some("HC-001"));
    assert_eq!(d.status.label(), "NOT ESTABLISHED");
}

#[test]
fn no_bundled_declaration_claims_a_positive_state() {
    let b = data::bundled().unwrap();
    assert!(b
        .compatibility
        .declarations
        .iter()
        .all(|d| !d.status.is_positive()));
    assert!(b
        .evidence
        .items
        .iter()
        .all(|e| e.class != C::TargetCompatibleResult && e.class != C::IndependentReproduction));
}

#[test]
fn no_jailbreak_action_is_offered() {
    assert!(!compatibility::jailbreak_action_available());
    assert!(compatibility::RESEARCH_ONLY_LABEL.starts_with("Research only"));
}

#[test]
fn labels_are_distinct_and_only_two_are_positive() {
    let all = [
        S::Supported,
        S::Unsupported,
        S::PartiallySupported,
        S::NotEstablished,
        S::Unknown,
    ];
    let labels: std::collections::BTreeSet<_> = all.iter().map(|s| s.label()).collect();
    assert_eq!(labels.len(), all.len());
    assert_eq!(all.iter().filter(|s| s.is_positive()).count(), 2);
}
