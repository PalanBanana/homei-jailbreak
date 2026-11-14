//! VM backend tests with MOCKED process output.
//!
//! Nothing here starts QEMU or boots a guest. Every result below comes from a mock runner, a
//! synthetic log or a synthetic observation, and none of it is evidence of a real boot.

#[path = "support.rs"]
mod support;

use homei_core::fidelity::{BootOutcome, FidelityStatus};
use homei_core::target::{TargetManifest, VmStatus};
use homei_lab::backend::*;
use homei_lab::process::*;
use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;
use support::TempDir;

const QEMU: &str = "qemu-system-aarch64";
const STOCK_MACHINES: &str = "Supported machines are:\nvirt                 QEMU 11.1 ARM Virtual Machine (alias of virt-11.1)\nraspi3b              Raspberry Pi 3B (revision 1.2)\n";
const FORK_MACHINES: &str = "Supported machines are:\ndarwin               Darwin (Apple Silicon) machine\nvirt                 QEMU ARM Virtual Machine\n";
const ACCEL: &str = "Accelerators supported in QEMU binary:\ntcg\n";

fn target() -> TargetManifest {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../Research/Targets/iPhone16,2-iOS27.0.1.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn runner(machines: &str) -> MockRunner {
    MockRunner::new()
        .exits(
            "qemu-system-aarch64 --version",
            0,
            "QEMU emulator version 11.1.3\nCopyright (c) 2003-2026 Fabrice Bellard\n",
        )
        .exits("qemu-system-aarch64 -accel help", 0, ACCEL)
        .exits("qemu-system-aarch64 -M help", 0, machines)
}

fn firmware_dir(files: &[&str]) -> TempDir {
    let t = TempDir::new("fw");
    for f in files {
        std::fs::write(t.0.join(f), format!("synthetic {f}")).unwrap();
    }
    t
}

const ALL: [&str; 6] = [
    "bootkc",
    "dtree",
    "ramdisk.dmg",
    "ramdisk.tc",
    "sptm",
    "txm",
];

#[test]
fn qemu_unavailable_is_reported_and_blocks_a_launch() {
    let t = firmware_dir(&ALL);
    let r = preflight(&MockRunner::new(), QEMU, &t.0, 30, 8, false);
    assert!(!r.qemu_found && !r.can_launch());
    assert!(r.blockers.iter().any(|b| b.contains("was not found")));
    assert!(r.uses_mock_runner, "a mock result must say it is a mock");
}

#[test]
fn stock_qemu_without_the_darwin_machine_is_a_missing_feature() {
    let t = firmware_dir(&ALL);
    let r = preflight(&runner(STOCK_MACHINES), QEMU, &t.0, 30, 8, false);
    assert!(r.qemu_found && !r.darwin_machine_available && !r.can_launch());
    assert_eq!(r.qemu_version.as_deref(), Some("11.1.3"));
    assert_eq!(r.accelerators, ["tcg"]);
    assert!(r
        .blockers
        .iter()
        .any(|b| b.contains("no 'darwin' machine type")));
    let ev = evidence_from(&r, None, None);
    assert_eq!(
        backend_status(&target(), &r, &ev).vm_status,
        VmStatus::BlockedByMissingQemuFeature
    );
}

#[test]
fn missing_firmware_is_listed_by_name_and_blocks_a_launch() {
    let t = firmware_dir(&["bootkc", "txm"]);
    let r = preflight(&runner(FORK_MACHINES), QEMU, &t.0, 30, 8, false);
    assert!(r.darwin_machine_available && !r.firmware_complete() && !r.can_launch());
    let msg = r
        .blockers
        .iter()
        .find(|b| b.contains("missing firmware"))
        .unwrap();
    for name in ["dtree", "ramdisk.dmg", "ramdisk.tc", "sptm"] {
        assert!(msg.contains(name), "{name}");
    }
    let ev = evidence_from(&r, None, None);
    assert_eq!(
        backend_status(&target(), &r, &ev).vm_status,
        VmStatus::BlockedByMissingFirmware
    );
}

#[test]
fn missing_memory_capability_blocks_a_launch() {
    let t = firmware_dir(&ALL);
    let r = preflight(&runner(FORK_MACHINES), QEMU, &t.0, 5, 8, false);
    assert!(r.firmware_complete() && !r.can_launch());
    assert!(r
        .blockers
        .iter()
        .any(|b| b.contains("5 GiB available, 8 GiB requested")));
}

#[test]
fn with_everything_present_a_launch_is_possible_but_nothing_is_started() {
    let t = firmware_dir(&ALL);
    let runner = runner(FORK_MACHINES);
    let r = preflight(&runner, QEMU, &t.0, 30, 8, true);
    assert!(r.can_launch(), "{:?}", r.blockers);
    assert!(r.firmware.iter().all(|f| f.sha256.is_some()));
    let calls = runner.calls.borrow();
    assert_eq!(
        calls.len(),
        3,
        "preflight runs --version, -accel help and -M help and nothing else: {calls:?}"
    );
    assert!(calls
        .iter()
        .all(|c| c.starts_with("qemu-system-aarch64 --version") || c.contains("help")));
    // Even with a clean preflight the status is NOT_TESTED: nothing has booted.
    let ev = evidence_from(&r, None, None);
    assert_eq!(
        backend_status(&target(), &r, &ev).vm_status,
        VmStatus::NotTested
    );
}

#[test]
fn the_launch_plan_is_a_dry_run_with_no_network_usb_or_host_sharing() {
    let plan = plan_launch("qemu-sptm/build/qemu-system-aarch64", "firmware", 8, 600);
    let line = plan.command_line();
    assert!(line.starts_with("qemu-sptm/build/qemu-system-aarch64 -M darwin "));
    for needed in [
        "-bootkc firmware/bootkc",
        "-dtree firmware/dtree",
        "-ramdisk firmware/ramdisk.dmg",
        "-sptm firmware/sptm",
        "-txm firmware/txm",
        "-m 8G",
        "-nographic",
    ] {
        assert!(line.contains(needed), "{needed}\n{line}");
    }
    for forbidden in [
        "-nic", "-netdev", "-net ", "-usb", "-device", "-virtfs", "-fsdev", "-chardev", "socket",
        "hostfwd", "-drive", "/dev/", "tcp:", "udp:",
    ] {
        assert!(!line.contains(forbidden), "{forbidden}");
    }
    assert!(plan.notes.iter().any(|n| n.contains("No network device")));
}

#[test]
fn the_real_runner_refuses_programs_that_are_not_allowlisted() {
    for program in [
        "sudo",
        "sh",
        "bash",
        "rm",
        "idevice_id",
        "ideviceinfo",
        "/usr/bin/sudo",
        "usbmuxd",
        "ssh",
        "curl",
    ] {
        assert!(!program_allowed(program), "{program}");
        let r = RealRunner.run(
            program,
            &["-c".into(), "true".into()],
            Duration::from_secs(1),
        );
        assert_eq!(r.status, RunStatus::NotAllowed, "{program}");
    }
    assert!(
        program_allowed("qemu-system-aarch64")
            && program_allowed("/opt/x/qemu-system-aarch64")
            && program_allowed("qemu-img")
    );
    assert!(!RealRunner.is_mock() && MockRunner::new().is_mock());
}

fn dtree(product: &str, board: &str, chip: &str) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("product".into(), product.into()),
        ("board".into(), board.into()),
        ("chip_id".into(), chip.into()),
    ])
}

const BANNER: &str = "Darwin Kernel Version 27.0.0: Thu Aug 13 21:40:21 PDT 2026; root:xnu-13432.2.10~2/RELEASE_ARM64_T8122";

#[test]
fn a_synthetic_log_with_a_banner_and_a_shell_is_classified_as_a_boot() {
    // MOCK: a synthetic log, not output from a real guest.
    let log = format!("{BANNER}\nlaunchd: starting\nbash-3.2# ");
    let o = observe_boot(
        &RunStatus::Exited(0),
        &log,
        Some("abc".into()),
        &dtree("iPhone16,2", "d84ap", "0x8130"),
    );
    assert_eq!(o.outcome, BootOutcome::Booted);
    assert!(o.reached_shell);
    assert_eq!(o.guest_kernel_version.as_deref(), Some(BANNER));
}

#[test]
fn boot_timeout_kernel_panic_and_incomplete_output_are_failures() {
    let d = dtree("iPhone16,2", "d84ap", "0x8130");
    let banner_only = format!("{BANNER}\nbooting...");
    assert_eq!(
        observe_boot(&RunStatus::Timeout, &banner_only, None, &d).outcome,
        BootOutcome::Timeout
    );
    assert_eq!(
        observe_boot(&RunStatus::Timeout, "", None, &d).outcome,
        BootOutcome::Timeout
    );
    let panic = format!("{BANNER}\npanic(cpu 0 caller 0xfffffff0): Kernel data abort\n");
    assert_eq!(
        observe_boot(&RunStatus::Exited(0), &panic, None, &d).outcome,
        BootOutcome::KernelPanic
    );
    assert_eq!(
        observe_boot(
            &RunStatus::Exited(0),
            "starting\nnothing useful\n",
            None,
            &d
        )
        .outcome,
        BootOutcome::IncompleteOutput
    );
    assert_eq!(
        observe_boot(&RunStatus::Exited(1), "", None, &d).outcome,
        BootOutcome::ProcessFailed
    );
    assert_eq!(
        observe_boot(&RunStatus::NotFound, "", None, &d).outcome,
        BootOutcome::ProcessFailed
    );
    assert_eq!(
        observe_boot(&RunStatus::Failed("killed".into()), &banner_only, None, &d).outcome,
        BootOutcome::ProcessFailed
    );
}

#[test]
fn a_panic_after_a_banner_is_never_a_boot_even_with_a_shell_prompt_in_the_log() {
    let d = dtree("iPhone16,2", "d84ap", "0x8130");
    let log = format!("{BANNER}\nbash-3.2# \npanic(cpu 0 caller 0x1): oops\n");
    assert_eq!(
        observe_boot(&RunStatus::Timeout, &log, None, &d).outcome,
        BootOutcome::Timeout
    );
    assert_eq!(
        observe_boot(&RunStatus::Exited(0), &log, None, &d).outcome,
        BootOutcome::KernelPanic
    );
}

#[test]
fn a_booted_mock_guest_is_still_not_an_exact_target_vm() {
    let t = target();
    let ref_hash = "71d3232aff357cf0e26d7252230cbcbc366b1f48d902de3d7fa5c13feea6f243";
    let tmp = firmware_dir(&ALL);
    let r = preflight(&runner(FORK_MACHINES), QEMU, &tmp.0, 30, 8, false);
    let boot = observe_boot(
        &RunStatus::Exited(0),
        &format!("{BANNER}\nbash-3.2# "),
        Some(ref_hash.into()),
        &dtree("iPhone16,2", "d84ap", "0x8130"),
    );
    let ev = evidence_from(&r, Some(ref_hash.into()), Some(boot));
    let status = backend_status(&t, &r, &ev);
    assert_eq!(
        status.vm_status,
        VmStatus::TargetKernelBootedHardwareUnverified
    );
    let report = homei_core::fidelity::assess(&t, &ev);
    assert_eq!(report.hardware, FidelityStatus::Approximate);
    assert!(
        !ev.backend.as_ref().unwrap().models_target_cpu,
        "darwin-vm uses a generic max CPU"
    );
}

#[test]
fn a_guest_for_the_wrong_model_or_build_stays_partial() {
    let t = target();
    let ref_hash = "71d3232aff357cf0e26d7252230cbcbc366b1f48d902de3d7fa5c13feea6f243";
    let tmp = firmware_dir(&ALL);
    let r = preflight(&runner(FORK_MACHINES), QEMU, &tmp.0, 30, 8, false);
    let log = format!("{BANNER}\nbash-3.2# ");
    for (name, hash, d) in [
        (
            "A18 Pro model",
            ref_hash,
            dtree("iPhone17,2", "d93ap", "0x8140"),
        ),
        (
            "wrong build",
            "f".repeat(64).leak() as &str,
            dtree("iPhone16,2", "d84ap", "0x8130"),
        ),
    ] {
        let boot = observe_boot(&RunStatus::Exited(0), &log, Some(hash.into()), &d);
        let ev = evidence_from(&r, Some(ref_hash.into()), Some(boot));
        assert_eq!(
            backend_status(&t, &r, &ev).vm_status,
            VmStatus::PartialDarwinGuest,
            "{name}"
        );
    }
}

#[test]
fn the_upstream_pins_and_the_optional_backend_are_recorded() {
    assert_eq!(DARWIN_VM_COMMIT.len(), 12);
    assert_eq!(QEMU_SPTM_COMMIT.len(), 12);
    let c = corellium_status();
    assert!(c.optional && !c.available);
    assert_eq!(c.vm_status, VmStatus::NotTested);
    assert!(c.blockers.iter().any(|b| b.contains("no integration")));
}
