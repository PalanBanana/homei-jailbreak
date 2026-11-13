use homei_core::compatibility;
use homei_core::data;
use homei_core::device_profile::Provenance;
use homei_core::diagnostics;
use std::collections::BTreeSet;

#[test]
fn bundle_loads_with_three_test_fixtures() {
    let b = data::bundled().unwrap();
    let ids: Vec<_> = b.fixtures.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(ids, ["FIX-A", "FIX-B", "FIX-C"]);
    assert!(b
        .fixtures
        .iter()
        .all(|f| f.profile.provenance == Provenance::TestFixture));
}

#[test]
fn fixtures_match_the_specified_records() {
    let b = data::bundled().unwrap();
    let row = |id: &str| {
        let p = &b.fixture(id).unwrap().profile;
        (
            p.product_type.clone(),
            p.product_version.clone(),
            p.build_version.clone(),
            p.soc_family().map(|s| s.name()),
        )
    };
    assert_eq!(
        row("FIX-A"),
        (
            "iPhone16,2".into(),
            "27.0.1".into(),
            "24A446".into(),
            Some("A17 Pro")
        )
    );
    assert_eq!(
        row("FIX-B"),
        (
            "iPhone18,1".into(),
            "27.0.1".into(),
            "24A446".into(),
            Some("A19 Pro")
        )
    );
    assert_eq!(
        row("FIX-C"),
        (
            "iPhone17,2".into(),
            "27.2".into(),
            "24B119".into(),
            Some("A18 Pro")
        )
    );
    assert_eq!(
        b.fixture("FIX-A")
            .unwrap()
            .profile
            .cpu_architecture
            .as_deref(),
        Some("arm64e")
    );
}

#[test]
fn fixtures_carry_no_private_observation_fields() {
    let b = data::bundled().unwrap();
    for f in &b.fixtures {
        assert!(f.profile.hardware_model.is_none(), "{}", f.id);
        assert!(f.profile.device_class.is_none(), "{}", f.id);
    }
}

#[test]
fn ids_are_unique_and_references_resolve() {
    let b = data::bundled().unwrap();
    let ev: BTreeSet<_> = b.evidence.items.iter().map(|e| e.id.clone()).collect();
    assert_eq!(ev.len(), b.evidence.items.len());
    let rf: BTreeSet<_> = b.research.findings.iter().map(|f| f.id.clone()).collect();
    assert_eq!(rf.len(), b.research.findings.len());
    for f in &b.research.findings {
        assert!(!f.evidence_ids.is_empty(), "{}", f.id);
        assert!(f.evidence_ids.iter().all(|id| ev.contains(id)), "{}", f.id);
        assert!(f.tracker_claim_id.starts_with("C-"));
    }
}

#[test]
fn evidence_sources_are_https_and_dated_consistently() {
    let b = data::bundled().unwrap();
    for e in &b.evidence.items {
        assert!(
            e.source_urls.iter().all(|u| u.starts_with("https://")),
            "{}",
            e.id
        );
        for d in [&e.publication_date, &e.access_date].into_iter().flatten() {
            assert!(
                d.len() == 10
                    && d.as_bytes()[4] == b'-'
                    && d.as_str() <= b.research_snapshot_date.as_str(),
                "{} {d}",
                e.id
            );
        }
    }
    assert_eq!(b.research_snapshot_date, "2026-11-12");
}

#[test]
fn validation_checks_all_pass() {
    let b = data::bundled().unwrap();
    for c in diagnostics::validate_bundle(&b) {
        assert!(c.passed, "{}: {}", c.name, c.detail);
    }
}

#[test]
fn validation_detects_a_broken_reference() {
    let mut b = data::bundled().unwrap();
    b.compatibility.declarations[0]
        .evidence_ids
        .push("EV-999".into());
    let checks = diagnostics::validate_bundle(&b);
    assert!(checks
        .iter()
        .any(|c| !c.passed && c.detail.contains("EV-999")));
}

#[test]
fn validation_detects_a_target_that_stops_being_not_established() {
    let mut b = data::bundled().unwrap();
    b.compatibility.declarations[0].status = compatibility::CompatibilityStatus::Unsupported;
    assert!(diagnostics::validate_bundle(&b)
        .iter()
        .any(|c| !c.passed && c.name.contains("NOT ESTABLISHED")));
}

#[test]
fn diagnostic_export_is_sanitized_and_explains_the_decision() {
    let b = data::bundled().unwrap();
    let f = b.fixture("FIX-A").unwrap();
    let d = compatibility::evaluate(&f.profile, &b.compatibility, &b.evidence);
    let json = diagnostics::export(&diagnostics::build_report(&b, &d)).unwrap();
    assert!(json.contains("NOT ESTABLISHED"));
    assert!(json.contains("not a connected device"));
    assert!(json.contains("\"tracker_commit\""));
}

#[test]
fn diagnostic_export_refuses_identifier_like_values() {
    let b = data::bundled().unwrap();
    let f = b.fixture("FIX-A").unwrap();
    let d = compatibility::evaluate(&f.profile, &b.compatibility, &b.evidence);
    for bad in [
        "0123456789abcdef0123456789abcdef01234567",
        "00008130-001234567890ABCD",
    ] {
        let mut report = diagnostics::build_report(&b, &d);
        report.explanation.push(format!("leaked {bad}"));
        assert!(diagnostics::export(&report).is_err(), "{bad}");
    }
}
