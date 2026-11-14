//! Source hygiene for the lab crate. It may touch the filesystem and start a small allowlist of
//! programs, so it gets its own, stricter rules: no device tools, no network, no privilege, no unsafe,
//! and exactly one place where a process is started.

use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn lab_sources() -> Vec<(String, String)> {
    let mut out = Vec::new();
    for entry in fs::read_dir(root().join("Lab/src")).unwrap() {
        let p = entry.unwrap().path();
        if p.is_file() {
            out.push((
                p.file_name().unwrap().to_string_lossy().into_owned(),
                fs::read_to_string(&p).unwrap(),
            ));
        } else if p.is_dir() {
            for e in fs::read_dir(&p).unwrap() {
                let q = e.unwrap().path();
                out.push((
                    format!(
                        "{}/{}",
                        p.file_name().unwrap().to_string_lossy(),
                        q.file_name().unwrap().to_string_lossy()
                    ),
                    fs::read_to_string(&q).unwrap(),
                ));
            }
        }
    }
    out
}

fn code_lines(text: &str) -> impl Iterator<Item = (usize, &str)> {
    text.lines()
        .enumerate()
        .filter(|(_, l)| !l.trim_start().starts_with("//"))
}

#[test]
fn the_lab_has_no_privilege_network_usb_device_or_unsafe_constructs() {
    // `sudo` and the device tools are named here only so the scan can refuse them.
    let forbidden = [
        "sudo",
        "pkexec",
        "doas",
        "std::net",
        "TcpStream",
        "TcpListener",
        "UdpSocket",
        "reqwest",
        "hyper",
        "unsafe {",
        "unsafe fn",
        "unsafe impl",
        "unsafe trait",
        "extern \"C\"",
        "libc::",
        "idevice",
        "usbmuxd",
        "lockdown",
        "libusb",
        "/dev/disk",
        "ssh ",
        "scp ",
        "curl ",
        "wget ",
    ];
    let mut hits = Vec::new();
    for (name, text) in lab_sources() {
        for (i, line) in code_lines(&text) {
            for pat in forbidden {
                // The allowlist test below legitimately contains these words in string literals; Lab/src does not.
                if line.contains(pat) {
                    hits.push(format!("Lab/src/{name}:{}: {pat}", i + 1));
                }
            }
        }
    }
    assert!(
        hits.is_empty(),
        "forbidden constructs:\n{}",
        hits.join("\n")
    );
}

#[test]
fn a_process_is_started_in_exactly_one_place_and_only_through_the_allowlist() {
    let mut spawn_sites = Vec::new();
    for (name, text) in lab_sources() {
        for (i, line) in code_lines(&text) {
            if line.contains("Command::new") {
                spawn_sites.push(format!("{name}:{}", i + 1));
            }
            // A process can only be started from a Command, but check the spawn call itself too.
            if line.contains(".spawn(") && !line.contains("thread::spawn") {
                assert!(
                    name == "process.rs",
                    "{name}:{} spawns outside process.rs",
                    i + 1
                );
            }
        }
    }
    assert_eq!(spawn_sites.len(), 1, "{spawn_sites:?}");
    assert!(spawn_sites[0].starts_with("process.rs:"));
    let process = fs::read_to_string(root().join("Lab/src/process.rs")).unwrap();
    assert!(
        process.contains("if !program_allowed(program)"),
        "the allowlist must be checked before spawning"
    );
    assert!(
        process.contains("ALLOWED_PROGRAMS: &[&str] = &[\"qemu-system-aarch64\", \"qemu-img\"]")
    );
    // Check code lines only. The module comment says "no shell", and prose is not a program name.
    for (n, line) in code_lines(&process) {
        for shell in ["\"sh\"", "\"bash\"", "\"/bin/sh\"", "\"-c\"", "\"cmd\""] {
            assert!(
                !line.contains(shell),
                "process.rs:{}: nothing may run through a shell ({shell})",
                n + 1
            );
        }
    }
}

#[test]
fn filesystem_writes_are_confined_to_the_guard_and_the_workspace_helpers() {
    // Raw filesystem mutation is allowed only in these files. Everything else must use GuestRoot.
    let allowed = [
        "guard.rs",
        "storage.rs",
        "bin/homei-lab.rs",
        "hash.rs",
        "packages.rs",
        "bootstrap.rs",
        "experiments.rs",
    ];
    for (name, text) in lab_sources() {
        for (i, line) in code_lines(&text) {
            for call in [
                "fs::write(",
                "fs::remove_file(",
                "fs::remove_dir_all(",
                "fs::create_dir_all(",
                "fs::rename(",
                "File::create(",
                "OpenOptions",
            ] {
                if line.contains(call) && !allowed.contains(&name.as_str()) {
                    panic!(
                        "Lab/src/{name}:{}: {call} outside the guarded modules",
                        i + 1
                    );
                }
            }
        }
    }
    // packages, bootstrap and experiments reach the disk only through GuestRoot methods.
    for guarded in ["packages.rs", "bootstrap.rs", "experiments.rs"] {
        let text = fs::read_to_string(root().join("Lab/src").join(guarded)).unwrap();
        for (i, line) in code_lines(&text) {
            for raw in [
                "std::fs::",
                "fs::write(",
                "fs::remove",
                "fs::create",
                "fs::rename",
                "File::create",
            ] {
                assert!(!line.contains(raw), "Lab/src/{guarded}:{}: {raw}", i + 1);
            }
        }
    }
}

#[test]
fn the_base_image_is_only_ever_read() {
    let storage = fs::read_to_string(root().join("Lab/src/storage.rs")).unwrap();
    let register = storage
        .split("pub fn register_base")
        .nth(1)
        .unwrap()
        .split("pub fn verify_base")
        .next()
        .unwrap();
    assert!(register.contains("sha256_file(path)") && register.contains("std::fs::metadata(path)"));
    for write in [
        "fs::write(path",
        "File::create(path",
        "OpenOptions",
        "remove_file(path",
    ] {
        assert!(!register.contains(write), "{write}");
    }
}

#[test]
fn the_lab_depends_only_on_the_approved_crates() {
    let toml = fs::read_to_string(root().join("Lab/Cargo.toml")).unwrap();
    let deps = toml
        .split("[dependencies]")
        .nth(1)
        .unwrap()
        .split("[lints]")
        .next()
        .unwrap();
    let names: Vec<&str> = deps
        .lines()
        .filter(|l| l.contains('='))
        .map(|l| l.split(['=', '.']).next().unwrap().trim())
        .collect();
    assert_eq!(
        names,
        ["serde", "serde_json", "homei-core", "sha2", "ed25519-dalek"]
    );
    assert!(!root().join("Lab/build.rs").exists());
}

#[test]
fn nothing_in_the_lab_names_a_physical_device_discovery_or_pairing_function() {
    for (name, text) in lab_sources() {
        for (i, line) in code_lines(&text) {
            let l = line.to_lowercase();
            for word in [
                "pairing record",
                "lockdownd",
                "afc",
                "usb_",
                "get_device_list",
                "idevice_new",
            ] {
                assert!(!l.contains(word), "Lab/src/{name}:{}: {word}", i + 1);
            }
        }
    }
}
