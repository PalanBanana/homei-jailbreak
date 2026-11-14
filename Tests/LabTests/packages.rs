//! Package manager tests. They use test archives and test-only keys, and write only to a temporary
//! guest overlay. They prove properties of this code. They do not install anything on a device.

#[path = "support.rs"]
mod support;

use homei_lab::guard::GuestRoot;
use homei_lab::packages::*;
use support::*;

fn open(tag: &str) -> (TempDir, PackageManager) {
    let t = TempDir::new(tag);
    let root = GuestRoot::create(&t.0.join("overlay")).unwrap();
    (t, PackageManager::open(root).unwrap())
}

fn want(id: &str) -> Vec<Request> {
    vec![Request {
        id: id.to_string(),
        requirement: "*".to_string(),
    }]
}

fn trusted(k: &ed25519_dalek::SigningKey) -> TrustRoot {
    TrustRoot::empty().with_key_hex(&pub_hex(k)).unwrap()
}

#[test]
fn versions_compare_numerically_not_textually() {
    let v = |s: &str| Version::parse(s).unwrap();
    assert!(v("1.10") > v("1.9"));
    assert!(v("2") > v("1.99.99"));
    assert_eq!(v("1.0"), v("1.0.0"));
    for bad in ["", "a", "1..2", "1.2.x", "1 2", "-1"] {
        assert!(Version::parse(bad).is_err(), "{bad:?}");
    }
}

#[test]
fn requirements_are_evaluated_exactly() {
    let v = Version::parse("1.5").unwrap();
    for (req, ok) in [
        ("*", true),
        (">=1.0,<2.0", true),
        (">=1.6", false),
        ("<1.5", false),
        ("<=1.5", true),
        ("=1.5.0", true),
        (">1.5", false),
    ] {
        assert_eq!(satisfies(&v, req).unwrap(), ok, "{req}");
    }
    assert!(satisfies(&v, "~1.0").is_err());
}

#[test]
fn package_metadata_and_provenance_are_recorded_on_install() {
    let k = key(1);
    let mut repo = Repo::new();
    repo.add(
        package("hello", "1.0.0", &[], &[], &[("bin/hello", b"data")]),
        Some(&k),
    );
    let (_t, mut pm) = open("meta");
    let tx = pm
        .install(
            Approval::user_approved(),
            &repo.index,
            &want("hello"),
            &repo.fetch(),
            &trusted(&k),
            InstallOptions::default(),
            NOW,
        )
        .unwrap();
    assert_eq!(tx.state, TxState::Committed);
    let rec = &pm.installed().unwrap()[0];
    assert_eq!((rec.id.as_str(), rec.version.as_str()), ("hello", "1.0.0"));
    assert_eq!(rec.signature, SignatureState::Valid);
    assert_eq!(rec.provenance, "test fixture");
    assert_eq!(rec.files.len(), 1);
    assert!(pm.root().exists("opt/homei-lab/packages/hello/bin/hello"));
    assert!(pm.verify_installed().unwrap().is_empty());
}

#[test]
fn dependencies_install_first_and_the_highest_satisfying_version_wins() {
    let k = key(1);
    let mut repo = Repo::new();
    repo.add(package("lib", "1.0.0", &[], &[], &[("a", b"1")]), Some(&k));
    repo.add(package("lib", "1.5.0", &[], &[], &[("a", b"2")]), Some(&k));
    repo.add(package("lib", "2.0.0", &[], &[], &[("a", b"3")]), Some(&k));
    repo.add(
        package(
            "app",
            "1.0.0",
            &[("lib", ">=1.0,<2.0")],
            &[],
            &[("b", b"x")],
        ),
        Some(&k),
    );
    let plan = resolve(&repo.index, &want("app"), &Default::default()).unwrap();
    let ids: Vec<_> = plan
        .iter()
        .map(|e| format!("{} {}", e.id, e.version))
        .collect();
    assert_eq!(ids, ["lib 1.5.0", "app 1.0.0"]);
}

#[test]
fn version_conflicts_missing_packages_declared_conflicts_and_cycles_are_errors() {
    let k = key(1);
    let mut repo = Repo::new();
    repo.add(package("lib", "1.0.0", &[], &[], &[("a", b"1")]), Some(&k));
    repo.add(
        package(
            "needs-new-lib",
            "1.0.0",
            &[("lib", ">=2.0")],
            &[],
            &[("b", b"x")],
        ),
        Some(&k),
    );
    repo.add(
        package(
            "needs-missing",
            "1.0.0",
            &[("ghost", "*")],
            &[],
            &[("b", b"x")],
        ),
        Some(&k),
    );
    repo.add(
        package("one", "1.0.0", &[], &["lib"], &[("c", b"x")]),
        Some(&k),
    );
    repo.add(
        package("loop-a", "1.0.0", &[("loop-b", "*")], &[], &[("d", b"x")]),
        Some(&k),
    );
    repo.add(
        package("loop-b", "1.0.0", &[("loop-a", "*")], &[], &[("d", b"x")]),
        Some(&k),
    );
    let none = Default::default();
    assert!(matches!(
        resolve(&repo.index, &want("needs-new-lib"), &none),
        Err(PkgError::NoVersionSatisfies { .. })
    ));
    assert!(matches!(
        resolve(&repo.index, &want("needs-missing"), &none),
        Err(PkgError::NotInIndex(_))
    ));
    let both = vec![
        Request {
            id: "one".into(),
            requirement: "*".into(),
        },
        Request {
            id: "lib".into(),
            requirement: "*".into(),
        },
    ];
    assert!(matches!(
        resolve(&repo.index, &both, &none),
        Err(PkgError::Conflict { .. })
    ));
    assert!(matches!(
        resolve(&repo.index, &want("loop-a"), &none),
        Err(PkgError::DependencyCycle(_))
    ));
}

#[test]
fn an_invalid_signature_is_refused_and_nothing_is_written() {
    let (good, bad) = (key(1), key(2));
    let mut repo = Repo::new();
    repo.add(
        package("hello", "1.0.0", &[], &[], &[("bin/hello", b"data")]),
        Some(&bad),
    );
    let (_t, mut pm) = open("badsig");
    let e = pm
        .install(
            Approval::user_approved(),
            &repo.index,
            &want("hello"),
            &repo.fetch(),
            &trusted(&good),
            InstallOptions::default(),
            NOW,
        )
        .unwrap_err();
    assert!(matches!(e, PkgError::InvalidSignature(_)), "{e:?}");
    assert!(pm.installed().unwrap().is_empty());
    assert!(
        pm.transactions().unwrap().is_empty(),
        "a refused archive must not even open a transaction"
    );
}

#[test]
fn a_signature_with_an_empty_trust_root_is_untrusted_and_garbled_signatures_fail() {
    let k = key(1);
    let mut repo = Repo::new();
    let mut e = repo.add(
        package("hello", "1.0.0", &[], &[], &[("a", b"1")]),
        Some(&k),
    );
    let bytes = repo.archives[&e.archive].clone();
    assert_eq!(
        verify_archive(&e, &bytes, &TrustRoot::empty(), false),
        Err(PkgError::UntrustedKey)
    );
    for garbage in ["", "zz", "00", &"0".repeat(128), &"f".repeat(128)] {
        e.signature = Some(garbage.to_string());
        assert!(
            verify_archive(&e, &bytes, &trusted(&k), false).is_err(),
            "{garbage:?}"
        );
    }
    assert!(TrustRoot::empty().with_key_hex("not a key").is_err());
}

#[test]
fn unsigned_packages_need_an_explicit_opt_in_and_are_marked() {
    let k = key(1);
    let mut repo = Repo::new();
    repo.add(package("hello", "1.0.0", &[], &[], &[("a", b"1")]), None);
    let (_t, mut pm) = open("unsigned");
    let refused = pm
        .install(
            Approval::user_approved(),
            &repo.index,
            &want("hello"),
            &repo.fetch(),
            &trusted(&k),
            InstallOptions::default(),
            NOW,
        )
        .unwrap_err();
    assert!(matches!(refused, PkgError::MissingSignature(_)));
    pm.install(
        Approval::user_approved(),
        &repo.index,
        &want("hello"),
        &repo.fetch(),
        &trusted(&k),
        InstallOptions {
            allow_unsigned: true,
            ..Default::default()
        },
        NOW,
    )
    .unwrap();
    assert_eq!(
        pm.installed().unwrap()[0].signature,
        SignatureState::UnsignedAccepted
    );
}

#[test]
fn a_tampered_archive_fails_the_integrity_check() {
    let k = key(1);
    let mut repo = Repo::new();
    let e = repo.add(
        package("hello", "1.0.0", &[], &[], &[("a", b"payload")]),
        Some(&k),
    );
    repo.archives.get_mut(&e.archive).unwrap().push(0);
    let (_t, mut pm) = open("tamper");
    let err = pm
        .install(
            Approval::user_approved(),
            &repo.index,
            &want("hello"),
            &repo.fetch(),
            &trusted(&k),
            InstallOptions::default(),
            NOW,
        )
        .unwrap_err();
    assert!(matches!(err, PkgError::IntegrityMismatch { .. }));
    assert!(pm.installed().unwrap().is_empty());
}

#[test]
fn invalid_archives_are_rejected() {
    let good = package("hello", "1.0.0", &[], &[], &[("a", b"payload")]);
    assert_eq!(
        parse_archive(b"not an archive at all"),
        Err(PkgError::BadMagic)
    );
    assert_eq!(parse_archive(&[]), Err(PkgError::BadMagic));
    let mut truncated = good.clone();
    truncated.truncate(truncated.len() - 3);
    assert!(parse_archive(&truncated).is_err());
    let mut changed = good.clone();
    let last = changed.len() - 1;
    changed[last] ^= 0xff;
    assert!(matches!(
        parse_archive(&changed),
        Err(PkgError::FileHashMismatch(_))
    ));
    let mut huge = good.clone();
    huge[5..13].copy_from_slice(&(u64::MAX).to_be_bytes());
    assert!(matches!(
        parse_archive(&huge),
        Err(PkgError::HeaderTooLarge(_))
    ));
    let mut extra = good;
    extra.push(1);
    assert!(matches!(
        parse_archive(&extra),
        Err(PkgError::PayloadSizeMismatch { .. })
    ));
}

#[test]
fn path_traversal_absolute_paths_and_symlink_tricks_are_refused() {
    for bad in [
        "../escape",
        "a/../../escape",
        "/etc/passwd",
        "a//b",
        "./a",
        "a/./b",
        "a\\b",
        "c:evil",
        "a\0b",
        "",
    ] {
        let (h, p) = header("evil", "1.0.0", &[], &[], &[(bad, b"x")]);
        let bytes = build_archive(&h, &p);
        assert!(
            matches!(parse_archive(&bytes), Err(PkgError::UnsafePath(_))),
            "{bad:?}"
        );
    }
    let (h, p) = header("dup", "1.0.0", &[], &[], &[("a", b"1"), ("a", b"2")]);
    assert!(matches!(
        parse_archive(&build_archive(&h, &p)),
        Err(PkgError::UnsafePath(_))
    ));
}

#[test]
fn a_package_cannot_write_through_a_symlink_that_points_outside_the_overlay() {
    let k = key(1);
    let mut repo = Repo::new();
    repo.add(
        package("hello", "1.0.0", &[], &[], &[("bin/hello", b"data")]),
        Some(&k),
    );
    let t = TempDir::new("symlink");
    let outside = t.0.join("outside");
    std::fs::create_dir_all(&outside).unwrap();
    let overlay = t.0.join("overlay");
    std::fs::create_dir_all(overlay.join("opt/homei-lab")).unwrap();
    std::os::unix::fs::symlink(&outside, overlay.join("opt/homei-lab/packages")).unwrap();
    let root = GuestRoot::open(&overlay).unwrap();
    let mut pm = PackageManager::open(root).unwrap();
    let r = pm.install(
        Approval::user_approved(),
        &repo.index,
        &want("hello"),
        &repo.fetch(),
        &trusted(&k),
        InstallOptions::default(),
        NOW,
    );
    assert!(r.is_err(), "a symlink on the way must stop the install");
    assert_eq!(
        std::fs::read_dir(&outside).unwrap().count(),
        0,
        "nothing may be written outside the overlay"
    );
}

#[test]
fn maintainer_scripts_are_rejected_and_never_run() {
    let (mut h, p) = header("scripty", "1.0.0", &[], &[], &[("a", b"1")]);
    h.scripts = vec!["postinst".to_string()];
    assert!(matches!(
        parse_archive(&build_archive(&h, &p)),
        Err(PkgError::MaintainerScript(_))
    ));
    for name in [
        "postinst",
        "preinst",
        "prerm",
        "postrm",
        "install.sh",
        "DEBIAN/postinst",
        "x/.scripts/run",
    ] {
        let (h, p) = header(
            "scripty",
            "1.0.0",
            &[],
            &[],
            &[(name, b"#!/bin/sh\ntouch /tmp/pwned\n")],
        );
        assert!(
            matches!(
                parse_archive(&build_archive(&h, &p)),
                Err(PkgError::MaintainerScript(_))
            ),
            "{name}"
        );
    }
    // The format has no field that lets an archive name a program to run.
    assert!(!std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../Lab/src/packages.rs"
    ))
    .unwrap()
    .contains("Command::new"));
}

#[test]
fn an_index_that_disagrees_with_the_archive_is_rejected() {
    let k = key(1);
    let mut repo = Repo::new();
    let mut e = repo.add(
        package("hello", "1.0.0", &[], &[], &[("a", b"1")]),
        Some(&k),
    );
    e.depends = vec![Dependency {
        id: "other".into(),
        requirement: "*".into(),
    }];
    repo.index.packages = vec![e];
    let (_t, mut pm) = open("indexmismatch");
    let err = pm.install(
        Approval::user_approved(),
        &repo.index,
        &want("hello"),
        &repo.fetch(),
        &trusted(&k),
        InstallOptions::default(),
        NOW,
    );
    assert!(err.is_err());
}

#[test]
fn removal_is_refused_while_another_package_needs_it() {
    let k = key(1);
    let mut repo = Repo::new();
    repo.add(package("lib", "1.0.0", &[], &[], &[("a", b"1")]), Some(&k));
    repo.add(
        package("app", "1.0.0", &[("lib", "*")], &[], &[("b", b"2")]),
        Some(&k),
    );
    let (_t, mut pm) = open("remove");
    pm.install(
        Approval::user_approved(),
        &repo.index,
        &want("app"),
        &repo.fetch(),
        &trusted(&k),
        InstallOptions::default(),
        NOW,
    )
    .unwrap();
    assert!(matches!(
        pm.remove(Approval::user_approved(), "lib", NOW),
        Err(PkgError::RequiredBy { .. })
    ));
    pm.remove(Approval::user_approved(), "app", NOW).unwrap();
    pm.remove(Approval::user_approved(), "lib", NOW).unwrap();
    assert!(pm.installed().unwrap().is_empty());
    assert!(!pm.root().exists("opt/homei-lab/packages/lib"));
    assert!(matches!(
        pm.remove(Approval::user_approved(), "lib", NOW),
        Err(PkgError::NotInstalled(_))
    ));
}

#[test]
fn a_transaction_rolls_back_an_install_and_an_upgrade() {
    let k = key(1);
    let mut repo = Repo::new();
    repo.add(
        package("tool", "1.0.0", &[], &[], &[("bin/tool", b"v1")]),
        Some(&k),
    );
    repo.add(
        package(
            "tool",
            "2.0.0",
            &[],
            &[],
            &[("bin/tool", b"v2"), ("bin/extra", b"e")],
        ),
        Some(&k),
    );
    let (_t, mut pm) = open("rollback");
    let r1 = vec![Request {
        id: "tool".into(),
        requirement: "=1.0.0".into(),
    }];
    let r2 = vec![Request {
        id: "tool".into(),
        requirement: "=2.0.0".into(),
    }];
    pm.install(
        Approval::user_approved(),
        &repo.index,
        &r1,
        &repo.fetch(),
        &trusted(&k),
        InstallOptions::default(),
        NOW,
    )
    .unwrap();
    pm.install(
        Approval::user_approved(),
        &repo.index,
        &r2,
        &repo.fetch(),
        &trusted(&k),
        InstallOptions::default(),
        NOW,
    )
    .unwrap();
    assert_eq!(pm.installed().unwrap()[0].version, "2.0.0");
    pm.rollback_last(Approval::user_approved()).unwrap();
    let rec = &pm.installed().unwrap()[0];
    assert_eq!(rec.version, "1.0.0", "the upgrade is undone");
    assert_eq!(
        pm.root()
            .read("opt/homei-lab/packages/tool/bin/tool")
            .unwrap(),
        b"v1"
    );
    assert!(!pm.root().exists("opt/homei-lab/packages/tool/bin/extra"));
    assert!(pm.verify_installed().unwrap().is_empty());
    pm.rollback_last(Approval::user_approved()).unwrap();
    assert!(
        pm.installed().unwrap().is_empty(),
        "the first install is undone too"
    );
    assert_eq!(
        pm.rollback_last(Approval::user_approved()),
        Err(PkgError::NothingToRollBack)
    );
}

#[test]
fn an_interrupted_install_rolls_back_and_leaves_no_partial_state() {
    let k = key(1);
    let mut repo = Repo::new();
    repo.add(package("lib", "1.0.0", &[], &[], &[("a", b"1")]), Some(&k));
    repo.add(
        package("app", "1.0.0", &[("lib", "*")], &[], &[("b", b"2")]),
        Some(&k),
    );
    let (_t, mut pm) = open("interrupt");
    let err = pm
        .install(
            Approval::user_approved(),
            &repo.index,
            &want("app"),
            &repo.fetch(),
            &trusted(&k),
            InstallOptions {
                abort_after_packages: Some(1),
                ..Default::default()
            },
            NOW,
        )
        .unwrap_err();
    assert_eq!(err, PkgError::Aborted);
    assert!(
        pm.installed().unwrap().is_empty(),
        "no package may remain after an interruption"
    );
    assert!(!pm.root().exists("opt/homei-lab/packages/lib"));
    let tx = &pm.transactions().unwrap()[0];
    assert_eq!(tx.state, TxState::RolledBack);
}

#[test]
fn a_transaction_left_in_progress_is_recovered_when_the_manager_is_reopened() {
    let k = key(1);
    let mut repo = Repo::new();
    repo.add(
        package("tool", "1.0.0", &[], &[], &[("bin/tool", b"v1")]),
        Some(&k),
    );
    let t = TempDir::new("recover");
    let overlay = t.0.join("overlay");
    {
        let mut pm = PackageManager::open(GuestRoot::create(&overlay).unwrap()).unwrap();
        pm.install(
            Approval::user_approved(),
            &repo.index,
            &want("tool"),
            &repo.fetch(),
            &trusted(&k),
            InstallOptions::default(),
            NOW,
        )
        .unwrap();
    }
    // Simulate a crash: the transaction is marked in progress, as if the process died mid-install.
    let root = GuestRoot::open(&overlay).unwrap();
    let rel = "opt/homei-lab/manifests/transactions/tx-0001.json";
    let mut tx: Transaction = serde_json::from_slice(&root.read(rel).unwrap()).unwrap();
    tx.state = TxState::InProgress;
    root.write(rel, &serde_json::to_vec(&tx).unwrap()).unwrap();
    root.mkdir_all("opt/homei-lab/.trash/tx-0001").unwrap();
    let pm = PackageManager::open(GuestRoot::open(&overlay).unwrap()).unwrap();
    assert_eq!(pm.recovered, vec!["tx-0001".to_string()]);
    assert!(pm.installed().unwrap().is_empty());
    assert_eq!(pm.transactions().unwrap()[0].state, TxState::RolledBack);
}

#[test]
fn every_write_stays_inside_the_guest_overlay() {
    let k = key(1);
    let mut repo = Repo::new();
    repo.add(
        package("lib", "1.0.0", &[], &[], &[("lib/a.dat", b"1")]),
        Some(&k),
    );
    repo.add(
        package("app", "1.0.0", &[("lib", "*")], &[], &[("bin/app", b"2")]),
        Some(&k),
    );
    let t = TempDir::new("scope");
    let before: Vec<_> = std::fs::read_dir(&t.0)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    let overlay = t.0.join("overlay");
    let mut pm = PackageManager::open(GuestRoot::create(&overlay).unwrap()).unwrap();
    pm.install(
        Approval::user_approved(),
        &repo.index,
        &want("app"),
        &repo.fetch(),
        &trusted(&k),
        InstallOptions::default(),
        NOW,
    )
    .unwrap();
    let mut after: Vec<_> = std::fs::read_dir(&t.0)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    after.sort();
    let mut expected: Vec<_> = before
        .into_iter()
        .chain([std::ffi::OsString::from("overlay")])
        .collect();
    expected.sort();
    assert_eq!(
        after, expected,
        "only the overlay directory may have been created"
    );
    for f in pm.root().list_all().unwrap() {
        assert!(
            f.starts_with("opt/homei-lab/"),
            "{f} is outside the research directory"
        );
    }
}

#[test]
fn the_exported_installation_manifest_has_no_host_path_and_names_the_scope() {
    let k = key(1);
    let mut repo = Repo::new();
    repo.add(
        package("hello", "1.0.0", &[], &[], &[("a", b"1")]),
        Some(&k),
    );
    let (t, mut pm) = open("manifest");
    pm.install(
        Approval::user_approved(),
        &repo.index,
        &want("hello"),
        &repo.fetch(),
        &trusted(&k),
        InstallOptions::default(),
        NOW,
    )
    .unwrap();
    let json = serde_json::to_string(&pm.export_manifest().unwrap()).unwrap();
    assert!(json.contains("guest overlay (not a device)"));
    assert!(!json.contains(t.0.to_str().unwrap()));
    assert!(!json.contains("/home/"));
}
