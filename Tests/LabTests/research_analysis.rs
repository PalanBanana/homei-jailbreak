//! Firmware analysis and feasibility on the committed, sanitized analysis of the target.
//!
//! The committed bundle under `Data/research` is the output of `homei-lab analyze` on the target
//! restore archive. These tests check that the bundle still describes the fixed target, that the
//! feasibility matrix regenerates from it byte for byte, and that no status is claimed without a
//! stated decision. They also run the parsers on synthetic inputs, so they do not need the
//! firmware on disk.

use homei_lab::analysis::im4p::{self, Container};
use homei_lab::analysis::research::{Availability, Bundle};
use homei_lab::feasibility::{self, FeasibilityMatrix, Status};
use std::io::Cursor;
use std::path::Path;

fn data(name: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("Data/research")
        .join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("cannot read {}: {e}", p.display()))
}

fn bundle() -> Bundle {
    serde_json::from_str(&data("t8130-firmware-analysis.json")).expect("bundle matches its schema")
}

#[test]
fn the_committed_bundle_describes_the_fixed_target() {
    let b = bundle();
    assert_eq!(b.target.product_type, "iPhone16,2");
    assert_eq!(b.target.board_config, "D84AP");
    assert_eq!(b.target.chip, "T8130");
    assert_eq!(b.target.build, "24A446");
    assert_eq!(b.target.product_version, "27.0.1");
    assert!(
        b.identity_checks.iter().all(|c| c.passed),
        "every identity check passes"
    );
    assert_eq!(
        b.components.len(),
        97,
        "the erase identity has 97 components"
    );
    assert_eq!(b.archive.entry_count, 127);
    assert_eq!(b.summary.erase_components, 97);
}

#[test]
fn the_bundle_records_no_local_paths_or_account_details() {
    let text = data("t8130-firmware-analysis.json");
    for needle in ["/home/", "homei-experiments", "/tmp/", "OWNER", "kakao"] {
        assert!(!text.contains(needle), "bundle contains {needle}");
    }
}

#[test]
fn encrypted_components_are_exactly_the_keybag_images() {
    let b = bundle();
    for c in &b.components {
        let keybag = matches!(&c.container, Some(Container::Im4p(i)) if i.keybag_present);
        let encrypted = c.availability == Availability::Encrypted;
        assert_eq!(
            keybag, encrypted,
            "{} keybag and availability disagree",
            c.name
        );
    }
    assert_eq!(b.summary.encrypted, 4);
}

#[test]
fn components_without_a_manifest_path_are_not_called_missing() {
    let b = bundle();
    let none: Vec<&str> = b
        .components
        .iter()
        .filter(|c| c.availability == Availability::NoManifestPath)
        .map(|c| c.name.as_str())
        .collect();
    assert_eq!(none.len(), 4);
    assert_eq!(
        b.summary.missing, 0,
        "no component is reported missing from the archive"
    );
}

#[test]
fn the_aic_window_translation_is_recorded_as_inferred() {
    let b = bundle();
    let aic = b.devicetree.as_ref().unwrap().aic.as_ref().unwrap();
    assert_eq!(aic.translated_base_hex.as_deref(), Some("0x2d1000000"));
    assert_eq!(aic.interrupt_cells, Some(1));
    let claim = b
        .claims
        .iter()
        .find(|c| c.statement.contains("translates"))
        .expect("translation claim exists");
    assert_eq!(
        claim.confidence,
        homei_lab::analysis::confidence::Confidence::Inferred
    );
}

#[test]
fn the_feasibility_matrix_regenerates_from_the_committed_bundle() {
    let b = bundle();
    let matrix = feasibility::evaluate(&b, "2026-11-14").expect("evaluates");
    let committed: FeasibilityMatrix =
        serde_json::from_str(&data("t8130-feasibility.json")).expect("matrix matches its schema");
    assert_eq!(
        matrix, committed,
        "regenerate with `homei-lab feasibility` if this fails"
    );
}

#[test]
fn the_matrix_has_the_sixteen_components_and_no_verified_exact_vm() {
    let m: FeasibilityMatrix = serde_json::from_str(&data("t8130-feasibility.json")).unwrap();
    assert_eq!(m.components.len(), 16);
    assert_eq!(m.exact_target_vm.status, "NOT_ESTABLISHED");
    let ids: Vec<&str> = m.components.iter().map(|c| c.id.as_str()).collect();
    for want in [
        "cpu",
        "memory_map",
        "interrupt_controller",
        "timer",
        "pmgr",
        "display",
        "gpu",
        "usb",
        "storage",
        "sep",
        "sptm",
        "txm",
        "devicetree",
        "kernel",
        "rootfs",
        "gui",
    ] {
        assert!(ids.contains(&want), "missing component {want}");
    }
    assert!(
        m.components.iter().all(|c| c.status != Status::Verified),
        "no component is VERIFIED for the exact target: no boot was observed"
    );
}

#[test]
fn every_decision_names_its_source_type_limitation_and_date() {
    let m: FeasibilityMatrix = serde_json::from_str(&data("t8130-feasibility.json")).unwrap();
    for c in &m.components {
        assert!(!c.decisions.is_empty(), "{} has no decision", c.id);
        for d in &c.decisions {
            assert!(
                !d.source.is_empty() && !d.evidence_type.is_empty(),
                "{}",
                c.id
            );
            assert!(
                !d.limitation.is_empty() && d.last_verified == "2026-11-14",
                "{}",
                c.id
            );
        }
    }
}

#[test]
fn no_status_is_inferred_from_another_soc() {
    let text = data("t8130-feasibility.json");
    for other in ["A16", "A18", "A19", "t8120", "t8103", "t6000"] {
        assert!(!text.contains(other), "matrix mentions {other}");
    }
}

#[test]
fn evaluation_refuses_a_bundle_that_fails_the_identity_check() {
    let mut b = bundle();
    b.identity_checks[1].passed = false;
    let err = feasibility::evaluate(&b, "2026-11-14").unwrap_err();
    assert!(err.contains("refusing"), "{err}");
}

#[test]
fn evaluation_refuses_a_bundle_without_a_device_tree() {
    let mut b = bundle();
    b.devicetree = None;
    assert!(feasibility::evaluate(&b, "2026-11-14").is_err());
}

#[test]
fn an_unreadable_container_is_classified_not_guessed() {
    let c = im4p::inspect(&mut Cursor::new(vec![0x30u8, 0x03, 1, 2, 3])).unwrap();
    assert_eq!(c, Container::Unclassified);
}

#[test]
fn the_sptm_census_summary_matches_the_recorded_counts() {
    let census: serde_json::Value =
        serde_json::from_str(&data("t8130-sptm-census.json")).expect("census is JSON");
    assert_eq!(census["register_count"], 127);
    assert_eq!(census["undefined_in_fork_count"], 98);
    assert_eq!(census["access_rows"], 1444);
    let regs = census["registers"].as_array().expect("register list");
    assert_eq!(regs.len(), 127);
    let undefined = regs
        .iter()
        .filter(|r| r["defined_in_fork"] == false)
        .count();
    assert_eq!(undefined, 98, "the list agrees with the stated count");
}
