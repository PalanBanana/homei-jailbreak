//! The canonical target manifest. These tests read the real manifest file.

use homei_core::target::*;
use std::path::Path;

fn load() -> TargetManifest {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../Research/Targets/iPhone16,2-iOS27.0.1.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn the_canonical_manifest_is_valid_and_has_the_exact_tuple() {
    let m = load();
    assert_eq!(m.validate(), Ok(()));
    assert!(m.is_canonical());
    assert_eq!(m.product_type, "iPhone16,2");
    assert_eq!(m.soc, "Apple A17 Pro");
    assert_eq!(m.cpu, "t8130");
    assert_eq!(m.board, "D84AP");
    assert_eq!(m.cpid, "0x8130");
    assert_eq!(m.bdid, "0x06");
    assert_eq!(m.architecture, "arm64e");
    assert_eq!(
        (m.os_version.as_str(), m.build_version.as_str()),
        ("27.0.1", "24A446")
    );
    assert_eq!(m.commercial_model, "iPhone 15 Pro Max");
}

#[test]
fn reference_metadata_never_claims_a_boot() {
    let m = load();
    assert_eq!(m.vm_status, VmStatus::NotTested);
    assert_eq!(m.hardware_fidelity, HardwareFidelity::Unverified);
    for claimed in [
        VmStatus::ExactTargetVerified,
        VmStatus::TargetKernelBootedHardwareUnverified,
        VmStatus::PartialDarwinGuest,
        VmStatus::BootFailure,
    ] {
        let mut bad = m.clone();
        bad.vm_status = claimed;
        assert_eq!(
            bad.validate(),
            Err(TargetError::ReferenceClaimsObservation(claimed))
        );
    }
}

#[test]
fn every_field_must_have_provenance_with_a_source() {
    let m = load();
    for field in PROVENANCE_FIELDS {
        let mut bad = m.clone();
        bad.provenance.remove(*field);
        assert_eq!(
            bad.validate(),
            Err(TargetError::MissingProvenance(field)),
            "{field}"
        );
        let mut empty = m.clone();
        empty.provenance.get_mut(*field).unwrap().source = "  ".into();
        assert_eq!(
            empty.validate(),
            Err(TargetError::EmptyProvenanceSource(field)),
            "{field}"
        );
    }
}

#[test]
fn a_different_model_cannot_be_substituted_silently() {
    let m = load();
    type Swap = (&'static str, fn(&mut TargetManifest));
    let swaps: [Swap; 10] = [
        ("commercial_model", |m| {
            m.commercial_model = "iPhone 16".into()
        }),
        ("product_type", |m| m.product_type = "iPhone17,2".into()),
        ("soc", |m| m.soc = "Apple A18 Pro".into()),
        ("cpu", |m| m.cpu = "t8140".into()),
        ("board", |m| m.board = "D93AP".into()),
        ("cpid", |m| m.cpid = "0x8140".into()),
        ("bdid", |m| m.bdid = "0x07".into()),
        ("architecture", |m| m.architecture = "arm64".into()),
        ("os_version", |m| m.os_version = "27.0".into()),
        ("build_version", |m| m.build_version = "24A437".into()),
    ];
    for (field, f) in swaps {
        let mut bad = m.clone();
        f(&mut bad);
        match bad.validate() {
            Err(TargetError::FieldMismatch { field: got, .. }) => assert_eq!(got, field),
            other => panic!("{field}: {other:?}"),
        }
    }
}

#[test]
fn an_unknown_key_or_a_wrong_schema_is_rejected() {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../Research/Targets/iPhone16,2-iOS27.0.1.json");
    let text = std::fs::read_to_string(path).unwrap();
    let with_extra = text.replacen(
        "\"schema_version\": 1,",
        "\"schema_version\": 1,\n  \"serial_number\": \"x\",",
        1,
    );
    assert!(serde_json::from_str::<TargetManifest>(&with_extra).is_err());
    let mut m = load();
    m.schema_version = 2;
    assert_eq!(m.validate(), Err(TargetError::UnsupportedSchema(2)));
}

#[test]
fn the_known_discrepancy_and_the_secondary_sources_are_recorded_not_hidden() {
    let m = load();
    assert_eq!(m.discrepancies.len(), 1);
    assert!(m.discrepancies[0].summary.contains("t8122"));
    assert_eq!(m.provenance["cpu"].kind, SourceKind::Secondary);
    assert_ne!(m.provenance["cpu"].confidence, Confidence::Verified);
    for primary in [
        "product_type",
        "board",
        "cpid",
        "bdid",
        "os_version",
        "build_version",
    ] {
        assert_eq!(m.provenance[primary].kind, SourceKind::Primary, "{primary}");
        assert_eq!(
            m.provenance[primary].confidence,
            Confidence::Verified,
            "{primary}"
        );
    }
}

#[test]
fn the_manifest_text_has_no_local_path_or_device_identifier() {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../Research/Targets/iPhone16,2-iOS27.0.1.json");
    let text = std::fs::read_to_string(path).unwrap();
    for bad in ["/home/", "/Users/", "UDID", "serial", "IMEI", "ECID"] {
        assert!(!text.contains(bad), "{bad}");
    }
}
