//! Source hygiene: the product crates must not contain code that could reach a device, a
//! network, a subprocess or the filesystem for writing, and must not pull in new dependencies.

use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let p = entry.unwrap().path();
        if p.is_dir() {
            rust_files(&p, out);
        } else if p.extension().is_some_and(|e| e == "rs") {
            out.push(p);
        }
    }
}

const PRODUCT_DIRS: &[&str] = &[
    "Core/src",
    "Catalog/src",
    "Bootstrap/src",
    "Tools/homei-cli/src",
];

const FORBIDDEN: &[&str] = &[
    "std::net",
    "TcpStream",
    "TcpListener",
    "UdpSocket",
    "process::Command",
    "Command::new",
    "unsafe ",
    "extern \"C\"",
    "std::fs::write",
    "fs::write(",
    "File::create",
    "OpenOptions",
    "remove_file",
    "remove_dir",
    "create_dir",
    "libc::",
    "ptrace",
    "dlopen",
    "transmute",
    "std::env::var",
    "std::thread",
    "reqwest",
    "hyper",
    "tokio",
];

#[test]
fn product_code_has_no_device_network_process_or_write_paths() {
    let mut files = Vec::new();
    for d in PRODUCT_DIRS {
        rust_files(&root().join(d), &mut files);
    }
    assert!(
        files.len() >= 10,
        "expected to scan the product sources, found {}",
        files.len()
    );
    let mut hits = Vec::new();
    for f in &files {
        for (i, line) in fs::read_to_string(f).unwrap().lines().enumerate() {
            let code = line.trim_start();
            if code.starts_with("//") {
                continue;
            }
            for pat in FORBIDDEN {
                if code.contains(pat) {
                    hits.push(format!(
                        "{}:{}: {pat}",
                        f.strip_prefix(root()).unwrap().display(),
                        i + 1
                    ));
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
fn only_serde_and_workspace_crates_are_dependencies() {
    let base = [
        "serde",
        "serde_json",
        "homei-core",
        "homei-catalog",
        "homei-bootstrap",
    ];
    // Each crate has its own explicit list. The portable crates take no cryptography and no
    // host-access crate. The host controller and the test crate take exactly what they need.
    let lab = ["serde", "serde_json", "homei-core", "sha2", "ed25519-dalek"];
    let tests = [
        "serde",
        "serde_json",
        "homei-core",
        "homei-catalog",
        "homei-bootstrap",
        "homei-lab",
        "ed25519-dalek",
    ];
    let crates: [(&str, &[&str]); 6] = [
        ("Core", &base),
        ("Catalog", &base),
        ("Bootstrap", &base),
        ("Tools/homei-cli", &base),
        ("Lab", &lab),
        ("Tests", &tests),
    ];
    for (m, allowed) in crates {
        let text = fs::read_to_string(root().join(m).join("Cargo.toml")).unwrap();
        let mut in_deps = false;
        for line in text.lines() {
            let l = line.trim();
            if l.starts_with('[') {
                in_deps = l == "[dependencies]"
                    || l == "[dev-dependencies]"
                    || l == "[build-dependencies]";
                continue;
            }
            if in_deps && !l.is_empty() && !l.starts_with('#') {
                let name = l.split(['=', '.', ' ']).next().unwrap();
                assert!(allowed.contains(&name), "{m}: unexpected dependency {name}");
            }
        }
    }
}

#[test]
fn product_crates_have_no_build_scripts() {
    for m in [
        "Core",
        "Catalog",
        "Bootstrap",
        "Tools/homei-cli",
        "Lab",
        "Tests",
    ] {
        assert!(
            !root().join(m).join("build.rs").exists(),
            "{m} has a build script"
        );
    }
}

#[test]
fn data_files_never_label_anything_exploitable() {
    for f in [
        "evidence.json",
        "compatibility.json",
        "research-index.json",
        "device-fixtures.json",
    ] {
        let text = fs::read_to_string(root().join("Data").join(f))
            .unwrap()
            .to_lowercase();
        assert!(!text.contains("\"exploitable\""), "{f}");
        assert!(!text.contains("exploitable:"), "{f}");
    }
}
