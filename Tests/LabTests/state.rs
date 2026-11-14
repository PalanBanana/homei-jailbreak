//! The published lab state: it must decode strictly, validate, say nothing it cannot back, and
//! carry no local path or device identifier.

use homei_core::experiment::{BackendKind, ExperimentStatus};
use homei_core::labstate::LabState;
use homei_core::target::VmStatus;
use std::path::PathBuf;

fn repo_file(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(rel)
}

fn text() -> String {
    std::fs::read_to_string(repo_file("Data/lab/lab-state.json")).expect("Data/lab/lab-state.json")
}

fn state() -> LabState {
    serde_json::from_str(&text()).expect("the lab state decodes strictly")
}

#[test]
fn the_published_state_decodes_validates_and_round_trips() {
    let s = state();
    s.validate().expect("valid");
    let again: serde_json::Value = serde_json::to_value(&s).unwrap();
    let file: serde_json::Value = serde_json::from_str(&text()).unwrap();
    assert_eq!(again, file, "the file holds nothing the types do not know");
}

#[test]
fn the_state_is_for_the_exact_target_and_is_not_a_boot_claim() {
    let s = state();
    assert_eq!(s.target.product_type, "iPhone16,2");
    assert_eq!(s.target.build_version, "24A446");
    assert_ne!(
        s.fidelity.vm_status,
        VmStatus::ExactTargetVerified,
        "no exact VM was demonstrated"
    );
    assert!(!s.fidelity.vm_status.claims_exact_target());
    assert!(!s.guest.vm_running);
    assert!(s
        .fidelity
        .notes
        .iter()
        .any(|n| n.contains("not an exact target")));
    assert!(s
        .fidelity
        .notes
        .iter()
        .any(|n| n.contains("Guest root is not physical device root")));
}

#[test]
fn experiments_are_honest_about_having_no_vm() {
    let s = state();
    assert!(!s.experiments.is_empty());
    for e in &s.experiments {
        assert_ne!(e.backend.kind, BackendKind::Real, "{}", e.experiment_id);
        assert!(e.guest_os_build.is_none(), "{}", e.experiment_id);
        assert!(e.boot_outcome.is_none(), "{}", e.experiment_id);
        assert!(!e.limitations.is_empty(), "{}", e.experiment_id);
        assert_eq!(e.target.build_version, "24A446");
        assert!(e.status.is_terminal() || e.status == ExperimentStatus::Planned);
    }
    assert!(
        s.correlations.is_empty(),
        "no correlation without a real guest"
    );
}

#[test]
fn firmware_metadata_is_for_the_target_build_and_hashes_are_well_formed() {
    let s = state();
    assert!(!s.firmware.is_empty());
    for f in &s.firmware {
        assert_eq!(f.source_build, "24A446", "{}", f.name);
        assert_eq!(f.sha256.len(), 64, "{}", f.name);
        assert!(
            f.sha256.bytes().all(|b| b.is_ascii_hexdigit()),
            "{}",
            f.name
        );
        assert!(!f.name.contains('/'), "{}", f.name);
    }
}

#[test]
fn the_state_contains_no_path_and_no_device_identifier() {
    let t = text();
    for marker in [
        "/home/",
        "/Users/",
        "/tmp/",
        "C:\\",
        "UDID",
        "ECID",
        "IMEI",
        "serial_number",
        "SerialNumber",
        "UniqueChipID",
        "@",
        "api_key",
        "apikey",
        "Bearer ",
        "BEGIN PRIVATE KEY",
        "password",
    ] {
        assert!(!t.contains(marker), "marker {marker:?} found");
    }
}

#[test]
fn the_guest_summary_is_an_isolated_overlay() {
    let s = state();
    assert!(s.guest.filesystem_scope.contains("not a device"));
    assert!(s.guest.overlay_present);
    for p in &s.guest.packages {
        assert!(!p.id.is_empty() && !p.version.is_empty());
    }
}

#[test]
fn the_golden_fixture_matches_the_published_state() {
    // The Swift tests decode this same file strictly.
    let golden = std::fs::read_to_string(repo_file("Tests/golden/lab-state-fixture.json"))
        .expect("Tests/golden/lab-state-fixture.json");
    assert_eq!(golden, text(), "regenerate the fixture from the exporter");
}
