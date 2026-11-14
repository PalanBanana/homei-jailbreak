//! Read-only analysis helpers on temporary fixtures: provenance hashing and Ghidra metadata.
//! The fixtures are written to a temp directory by the test itself, never by the library.

#[path = "support.rs"]
mod support;

use homei_lab::analysis::ghidra_meta;
use homei_lab::analysis::provenance::record_input;
use homei_lab::hash::sha256_hex;
use std::fs;
use support::TempDir;

#[test]
fn provenance_records_name_size_and_digest_without_a_directory() {
    let dir = TempDir::new("provenance");
    let file = dir.0.join("sample.bin");
    fs::write(&file, b"abc").unwrap();
    let rec = record_input("test", &file).unwrap();
    assert_eq!(rec.file_name, "sample.bin");
    assert_eq!(rec.bytes, 3);
    assert_eq!(rec.sha256, sha256_hex(b"abc"));
    let json = serde_json::to_string(&rec).unwrap();
    assert!(
        !json.contains(dir.0.to_string_lossy().as_ref()),
        "no directory is recorded"
    );
}

#[test]
fn ghidra_names_are_found_across_chunk_boundaries_and_counts_stay_unknown() {
    let dir = TempDir::new("ghidra");
    let rep = dir.0.join("p.rep").join("idata");
    fs::create_dir_all(&rep).unwrap();
    fs::write(dir.0.join("p.gpr"), b"").unwrap();
    let mut data = vec![0u8; (8 << 20) - 4];
    data.extend(b"AppleT8130PMGR");
    fs::write(rep.join("00"), &data).unwrap();
    let r =
        ghidra_meta::inspect(&dir.0.join("p.gpr"), &["AppleT8130PMGR", "missing-name"]).unwrap();
    assert_eq!(r.project_file_state, "empty");
    assert!(r.repository_present);
    assert_eq!(r.index_files, 1);
    assert!(
        r.names[0].present_in_index,
        "a name straddling the chunk edge must be found"
    );
    assert!(!r.names[1].present_in_index);
    assert_eq!(r.function_count, None, "no export means no function count");
    assert!(!r.export_present);
}
