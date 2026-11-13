use homei_core::compatibility::{CompatibilityStatus, Decision};
use homei_core::device_profile::{Provenance, SocFamily};
use homei_core::recovery::{self, GoNoGo, TopicStatus};
use homei_core::safety::{self, Operation};

fn decision(status: CompatibilityStatus) -> Decision {
    Decision {
        status,
        soc_family: Some(SocFamily::A17Pro),
        matched_declaration: None,
        downgraded: false,
        profile_provenance: Provenance::TestFixture,
        explanation: vec![],
    }
}

#[test]
fn operation_names_parse_and_unknown_names_are_kept() {
    assert_eq!(
        Operation::parse("view_compatibility"),
        Operation::ViewCompatibility
    );
    assert_eq!(
        Operation::parse("install_jailbreak"),
        Operation::InstallJailbreak
    );
    assert_eq!(
        Operation::parse("nope"),
        Operation::Unknown("nope".to_string())
    );
    assert_eq!(Operation::parse(""), Operation::Unknown(String::new()));
    assert_eq!(
        Operation::parse("VIEW_COMPATIBILITY"),
        Operation::Unknown("VIEW_COMPATIBILITY".to_string())
    );
}

#[test]
fn go_no_go_is_always_no_go_in_this_release() {
    for status in [
        CompatibilityStatus::Supported,
        CompatibilityStatus::PartiallySupported,
        CompatibilityStatus::NotEstablished,
        CompatibilityStatus::Unsupported,
        CompatibilityStatus::Unknown,
    ] {
        let g = recovery::guidance(&decision(status));
        assert_eq!(g.verdict, GoNoGo::NoGo, "{status:?}");
        assert!(g
            .reasons
            .iter()
            .any(|r| r.contains("no operational backend")));
    }
}

#[test]
fn recovery_topics_cover_the_required_subjects() {
    let topics = recovery::topics();
    for id in [
        "firmware_signing",
        "restore_limits",
        "backup_limits",
        "data_loss",
        "app_compatibility",
        "firmware_preservation",
        "recovery_uncertainty",
    ] {
        assert!(topics.iter().any(|t| t.id == id), "missing {id}");
    }
}

#[test]
fn recovery_text_does_not_promise_restorability() {
    let by_id = |id: &str| recovery::topics().into_iter().find(|t| t.id == id).unwrap();
    assert!(by_id("backup_limits").body.contains("does not guarantee"));
    assert!(by_id("firmware_preservation")
        .body
        .contains("do not by themselves guarantee"));
    assert_eq!(by_id("recovery_uncertainty").status, TopicStatus::Uncertain);
    for t in recovery::topics() {
        assert!(
            !t.body.to_lowercase().contains("guaranteed restore"),
            "{}",
            t.id
        );
    }
}

#[test]
fn signing_observations_are_dated_and_unverified() {
    let obs = recovery::signing_observations();
    assert!(!obs.is_empty());
    for o in obs {
        assert!(!o.verified);
        assert_eq!(o.build, "24A446");
        assert!(o.date.len() == 10 && o.date.as_bytes()[4] == b'-');
    }
}

#[test]
fn read_only_operations_are_the_only_allowed_ones() {
    for name in [
        "view_compatibility",
        "validate_data",
        "export_sanitized_diagnostics",
        "browse_catalog",
        "simulate_bootstrap_plan",
        "view_recovery_info",
    ] {
        assert!(
            safety::authorize(&Operation::parse(name)).is_allowed(),
            "{name}"
        );
    }
}
