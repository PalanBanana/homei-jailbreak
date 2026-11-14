//! Firmware artifact metadata and the read-only Ghidra inspector, on synthetic fixtures in a temp dir.

#[path = "support.rs"]
mod support;

use homei_lab::firmware::*;
use homei_lab::ghidra::*;
use homei_lab::hash::sha256_hex;
use support::TempDir;

fn manifest(entries: &[(&str, &str, &str)]) -> String {
    let rows: Vec<String> = entries
        .iter()
        .map(|(name, sha, fw)| format!(r#"{{"artifact":"{name}","path_relative":"x/{name}","bytes":10,"sha256":"{sha}","extraction_method":"test","source_firmware":"{fw}"}}"#))
        .collect();
    format!("[{}]", rows.join(","))
}

#[test]
fn artifact_metadata_is_parsed_and_filtered_to_the_target_build() {
    let h = sha256_hex(b"a");
    let json = manifest(&[
        ("kernelcache.release.iphone16", &h, "iPhone16,2 24A446"),
        ("kernelcache.release.iphone16", &h, "iPhone16,2 24B119"),
        ("sptm.t8122.release", &h, "iPhone16,2 24A446"),
    ]);
    let records = parse_artifact_hashes(&json, "24A446").unwrap();
    assert_eq!(records.len(), 2);
    assert!(records.iter().all(|r| r.source_firmware.contains("24A446")));
    let public = to_public(&records[0], "24A446");
    assert_eq!(
        (public.role.as_str(), public.source_build.as_str()),
        ("kernelcache", "24A446")
    );
    assert!(
        !serde_json::to_string(&public).unwrap().contains("x/"),
        "the relative path must not be published"
    );
}

#[test]
fn malformed_manifests_and_digests_are_rejected() {
    assert!(matches!(
        parse_artifact_hashes("not json", "24A446"),
        Err(FirmwareError::Parse(_))
    ));
    assert!(matches!(
        parse_artifact_hashes(&manifest(&[("a", "xyz", "24A446")]), "24A446"),
        Err(FirmwareError::BadDigest(_))
    ));
    assert!(matches!(
        parse_artifact_hashes(&manifest(&[("a", &"g".repeat(64), "24A446")]), "24A446"),
        Err(FirmwareError::BadDigest(_))
    ));
}

#[test]
fn a_record_for_another_build_is_a_build_mismatch() {
    let h = sha256_hex(b"a");
    let all =
        parse_artifact_hashes(&manifest(&[("k", &h, "iPhone16,2 24B119")]), "24B119").unwrap();
    assert!(matches!(
        require_build(&all[0], "24A446"),
        Err(FirmwareError::BuildMismatch { .. })
    ));
    assert_eq!(require_build(&all[0], "24B119"), Ok(()));
}

#[test]
fn a_local_file_is_checked_by_hash_and_a_missing_one_is_reported() {
    let t = TempDir::new("fw");
    let file = t.0.join("k");
    std::fs::write(&file, b"payload").unwrap();
    let rec = |sha: &str| {
        parse_artifact_hashes(&manifest(&[("k", sha, "24A446")]), "24A446")
            .unwrap()
            .remove(0)
    };
    assert_eq!(verify_local(&file, &rec(&sha256_hex(b"payload"))), Ok(()));
    assert!(matches!(
        verify_local(&file, &rec(&sha256_hex(b"other"))),
        Err(FirmwareError::HashMismatch { .. })
    ));
    assert!(matches!(
        verify_local(&t.0.join("absent"), &rec(&sha256_hex(b"payload"))),
        Err(FirmwareError::MissingBinary(_))
    ));
    assert_eq!(
        std::fs::read(&file).unwrap(),
        b"payload",
        "verification must not modify the file"
    );
}

fn project(t: &TempDir, name: &str) {
    std::fs::create_dir_all(t.0.join("proj")).unwrap();
    std::fs::write(t.0.join(format!("proj/{name}.gpr")), b"project").unwrap();
    std::fs::create_dir_all(t.0.join(format!("proj/{name}.rep"))).unwrap();
}

fn inspector(t: &TempDir) -> Inspector {
    Inspector {
        project_dir: t.0.join("proj"),
        project_name: "P".to_string(),
        analysis_dir: t.0.join("analysis"),
        exports_dir: t.0.join("exports"),
        logs_dir: t.0.join("logs"),
    }
}

#[test]
fn a_missing_project_is_reported_as_missing_and_nothing_is_invented() {
    let t = TempDir::new("noproj");
    let s = inspector(&t).inspect(Some("12.1.2".into()), "kc", "arm64e", None);
    assert!(!s.project_present);
    assert_eq!(s.import_status, "project not found");
    assert_eq!((s.function_count, s.fileset_entries), (0, 0));
    assert_eq!(s.confidence, "none");
}

#[test]
fn an_existing_project_is_detected_without_opening_it() {
    let t = TempDir::new("proj");
    project(&t, "P");
    let before = std::fs::read(t.0.join("proj/P.gpr")).unwrap();
    let s = inspector(&t).inspect(None, "kc", "arm64e", None);
    assert!(s.project_present);
    assert_eq!(
        s.import_status,
        "project present, no analysis metrics found"
    );
    assert_eq!(std::fs::read(t.0.join("proj/P.gpr")).unwrap(), before);
    std::fs::remove_dir_all(t.0.join("proj/P.rep")).unwrap();
    assert!(
        !inspector(&t).project_present(),
        "both the .gpr file and the .rep directory are required"
    );
}

#[test]
fn analysis_metrics_are_imported_as_numbers_and_the_hash_is_preserved() {
    let t = TempDir::new("metrics");
    project(&t, "P");
    std::fs::create_dir_all(t.0.join("analysis")).unwrap();
    std::fs::write(
        t.0.join("analysis/function-counts-by-fileset-entry.json"),
        r#"{"functions_total": 230298}"#,
    )
    .unwrap();
    std::fs::write(
        t.0.join("analysis/kernel-fileset-coverage.json"),
        r#"{"fileset_entries_total": 264, "fileset_entries_start_address_covered": 264}"#,
    )
    .unwrap();
    std::fs::write(
        t.0.join("analysis/kernel-symbolication-metrics.json"),
        format!(
            r#"{{"kernelcache_sha256": "{}", "ipsw_reported": {{"percent": 61.28}}}}"#,
            "ab".repeat(32)
        ),
    )
    .unwrap();
    let s = inspector(&t).inspect(
        Some("12.1.2".into()),
        "kernelcache.release.iPhone16,2",
        "AARCH64:LE:64:AppleSilicon",
        Some("2026-11-12".into()),
    );
    assert_eq!(
        (
            s.function_count,
            s.fileset_entries,
            s.fileset_entries_covered
        ),
        (230_298, 264, 264)
    );
    assert_eq!(s.symbol_match_basis_points, Some(6128));
    assert_eq!(s.firmware_sha256.as_deref(), Some("ab".repeat(32).as_str()));
    assert_eq!(
        s.import_status,
        "imported and analysed, no classified log issues"
    );
    assert_eq!(s.confidence, "static observation, medium");
    let json = serde_json::to_string(&s).unwrap();
    assert!(
        !json.contains(t.0.to_str().unwrap()),
        "no local path may appear in the summary"
    );
}

#[test]
fn import_errors_in_logs_are_classified_and_counted() {
    assert_eq!(
        classify_line("ERROR: No load spec found for file"),
        Some(ImportIssue::NoLoadSpec)
    );
    assert_eq!(
        classify_line("java.lang.OutOfMemoryError: Java heap space"),
        Some(ImportIssue::OutOfMemory)
    );
    assert_eq!(
        classify_line("Exception in thread main"),
        Some(ImportIssue::Exception)
    );
    assert_eq!(
        classify_line("Unable to resolve reference"),
        Some(ImportIssue::UnresolvedReference)
    );
    assert_eq!(
        classify_line("ERROR: something else"),
        Some(ImportIssue::Other)
    );
    assert_eq!(classify_line("INFO: analysis finished, 0 errors"), None);
    assert_eq!(classify_line("INFO: all good"), None);
    let counts = classify_log("INFO\nException a\nException b\nERROR: no load spec\n");
    assert!(
        counts.contains(&(ImportIssue::Exception, 2))
            && counts.contains(&(ImportIssue::NoLoadSpec, 1))
    );
}

#[test]
fn logged_issues_change_the_import_status_and_are_listed_by_file() {
    let t = TempDir::new("logs");
    project(&t, "P");
    std::fs::create_dir_all(t.0.join("analysis")).unwrap();
    std::fs::create_dir_all(t.0.join("logs")).unwrap();
    std::fs::write(
        t.0.join("analysis/function-counts-by-fileset-entry.json"),
        r#"{"functions_total": 5}"#,
    )
    .unwrap();
    std::fs::write(
        t.0.join("logs/ghidra-import-x.log"),
        "INFO ok\nException while importing\n",
    )
    .unwrap();
    std::fs::write(t.0.join("logs/unrelated.log"), "Exception\n").unwrap();
    let s = inspector(&t).inspect(None, "kc", "arm64e", None);
    assert_eq!(
        s.import_errors,
        vec!["ghidra-import-x.log: 1 x exception during import or analysis".to_string()]
    );
    assert!(s.import_status.contains("classified log lines"));
}

#[test]
fn the_inspector_never_modifies_what_it_reads() {
    let t = TempDir::new("ro");
    project(&t, "P");
    std::fs::create_dir_all(t.0.join("analysis")).unwrap();
    let f = t.0.join("analysis/function-counts-by-fileset-entry.json");
    std::fs::write(&f, r#"{"functions_total": 9}"#).unwrap();
    let before = (
        std::fs::read(&f).unwrap(),
        std::fs::metadata(&f).unwrap().modified().unwrap(),
    );
    inspector(&t).inspect(None, "kc", "arm64e", None);
    assert_eq!(
        (
            std::fs::read(&f).unwrap(),
            std::fs::metadata(&f).unwrap().modified().unwrap()
        ),
        before
    );
}

#[test]
fn a_record_without_a_relative_path_is_accepted() {
    let h = sha256_hex(b"package");
    let json = format!(
        r#"[{{"artifact":"IPSW package","path_relative":null,"bytes":10,"sha256":"{h}","sha384":null,"extraction_method":"test","source_firmware":"iPhone16,2 / 27.0.1 / 24A446"}}]"#
    );
    let records = parse_artifact_hashes(&json, "24A446").unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].path_relative, None);
}

#[test]
fn format_and_role_come_from_the_artifact_description() {
    let h = sha256_hex(b"x");
    let names = [
        ("BuildManifest.plist", "property list", "build manifest"),
        (
            "kernelcache.release.iphone16 (raw IM4P container)",
            "IM4P container",
            "kernelcache",
        ),
        (
            "kernelcache analysis artifact (Mach-O MH_FILESET)",
            "Mach-O fileset payload",
            "kernelcache",
        ),
        (
            "KEXT com.example.driver, imports resolved",
            "Mach-O extracted from the kernelcache",
            "kernel extension",
        ),
        (
            "IPSW iPhone16,2_27.0.1_24A446_Restore.ipsw (full file)",
            "zip package",
            "full firmware package",
        ),
        ("something.bin", "not classified", "firmware component"),
    ];
    for (name, format, role) in names {
        let json = manifest(&[(name, &h, "iPhone16,2 24A446")]);
        let r = &parse_artifact_hashes(&json, "24A446").unwrap()[0];
        let p = to_public(r, "24A446");
        assert_eq!(
            (p.format.as_str(), p.role.as_str()),
            (format, role),
            "{name}"
        );
    }
}
