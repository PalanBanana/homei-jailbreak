//! VM status classification and the three-axis fidelity report. Inputs are synthetic evidence.

use homei_core::fidelity::*;
use homei_core::target::*;
use std::collections::BTreeMap;
use std::path::Path;

const REF_KERNEL: &str = "71d3232aff357cf0e26d7252230cbcbc366b1f48d902de3d7fa5c13feea6f243";

fn target() -> TargetManifest {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../Research/Targets/iPhone16,2-iOS27.0.1.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn backend(models_cpu: bool) -> BackendIdentity {
    BackendIdentity {
        name: "darwin-vm (qemu-sptm)".into(),
        version: Some("test".into()),
        machine: Some("darwin".into()),
        cpu_model: Some("max".into()),
        models_target_cpu: models_cpu,
    }
}

fn booted(kernel: &str, product: &str, board: &str, chip: &str, shell: bool) -> BootObservation {
    BootObservation {
        outcome: BootOutcome::Booted,
        kernel_sha256: Some(kernel.into()),
        guest_kernel_version: Some("Darwin Kernel Version 27.0.0".into()),
        device_tree_product: Some(product.into()),
        device_tree_board: Some(board.into()),
        device_tree_chip_id: Some(chip.into()),
        reached_shell: shell,
    }
}

fn evidence(boot: Option<BootObservation>) -> LabEvidence {
    LabEvidence {
        backend: Some(backend(false)),
        qemu_available: true,
        darwin_machine_available: true,
        backend_rejects_target: false,
        firmware_present: true,
        reference_kernel_sha256: Some(REF_KERNEL.into()),
        boot,
        observed_features: BTreeMap::new(),
    }
}

fn exact_boot() -> BootObservation {
    booted(REF_KERNEL, "iPhone16,2", "d84ap", "0x8130", true)
}

#[test]
fn no_backend_means_reference_device_only() {
    let mut e = evidence(None);
    e.backend = None;
    assert_eq!(
        classify_vm_status(&target(), &e),
        VmStatus::ReferenceDeviceOnly
    );
}

#[test]
fn blockers_are_classified_in_a_fixed_order() {
    let t = target();
    let mut e = evidence(None);
    e.qemu_available = false;
    assert_eq!(
        classify_vm_status(&t, &e),
        VmStatus::BlockedByMissingQemuFeature
    );
    let mut e = evidence(None);
    e.darwin_machine_available = false;
    assert_eq!(
        classify_vm_status(&t, &e),
        VmStatus::BlockedByMissingQemuFeature
    );
    let mut e = evidence(None);
    e.firmware_present = false;
    assert_eq!(
        classify_vm_status(&t, &e),
        VmStatus::BlockedByMissingFirmware
    );
    let mut e = evidence(None);
    e.backend_rejects_target = true;
    assert_eq!(classify_vm_status(&t, &e), VmStatus::UnsupportedTarget);
    assert_eq!(classify_vm_status(&t, &evidence(None)), VmStatus::NotTested);
    let mut e = evidence(None);
    e.boot = Some(BootObservation {
        outcome: BootOutcome::NotAttempted,
        ..exact_boot()
    });
    assert_eq!(classify_vm_status(&t, &e), VmStatus::NotTested);
}

#[test]
fn every_failed_boot_outcome_is_a_boot_failure() {
    for outcome in [
        BootOutcome::Timeout,
        BootOutcome::KernelPanic,
        BootOutcome::Interrupted,
        BootOutcome::ProcessFailed,
        BootOutcome::IncompleteOutput,
    ] {
        let e = evidence(Some(BootObservation {
            outcome,
            ..exact_boot()
        }));
        assert_eq!(
            classify_vm_status(&target(), &e),
            VmStatus::BootFailure,
            "{outcome:?}"
        );
    }
}

#[test]
fn a_matching_guest_on_a_generic_machine_is_never_exact() {
    let e = evidence(Some(exact_boot()));
    assert_eq!(
        classify_vm_status(&target(), &e),
        VmStatus::TargetKernelBootedHardwareUnverified
    );
    let a = assess(&target(), &e);
    assert_eq!(a.firmware_identity, FirmwareIdentity::ExactReferenceMatch);
    assert_eq!(a.firmware, FidelityStatus::Exact);
    assert_eq!(a.hardware, FidelityStatus::Approximate);
    assert_eq!(a.hardware_fidelity, HardwareFidelity::Approximate);
    assert!(!a.vm_status.claims_exact_target());
}

#[test]
fn exact_needs_a_backend_that_models_the_target_cpu_and_a_shell() {
    let mut e = evidence(Some(exact_boot()));
    e.backend = Some(backend(true));
    assert_eq!(
        classify_vm_status(&target(), &e),
        VmStatus::ExactTargetVerified
    );
    assert_eq!(assess(&target(), &e).hardware, FidelityStatus::Exact);
    e.boot = Some(booted(REF_KERNEL, "iPhone16,2", "d84ap", "0x8130", false));
    assert_eq!(
        classify_vm_status(&target(), &e),
        VmStatus::TargetKernelBootedHardwareUnverified,
        "no shell, no exact claim"
    );
}

#[test]
fn a_guest_that_is_not_the_target_is_only_a_partial_darwin_guest() {
    let t = target();
    let cases = [
        (
            "wrong kernel build",
            booted(
                "0".repeat(64).as_str(),
                "iPhone16,2",
                "d84ap",
                "0x8130",
                true,
            ),
        ),
        (
            "wrong device model (A18 Pro)",
            booted(REF_KERNEL, "iPhone17,2", "d93ap", "0x8140", true),
        ),
        (
            "wrong board",
            booted(REF_KERNEL, "iPhone16,2", "d83ap", "0x8130", true),
        ),
        (
            "wrong chip id",
            booted(REF_KERNEL, "iPhone16,2", "d84ap", "0x8140", true),
        ),
    ];
    for (name, boot) in cases {
        let mut e = evidence(Some(boot));
        e.backend = Some(backend(true));
        assert_eq!(
            classify_vm_status(&t, &e),
            VmStatus::PartialDarwinGuest,
            "{name}"
        );
    }
    let mut e = evidence(Some(BootObservation {
        guest_kernel_version: None,
        ..exact_boot()
    }));
    e.backend = Some(backend(true));
    assert_eq!(
        classify_vm_status(&t, &e),
        VmStatus::PartialDarwinGuest,
        "no kernel banner"
    );
}

#[test]
fn identity_comparison_is_case_insensitive_for_hashes_and_boards_but_exact_for_the_model() {
    let t = target();
    let ok = booted(
        &REF_KERNEL.to_uppercase(),
        "iPhone16,2",
        "D84AP",
        "0x8130",
        true,
    );
    assert!(boot_matches_target(&t, &Some(REF_KERNEL.into()), &ok));
    let wrong_case_model = booted(REF_KERNEL, "iphone16,2", "d84ap", "0x8130", true);
    assert!(!boot_matches_target(
        &t,
        &Some(REF_KERNEL.into()),
        &wrong_case_model
    ));
    assert!(
        !boot_matches_target(&t, &None, &ok),
        "no reference hash, no match"
    );
}

#[test]
fn the_report_lists_every_feature_and_never_derives_a_positive_from_a_similar_generation() {
    let a = assess(&target(), &evidence(None));
    assert_eq!(a.firmware_identity, FirmwareIdentity::ExactReferenceMatch);
    assert_eq!(
        a.firmware,
        FidelityStatus::Unverified,
        "the reference names the build, but nothing was run"
    );
    assert_eq!(a.hardware, FidelityStatus::Unverified);
    for f in FEATURES {
        assert!(a.functional.contains_key(*f), "{f}");
    }
    for unsupported in ["springboard", "graphics", "wifi", "bluetooth"] {
        assert_eq!(a.functional[unsupported], FidelityStatus::Unsupported);
    }
    assert_eq!(a.functional["guest_boot"], FidelityStatus::NotTested);
    assert_eq!(a.functional["kernel_research"], FidelityStatus::NotTested);
    assert!(a
        .notes
        .iter()
        .any(|n| n.contains("not an exact target virtual machine")));
    assert!(a
        .notes
        .iter()
        .any(|n| n.contains("Guest root is not physical device root")));
}

#[test]
fn guest_root_never_implies_physical_device_root() {
    assert!(PrivilegeScope::GuestRoot.implies(PrivilegeScope::GuestRoot));
    assert!(!PrivilegeScope::GuestRoot.implies(PrivilegeScope::PhysicalDeviceRoot));
    assert!(!PrivilegeScope::PhysicalDeviceRoot.implies(PrivilegeScope::GuestRoot));
}

#[test]
fn only_one_status_claims_an_exact_target() {
    let all = [
        VmStatus::ExactTargetVerified,
        VmStatus::TargetKernelBootedHardwareUnverified,
        VmStatus::PartialDarwinGuest,
        VmStatus::ReferenceDeviceOnly,
        VmStatus::UnsupportedTarget,
        VmStatus::BlockedByMissingFirmware,
        VmStatus::BlockedByMissingQemuFeature,
        VmStatus::BootFailure,
        VmStatus::NotTested,
    ];
    assert_eq!(all.iter().filter(|s| s.claims_exact_target()).count(), 1);
    let names: std::collections::BTreeSet<_> = all.iter().map(|s| s.as_str()).collect();
    assert_eq!(names.len(), 9);
    for s in all {
        assert_eq!(
            serde_json::to_string(&s).unwrap(),
            format!("\"{}\"", s.as_str())
        );
    }
}
