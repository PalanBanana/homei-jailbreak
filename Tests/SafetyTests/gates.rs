use homei_bootstrap::{models::Manifest, planning, simulation};
use homei_core::compatibility::{self, CompatibilityStatus};
use homei_core::data;
use homei_core::safety::{self, ArtifactOrigin, Operation, SafetyDecision};

const PRIVILEGED: &[&str] = &[
    "install_jailbreak",
    "execute_exploit",
    "escalate_privileges",
    "modify_kernel_memory",
    "bypass_sptm",
    "bypass_txm",
    "bypass_amfi",
    "extract_protected_memory",
    "modify_root_filesystem",
    "install_bootstrap",
    "inject_tweak",
    "install_ipa",
    "install_package",
    "modify_trust_cache",
    "reboot_device",
    "enter_dfu_or_recovery",
    "update_firmware",
    "restore_firmware",
    "downgrade_firmware",
    "modify_ota_settings",
    "modify_configuration_profile",
    "connect_usb_device",
    "pair_device",
    "create_backup",
    "collect_sysdiagnose",
    "run_remote_script",
];

#[test]
fn every_privileged_or_device_operation_is_denied_with_a_reason() {
    for name in PRIVILEGED {
        match safety::authorize(&Operation::parse(name)) {
            SafetyDecision::Denied { reason } => assert!(!reason.is_empty(), "{name}"),
            SafetyDecision::Allowed => panic!("{name} must be denied"),
        }
    }
}

#[test]
fn unknown_operations_are_denied_by_default() {
    for name in [
        "",
        "frobnicate",
        "install_jailbreak ",
        "InstallJailbreak",
        "view_compatibility;rm -rf /",
        "../../etc/passwd",
    ] {
        assert!(
            !safety::authorize(&Operation::parse(name)).is_allowed(),
            "{name:?}"
        );
    }
}

#[test]
fn device_connection_and_remote_scripts_have_specific_denials() {
    let reason = |n: &str| match safety::authorize(&Operation::parse(n)) {
        SafetyDecision::Denied { reason } => reason,
        _ => panic!(),
    };
    assert!(reason("pair_device").contains("no device connection or pairing code"));
    assert!(reason("connect_usb_device").contains("separate, explicit authorisation"));
    assert!(reason("run_remote_script").contains("never executed"));
}

#[test]
fn external_artifacts_are_rejected_until_verified() {
    assert!(!safety::authorize_artifact(ArtifactOrigin::ExternalUnverified).is_allowed());
    assert!(safety::authorize_artifact(ArtifactOrigin::BundledFixture).is_allowed());
}

#[test]
fn unverified_compatibility_never_unlocks_anything() {
    for s in [
        CompatibilityStatus::NotEstablished,
        CompatibilityStatus::Unknown,
        CompatibilityStatus::Unsupported,
    ] {
        assert!(!safety::require_verified_compatibility(s.label(), s.is_positive()).is_allowed());
    }
}

#[test]
fn every_privileged_capability_is_unavailable_and_returns_unsupported() {
    let caps = safety::privileged_capabilities();
    assert_eq!(caps.len(), 12);
    for needed in [
        "Kernel read",
        "Kernel write",
        "SPTM bypass",
        "AMFI compatibility",
        "Complete jailbreak",
        "Persistence",
        "Bootstrap",
    ] {
        assert!(caps.iter().any(|c| c.name == needed), "{needed}");
    }
    for c in caps {
        assert_eq!(c.state, "unavailable");
        let err = safety::invoke_capability(c.name).unwrap_err();
        assert!(err.reason.contains("No verified backend"));
    }
}

#[test]
fn simulation_never_writes_and_is_blocked_for_the_target() {
    let b = data::bundled().unwrap();
    let a = b.fixture("FIX-A").unwrap();
    let d = compatibility::evaluate(&a.profile, &b.compatibility, &b.evidence);
    let plan = planning::plan(&Manifest::bundled().unwrap(), &["fixture.catalog-client"]).unwrap();
    let r = simulation::simulate(&plan, &d);
    assert_eq!(r.writes_performed, 0);
    assert!(!r.system_changes);
    assert!(!r.installable);
    assert!(r
        .steps
        .iter()
        .all(|s| s.outcome == simulation::StepOutcome::BlockedByCompatibility));
    assert!(r.status_lines[0].starts_with("SIMULATION ONLY"));
    assert!(r
        .status_lines
        .iter()
        .all(|l| !l.to_lowercase().contains("success")));
}
