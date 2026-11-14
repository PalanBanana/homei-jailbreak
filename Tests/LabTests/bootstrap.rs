//! VM-only research bootstrap. It provisions a guest overlay and nothing else.

#[path = "support.rs"]
mod support;

use homei_lab::bootstrap::*;
use homei_lab::guard::GuestRoot;
use homei_lab::hash::sha256_file;
use homei_lab::packages::*;
use homei_lab::storage::Workspace;
use support::*;

fn overlay(tag: &str) -> (TempDir, GuestRoot) {
    let t = TempDir::new(tag);
    let r = GuestRoot::create(&t.0.join("overlay")).unwrap();
    (t, r)
}

fn job<'a>(
    repo: &'a Repo,
    trust: &'a TrustRoot,
    fetch: &'a dyn Fn(&IndexEntry) -> Result<Vec<u8>, PkgError>,
    id: &str,
) -> PackageJob<'a> {
    PackageJob {
        index: &repo.index,
        requests: vec![Request {
            id: id.into(),
            requirement: "*".into(),
        }],
        fetch,
        trust,
        options: InstallOptions::default(),
        approval: Approval::user_approved(),
    }
}

#[test]
fn a_fresh_overlay_gets_the_documented_layout_and_a_versioned_manifest() {
    let (_t, root) = overlay("fresh");
    let r = run(&root, NOW, None);
    assert_eq!(r.outcome, Outcome::Fresh);
    assert_eq!(r.error, None);
    for d in [
        "bin",
        "packages",
        "profiles",
        "analysis",
        "logs",
        "manifests",
        "workspace",
    ] {
        assert!(root.exists(&format!("opt/homei-lab/{d}")), "{d}");
    }
    let m = state(&root).unwrap();
    assert_eq!(m.bootstrap_version, "1");
    assert_eq!(m.state, "complete");
    assert!(m.filesystem_scope.contains("not a device"));
    assert!(
        String::from_utf8(root.read("opt/homei-lab/profiles/default.json").unwrap())
            .unwrap()
            .contains("Guest root is not physical device root")
    );
    assert!(
        String::from_utf8(root.read("opt/homei-lab/logs/bootstrap.log").unwrap())
            .unwrap()
            .contains("Fresh")
    );
}

#[test]
fn repeating_the_bootstrap_changes_nothing() {
    let (_t, root) = overlay("idem");
    run(&root, NOW, None);
    let before = root
        .list_all()
        .unwrap()
        .into_iter()
        .filter(|f| !f.ends_with("bootstrap.log"))
        .map(|f| (root.read(&f).unwrap(), f))
        .collect::<Vec<_>>();
    let again = run(&root, "2026-11-15T00:00:00Z", None);
    assert_eq!(again.outcome, Outcome::Unchanged);
    assert!(again.created.is_empty());
    let after = root
        .list_all()
        .unwrap()
        .into_iter()
        .filter(|f| !f.ends_with("bootstrap.log"))
        .map(|f| (root.read(&f).unwrap(), f))
        .collect::<Vec<_>>();
    assert_eq!(
        before, after,
        "an idempotent bootstrap must leave every file byte-identical"
    );
    let log = String::from_utf8(root.read("opt/homei-lab/logs/bootstrap.log").unwrap()).unwrap();
    assert_eq!(log.lines().count(), 2, "every run is logged");
}

#[test]
fn a_package_step_installs_through_the_same_guarded_manager() {
    let k = key(1);
    let mut repo = Repo::new();
    repo.add(
        package("probe", "1.0.0", &[], &[], &[("bin/probe", b"tool")]),
        Some(&k),
    );
    let trust = TrustRoot::empty().with_key_hex(&pub_hex(&k)).unwrap();
    let (_t, root) = overlay("pkgok");
    let fetch = repo.fetch();
    let r = run(&root, NOW, Some(job(&repo, &trust, &fetch, "probe")));
    assert_eq!(r.outcome, Outcome::Fresh);
    assert_eq!(r.package_transaction.as_deref(), Some("tx-0001"));
    assert_eq!(r.manifest.packages_requested, ["probe"]);
    assert!(root.exists("opt/homei-lab/packages/probe/bin/probe"));
    assert_eq!(
        PackageManager::open(root)
            .unwrap()
            .installed()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn a_failed_installation_is_reported_as_failed_and_leaves_no_package_behind() {
    let (good, bad) = (key(1), key(2));
    let mut repo = Repo::new();
    repo.add(
        package("probe", "1.0.0", &[], &[], &[("bin/probe", b"tool")]),
        Some(&bad),
    );
    let trust = TrustRoot::empty().with_key_hex(&pub_hex(&good)).unwrap();
    let (_t, root) = overlay("pkgfail");
    let fetch = repo.fetch();
    let r = run(&root, NOW, Some(job(&repo, &trust, &fetch, "probe")));
    assert_eq!(
        r.outcome,
        Outcome::Failed,
        "a failed step must never be reported as success"
    );
    assert!(r.error.as_deref().unwrap().contains("package step failed"));
    assert_eq!(r.package_transaction, None);
    assert_eq!(
        state(&root).unwrap().state,
        "layout_only",
        "the guest records that the package step did not apply"
    );
    let pm = PackageManager::open(root.clone()).unwrap();
    assert!(pm.installed().unwrap().is_empty());
    assert!(!root.exists("opt/homei-lab/packages/probe"));
    assert!(
        String::from_utf8(root.read("opt/homei-lab/logs/bootstrap.log").unwrap())
            .unwrap()
            .contains("FAILED")
    );
}

#[test]
fn an_interrupted_transaction_inside_a_bootstrap_rolls_back() {
    let k = key(1);
    let mut repo = Repo::new();
    repo.add(package("lib", "1.0.0", &[], &[], &[("a", b"1")]), Some(&k));
    repo.add(
        package("app", "1.0.0", &[("lib", "*")], &[], &[("b", b"2")]),
        Some(&k),
    );
    let trust = TrustRoot::empty().with_key_hex(&pub_hex(&k)).unwrap();
    let (_t, root) = overlay("pkgint");
    let fetch = repo.fetch();
    let mut j = job(&repo, &trust, &fetch, "app");
    j.options.abort_after_packages = Some(1);
    let r = run(&root, NOW, Some(j));
    assert_eq!(r.outcome, Outcome::Failed);
    assert!(PackageManager::open(root)
        .unwrap()
        .installed()
        .unwrap()
        .is_empty());
}

#[test]
fn bootstrap_is_reversible_by_discarding_the_overlay_and_never_touches_the_base() {
    let t = TempDir::new("reverse");
    let base = t.0.join("base.img");
    std::fs::write(&base, b"immutable base image").unwrap();
    let before = sha256_file(&base).unwrap();
    let mut w = Workspace::init(&t.0.join("ws"), NOW).unwrap();
    w.register_base(&base, "base", NOW).unwrap();
    let r = run(w.overlay(), NOW, None);
    assert_eq!(r.outcome, Outcome::Fresh);
    assert!(!w.overlay_files().unwrap().is_empty());
    w.discard_overlay(true).unwrap();
    assert!(
        w.overlay_files().unwrap().is_empty(),
        "discarding the overlay returns the guest to the base state"
    );
    assert_eq!(sha256_file(&base).unwrap(), before);
    assert_eq!(
        run(w.overlay(), NOW, None).outcome,
        Outcome::Fresh,
        "and the bootstrap can be applied again"
    );
}

#[test]
fn bootstrap_only_writes_below_the_research_directory() {
    let (_t, root) = overlay("scope");
    run(&root, NOW, None);
    for f in root.list_all().unwrap() {
        assert!(f.starts_with("opt/homei-lab/"), "{f}");
    }
}

#[test]
fn an_unreported_state_is_not_invented() {
    let (_t, root) = overlay("nostate");
    assert!(
        state(&root).is_none(),
        "before a bootstrap there is no recorded state"
    );
}

#[test]
fn a_repeat_run_without_a_package_step_is_unchanged_after_an_install() {
    let k = key(1);
    let mut repo = Repo::new();
    repo.add(
        package("probe", "1.0.0", &[], &[], &[("bin/probe", b"tool")]),
        Some(&k),
    );
    let trust = TrustRoot::empty().with_key_hex(&pub_hex(&k)).unwrap();
    let (_t, root) = overlay("repeat-after-install");
    let fetch = repo.fetch();
    run(&root, NOW, Some(job(&repo, &trust, &fetch, "probe")));
    let again = run(&root, "2026-11-15T00:00:00Z", None);
    assert_eq!(again.outcome, Outcome::Unchanged);
    assert_eq!(again.manifest.packages_requested, ["probe"]);
}
