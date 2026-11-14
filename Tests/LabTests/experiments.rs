//! Experiment records, their state machine, log integrity, reproducibility and comparison.

#[path = "support.rs"]
mod support;

use homei_core::experiment::*;
use homei_core::fidelity::BootOutcome;
use homei_lab::experiments::*;
use homei_lab::guard::GuestRoot;
use std::collections::BTreeMap;
use support::*;

fn target(product: &str, soc: &str, board: &str, build: &str) -> TargetRef {
    TargetRef {
        target_id: format!("{product}-{build}"),
        product_type: product.into(),
        soc: soc.into(),
        board: board.into(),
        build_version: build.into(),
    }
}

fn a17() -> TargetRef {
    target("iPhone16,2", "Apple A17 Pro", "D84AP", "24A446")
}

fn backend(kind: BackendKind) -> BackendRef {
    BackendRef {
        name: "darwin-vm (qemu-sptm)".into(),
        kind,
        version: Some("test".into()),
        qemu_version: Some("11.1.3".into()),
    }
}

fn spec(t: TargetRef, kind: BackendKind, config: &[(&str, &str)]) -> ExperimentSpec {
    ExperimentSpec {
        kind: "test".into(),
        target: t,
        backend: backend(kind),
        firmware_artifacts: vec![ArtifactHash {
            name: "kernelcache".into(),
            sha256: "a".repeat(64),
        }],
        config: config
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect::<BTreeMap<_, _>>(),
        inputs: vec!["input".into()],
        limitations: vec!["synthetic".into()],
        snapshot_id: None,
    }
}

fn store(tag: &str) -> (TempDir, ExperimentStore) {
    let t = TempDir::new(tag);
    let s = ExperimentStore::open(GuestRoot::create(&t.0.join("ws")).unwrap()).unwrap();
    (t, s)
}

#[test]
fn a_planned_experiment_keeps_its_target_backend_and_exact_build() {
    let (_t, s) = store("plan");
    let r = s
        .plan(spec(a17(), BackendKind::None, &[("k", "v")]), NOW)
        .unwrap();
    assert_eq!(r.experiment_id, "exp-0001");
    assert_eq!(r.status, ExperimentStatus::Planned);
    assert_eq!(r.target.product_type, "iPhone16,2");
    assert_eq!(r.target.build_version, "24A446");
    assert_eq!(r.backend.kind, BackendKind::None);
    assert_eq!(
        r.guest_os_build, None,
        "a guest build is only recorded when it was observed"
    );
    assert_eq!(r.config_sha256.len(), 64);
    assert_eq!(s.get("exp-0001").unwrap(), r);
}

#[test]
fn the_state_machine_allows_only_planned_running_then_one_terminal_state() {
    let (_t, s) = store("fsm");
    let r = s.plan(spec(a17(), BackendKind::Mock, &[]), NOW).unwrap();
    let id = &r.experiment_id;
    assert!(
        matches!(
            s.finish(id, ExperimentStatus::Completed, None, "x", None),
            Err(ExperimentError::BadTransition { .. })
        ),
        "cannot finish what never started"
    );
    s.start(id).unwrap();
    assert!(matches!(
        s.start(id),
        Err(ExperimentError::BadTransition { .. })
    ));
    assert!(matches!(
        s.finish(id, ExperimentStatus::Planned, None, "x", None),
        Err(ExperimentError::BadTransition { .. })
    ));
    assert!(matches!(
        s.finish(id, ExperimentStatus::Running, None, "x", None),
        Err(ExperimentError::BadTransition { .. })
    ));
    assert!(matches!(
        s.finish(id, ExperimentStatus::NotReproducible, None, "x", None),
        Err(ExperimentError::BadTransition { .. })
    ));
    let done = s
        .finish(
            id,
            ExperimentStatus::Failed,
            Some(BootOutcome::KernelPanic),
            "panic",
            None,
        )
        .unwrap();
    assert_eq!(done.status, ExperimentStatus::Failed);
    assert!(!done.status.is_success());
    assert!(
        matches!(
            s.finish(id, ExperimentStatus::Completed, None, "again", None),
            Err(ExperimentError::BadTransition { .. })
        ),
        "a terminal record cannot be rewritten"
    );
    assert!(matches!(
        s.get("exp-0099"),
        Err(ExperimentError::NotFound(_))
    ));
}

#[test]
fn only_completed_is_a_success() {
    for (s, ok) in [
        (ExperimentStatus::Completed, true),
        (ExperimentStatus::Failed, false),
        (ExperimentStatus::Interrupted, false),
        (ExperimentStatus::Inconclusive, false),
        (ExperimentStatus::NotReproducible, false),
        (ExperimentStatus::Planned, false),
        (ExperimentStatus::Running, false),
    ] {
        assert_eq!(s.is_success(), ok, "{s:?}");
    }
}

#[test]
fn a_run_that_was_still_running_is_recorded_as_interrupted_never_successful() {
    let t = TempDir::new("interrupt");
    let root = || GuestRoot::create(&t.0.join("ws")).unwrap();
    let id = {
        let s = ExperimentStore::open(root()).unwrap();
        let r = s.plan(spec(a17(), BackendKind::Real, &[]), NOW).unwrap();
        s.start(&r.experiment_id).unwrap();
        r.experiment_id
    };
    let s = ExperimentStore::open(root()).unwrap();
    assert_eq!(s.interrupted_on_open, vec![id.clone()]);
    let r = s.get(&id).unwrap();
    assert_eq!(r.status, ExperimentStatus::Interrupted);
    assert!(!r.status.is_success());
    assert!(r.result.contains("interrupted"));
}

#[test]
fn attached_logs_are_hashed_and_tampering_is_detected() {
    let (t, s) = store("logs");
    let r = s.plan(spec(a17(), BackendKind::None, &[]), NOW).unwrap();
    let r = s
        .attach_log(&r.experiment_id, "serial.log", b"line one\nline two\n")
        .unwrap();
    assert_eq!(r.log_refs.len(), 1);
    assert!(s.verify_logs(&r).is_empty());
    std::fs::write(t.0.join("ws/logs/exp-0001-serial.log"), b"edited").unwrap();
    assert_eq!(
        s.verify_logs(&r),
        vec!["logs/exp-0001-serial.log".to_string()]
    );
    std::fs::remove_file(t.0.join("ws/logs/exp-0001-serial.log")).unwrap();
    assert_eq!(s.verify_logs(&r).len(), 1, "a missing log is a mismatch");
}

#[test]
fn oversized_logs_are_capped_and_the_cap_is_recorded() {
    let (_t, s) = store("cap");
    let r = s.plan(spec(a17(), BackendKind::None, &[]), NOW).unwrap();
    let big = vec![b'x'; MAX_LOG_BYTES + 500];
    let r = s.attach_log(&r.experiment_id, "big.log", &big).unwrap();
    assert!(r.limitations.iter().any(|l| l.contains("truncated")));
    assert!(s.verify_logs(&r).is_empty());
}

#[test]
fn configuration_hashes_ignore_ordering_and_detect_any_change() {
    let m = |p: &[(&str, &str)]| {
        p.iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect::<BTreeMap<_, _>>()
    };
    assert_eq!(
        config_hash(&m(&[("a", "1"), ("b", "2")])),
        config_hash(&m(&[("b", "2"), ("a", "1")]))
    );
    assert_ne!(
        config_hash(&m(&[("a", "1")])),
        config_hash(&m(&[("a", "2")]))
    );
    assert_ne!(config_hash(&m(&[])), config_hash(&m(&[("a", "")])));
}

#[test]
fn a_repeat_run_with_the_same_inputs_and_result_is_reproduced() {
    let (_t, s) = store("repro");
    for _ in 0..2 {
        let r = s
            .plan(spec(a17(), BackendKind::None, &[("k", "v")]), NOW)
            .unwrap();
        s.start(&r.experiment_id).unwrap();
        s.finish(
            &r.experiment_id,
            ExperimentStatus::Completed,
            None,
            "same result",
            None,
        )
        .unwrap();
    }
    assert!(s.record_reproduction("exp-0001", "exp-0002").unwrap());
    let r = s.get("exp-0001").unwrap();
    assert_eq!(r.reproducible, Some(true));
    assert_eq!(r.status, ExperimentStatus::Completed);
}

#[test]
fn a_repeat_run_with_a_different_result_makes_the_original_not_reproducible() {
    let (_t, s) = store("norepro");
    for result in ["first result", "second result"] {
        let r = s
            .plan(spec(a17(), BackendKind::None, &[("k", "v")]), NOW)
            .unwrap();
        s.start(&r.experiment_id).unwrap();
        s.finish(
            &r.experiment_id,
            ExperimentStatus::Completed,
            None,
            result,
            None,
        )
        .unwrap();
    }
    assert!(!s.record_reproduction("exp-0001", "exp-0002").unwrap());
    let r = s.get("exp-0001").unwrap();
    assert_eq!(r.status, ExperimentStatus::NotReproducible);
    assert_eq!(r.reproducible, Some(false));
}

#[test]
fn a_different_configuration_is_not_a_reproduction() {
    let (_t, s) = store("cfg");
    for v in ["1", "2"] {
        let r = s
            .plan(spec(a17(), BackendKind::None, &[("k", v)]), NOW)
            .unwrap();
        s.start(&r.experiment_id).unwrap();
        s.finish(
            &r.experiment_id,
            ExperimentStatus::Completed,
            None,
            "r",
            None,
        )
        .unwrap();
    }
    assert!(!s.record_reproduction("exp-0001", "exp-0002").unwrap());
}

#[test]
fn comparing_an_a18_result_with_an_a17_result_is_rejected() {
    let (_t, s) = store("cross");
    let a = s.plan(spec(a17(), BackendKind::None, &[]), NOW).unwrap();
    let b = s
        .plan(
            spec(
                target("iPhone17,2", "Apple A18 Pro", "D93AP", "24A446"),
                BackendKind::None,
                &[],
            ),
            NOW,
        )
        .unwrap();
    match compare(&a, &b) {
        Err(CompareError::IncompatibleHardware { field, .. }) => assert_eq!(field, "product_type"),
        other => panic!("{other:?}"),
    }
    let mut c = b.clone();
    c.target.product_type = a.target.product_type.clone();
    match compare(&a, &c) {
        Err(CompareError::IncompatibleHardware { field, .. }) => assert_eq!(field, "soc"),
        other => panic!("{other:?}"),
    }
    c.target.soc = a.target.soc.clone();
    match compare(&a, &c) {
        Err(CompareError::IncompatibleHardware { field, .. }) => assert_eq!(field, "board"),
        other => panic!("{other:?}"),
    }
    assert!(matches!(compare(&a, &a), Err(CompareError::SameRecord)));
}

#[test]
fn comparing_the_same_hardware_reports_differences_and_flags_mocks() {
    let (_t, s) = store("same");
    let a = s
        .plan(spec(a17(), BackendKind::Real, &[("k", "1")]), NOW)
        .unwrap();
    let mut b = s
        .plan(spec(a17(), BackendKind::Mock, &[("k", "2")]), NOW)
        .unwrap();
    b.target.build_version = "24B119".into();
    let c = compare(&a, &b).unwrap();
    assert!(!c.same_build && !c.same_config && !c.same_backend);
    assert!(c.notes.iter().any(|n| n.contains("different builds")));
    assert!(c.notes.iter().any(|n| n.contains("mock backend")));
}

#[test]
fn a_correlation_needs_the_same_build_a_completed_real_experiment_and_honest_evidence_kind() {
    let (_t, s) = store("corr");
    let mk = |kind: BackendKind, status: ExperimentStatus| {
        let r = s.plan(spec(a17(), kind, &[]), NOW).unwrap();
        s.start(&r.experiment_id).unwrap();
        s.finish(
            &r.experiment_id,
            status,
            None,
            "r",
            Some("27.0.0 xnu-13432".into()),
        )
        .unwrap()
    };
    let hash = "71d3232a".repeat(8);
    let real = mk(BackendKind::Real, ExperimentStatus::Completed);
    let c = correlate(
        "HOMEI_A17Pro_iOS27_0_1_24A446",
        &hash,
        "24A446",
        &real,
        EvidenceKind::EmulatorObservation,
        "low",
    )
    .unwrap();
    assert_eq!(
        (
            c.build_version.as_str(),
            c.firmware_sha256.as_str(),
            c.guest_identity.as_str()
        ),
        ("24A446", hash.as_str(), "27.0.0 xnu-13432")
    );
    assert!(c
        .limitations
        .iter()
        .any(|l| l.contains("not a statement about physical hardware")));
    assert!(matches!(
        correlate(
            "p",
            &hash,
            "24B119",
            &real,
            EvidenceKind::EmulatorObservation,
            "low"
        ),
        Err(CorrelationError::BuildMismatch { .. })
    ));
    assert_eq!(
        correlate(
            "p",
            &hash,
            "24A446",
            &real,
            EvidenceKind::ReproducedBehavior,
            "low"
        ),
        Err(CorrelationError::OverclaimedEvidence)
    );
    let failed = mk(BackendKind::Real, ExperimentStatus::Failed);
    assert!(matches!(
        correlate(
            "p",
            &hash,
            "24A446",
            &failed,
            EvidenceKind::EmulatorObservation,
            "low"
        ),
        Err(CorrelationError::ExperimentNotCompleted(_))
    ));
    let mock = mk(BackendKind::Mock, ExperimentStatus::Completed);
    assert_eq!(
        correlate(
            "p",
            &hash,
            "24A446",
            &mock,
            EvidenceKind::EmulatorObservation,
            "low"
        ),
        Err(CorrelationError::MockBackend)
    );
}
