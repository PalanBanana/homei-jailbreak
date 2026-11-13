use homei_core::device_profile::{looks_like_identifier, DeviceProfile, Provenance, SocFamily};

fn profile(json: &str) -> Result<DeviceProfile, serde_json::Error> {
    serde_json::from_str(json)
}

const TARGET: &str = r#"{"product_type":"iPhone16,2","product_version":"27.0.1","build_version":"24A446","device_class":null,"hardware_model":null,"cpu_architecture":"arm64e","provenance":"test_fixture"}"#;

fn with(field: &str, value: &str) -> DeviceProfile {
    let mut p = profile(TARGET).unwrap();
    match field {
        "product_type" => p.product_type = value.into(),
        "product_version" => p.product_version = value.into(),
        "build_version" => p.build_version = value.into(),
        "hardware_model" => p.hardware_model = Some(value.into()),
        "device_class" => p.device_class = Some(value.into()),
        "cpu_architecture" => p.cpu_architecture = Some(value.into()),
        _ => unreachable!(),
    }
    p
}

#[test]
fn accepts_the_target_profile() {
    let p = profile(TARGET).unwrap();
    assert!(p.validate().is_ok());
    assert_eq!(p.soc_family(), Some(SocFamily::A17Pro));
}

#[test]
fn rejects_unknown_keys_so_identifiers_cannot_be_imported() {
    for key in [
        "serial_number",
        "udid",
        "ecid",
        "imei",
        "eid",
        "apple_account",
    ] {
        let json = TARGET.replace("\"provenance\"", &format!("\"{key}\":\"x\",\"provenance\""));
        assert!(profile(&json).is_err(), "{key} must be rejected");
    }
}

#[test]
fn rejects_identifier_like_values() {
    let hex40 = "0123456789abcdef0123456789abcdef01234567";
    let udid_shape = "00008130-001234567890ABCD";
    let digits = "123456789012345";
    for bad in [hex40, udid_shape, digits] {
        assert!(looks_like_identifier(bad), "{bad}");
        assert!(with("hardware_model", bad).validate().is_err(), "{bad}");
        assert!(with("device_class", bad).validate().is_err(), "{bad}");
        assert!(with("cpu_architecture", bad).validate().is_err(), "{bad}");
    }
    assert!(!looks_like_identifier("iPhone16,2"));
    assert!(!looks_like_identifier("24A446"));
}

#[test]
fn rejects_malformed_product_types() {
    for bad in [
        "",
        "iPhone",
        "iPhone16",
        "16,2",
        "iPhone16,2;rm",
        "iPhone16,2 ",
        "iPhone,2",
        "iPhone16,",
    ] {
        assert!(with("product_type", bad).validate().is_err(), "{bad:?}");
    }
    for good in ["iPhone16,2", "iPhone18,1", "iPad14,3"] {
        assert!(with("product_type", good).validate().is_ok(), "{good}");
    }
}

#[test]
fn rejects_malformed_versions_and_builds() {
    for bad in ["27", "27.0.1.1", "a.b", "", "27..1"] {
        assert!(with("product_version", bad).validate().is_err(), "{bad:?}");
    }
    for bad in ["24a446", "24A", "XYZ", "", "24A446xx", "2424A446"] {
        assert!(with("build_version", bad).validate().is_err(), "{bad:?}");
    }
    for good in ["24A446", "24B119", "24B156", "24A5001a"] {
        assert!(with("build_version", good).validate().is_ok(), "{good}");
    }
}

#[test]
fn provenance_round_trips_in_snake_case() {
    for (p, s) in [
        (Provenance::ManualProfile, "manual_profile"),
        (Provenance::TestFixture, "test_fixture"),
        (
            Provenance::UserAuthorizedReadOnlyObservation,
            "user_authorized_read_only_observation",
        ),
        (
            Provenance::PublicFirmwareMetadata,
            "public_firmware_metadata",
        ),
    ] {
        assert_eq!(serde_json::to_string(&p).unwrap(), format!("\"{s}\""));
        assert_eq!(
            serde_json::from_str::<Provenance>(&format!("\"{s}\"")).unwrap(),
            p
        );
    }
}

#[test]
fn no_provenance_label_claims_a_live_connection() {
    assert!(Provenance::TestFixture
        .label()
        .contains("not a connected device"));
    assert!(Provenance::UserAuthorizedReadOnlyObservation
        .label()
        .contains("not live"));
    assert!(Provenance::UserAuthorizedReadOnlyObservation.is_observation());
    assert!(!Provenance::TestFixture.is_observation());
    assert!(!Provenance::ManualProfile.is_observation());
}

#[test]
fn soc_mapping_covers_only_sourced_product_types() {
    assert_eq!(
        SocFamily::from_product_type("iPhone16,2"),
        Some(SocFamily::A17Pro)
    );
    assert_eq!(
        SocFamily::from_product_type("iPhone17,2"),
        Some(SocFamily::A18Pro)
    );
    assert_eq!(
        SocFamily::from_product_type("iPhone18,1"),
        Some(SocFamily::A19Pro)
    );
    assert_eq!(SocFamily::from_product_type("iPhone16,1"), None);
    assert_eq!(SocFamily::from_product_type("iPhone15,2"), None);
    assert!(SocFamily::A17Pro.basis().contains("partially verified"));
}
