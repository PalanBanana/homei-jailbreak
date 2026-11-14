//! Guest storage: base image immutability, overlay, snapshots, corruption and confirmation rules.

#[path = "support.rs"]
mod support;

use homei_lab::guard::*;
use homei_lab::hash::{sha256_file, sha256_hex};
use homei_lab::storage::*;
use support::*;

fn ws(tag: &str) -> (TempDir, Workspace) {
    let t = TempDir::new(tag);
    let w = Workspace::init(&t.0.join("ws"), NOW).unwrap();
    (t, w)
}

#[test]
fn guest_paths_reject_traversal_absolute_paths_and_unsafe_characters() {
    for bad in [
        "", "/etc", "..", "a/..", "a/../b", "./a", "a/./b", "a//b", "a\\b", "a\0b", "c:x", "a/",
    ] {
        assert!(components(bad).is_err(), "{bad:?}");
    }
    assert_eq!(
        components("opt/homei-lab/bin").unwrap(),
        ["opt", "homei-lab", "bin"]
    );
}

#[test]
fn the_guard_refuses_a_symlink_anywhere_on_the_path() {
    let t = TempDir::new("guard");
    let root = GuestRoot::create(&t.0.join("r")).unwrap();
    std::fs::create_dir_all(t.0.join("outside")).unwrap();
    std::os::unix::fs::symlink(t.0.join("outside"), t.0.join("r/link")).unwrap();
    assert!(matches!(
        root.write("link/file", b"x"),
        Err(GuardError::Symlink(_))
    ));
    assert!(matches!(root.resolve("link"), Err(GuardError::Symlink(_))));
    assert!(!t.0.join("outside/file").exists());
    assert!(GuestRoot::open(&t.0.join("missing")).is_err());
}

#[test]
fn the_base_image_is_registered_by_hash_and_never_modified() {
    let (t, mut w) = ws("base");
    let base = t.0.join("base.img");
    std::fs::write(&base, b"original firmware image").unwrap();
    let before = sha256_file(&base).unwrap();
    let mtime = std::fs::metadata(&base).unwrap().modified().unwrap();
    let reg = w.register_base(&base, "base", NOW).unwrap();
    assert_eq!(reg.sha256, before);
    w.overlay().write("opt/x", b"guest change").unwrap();
    w.snapshot("s", NOW).unwrap();
    w.discard_overlay(true).unwrap();
    assert_eq!(
        sha256_file(&base).unwrap(),
        before,
        "base must be byte-identical after guest activity"
    );
    assert_eq!(
        std::fs::metadata(&base).unwrap().modified().unwrap(),
        mtime,
        "base must not even be touched"
    );
    assert_eq!(w.verify_base(&base), Ok(()));
}

#[test]
fn a_changed_or_missing_base_is_detected() {
    let (t, mut w) = ws("basechg");
    let base = t.0.join("base.img");
    assert_eq!(w.verify_base(&base), Err(StorageError::NoBaseRegistered));
    std::fs::write(&base, b"v1").unwrap();
    w.register_base(&base, "base", NOW).unwrap();
    std::fs::write(&base, b"v2").unwrap();
    assert!(matches!(
        w.verify_base(&base),
        Err(StorageError::BaseChanged { .. })
    ));
    std::fs::remove_file(&base).unwrap();
    assert!(w.verify_base(&base).is_err());
}

#[test]
fn a_fresh_overlay_is_empty_and_writes_stay_inside_it() {
    let (t, w) = ws("overlay");
    assert!(w.overlay_files().unwrap().is_empty());
    w.overlay().write("opt/homei-lab/a.txt", b"a").unwrap();
    assert_eq!(w.overlay_files().unwrap().len(), 1);
    assert!(w.overlay().write("../escape", b"x").is_err());
    assert!(!t.0.join("ws/escape").exists());
    assert_eq!(w.usage().unwrap().overlay_bytes, 1);
}

#[test]
fn a_snapshot_records_hashes_and_restore_returns_exactly_that_state() {
    let (_t, w) = ws("snap");
    w.overlay().write("opt/one", b"1").unwrap();
    w.overlay().write("opt/dir/two", b"22").unwrap();
    let s = w.snapshot("before", NOW).unwrap();
    assert_eq!(s.id, "snap-0001");
    assert_eq!(s.files.len(), 2);
    assert_eq!(s.files.iter().map(|f| f.bytes).sum::<u64>(), 3);
    let expected = w.overlay_tree_hash().unwrap();
    assert_eq!(s.tree_sha256, expected);
    w.overlay().write("opt/one", b"changed").unwrap();
    w.overlay().write("opt/three", b"3").unwrap();
    w.overlay().remove_file("opt/dir/two").unwrap();
    assert!(w.verify_overlay(&expected).is_err());
    w.restore("snap-0001").unwrap();
    assert_eq!(w.overlay_tree_hash().unwrap(), expected);
    assert_eq!(w.overlay().read("opt/one").unwrap(), b"1");
    assert!(!w.overlay().exists("opt/three"));
    assert_eq!(w.overlay().read("opt/dir/two").unwrap(), b"22");
}

#[test]
fn snapshot_ids_increase_and_listing_is_ordered() {
    let (_t, w) = ws("ids");
    w.overlay().write("a", b"1").unwrap();
    for label in ["first", "second", "third"] {
        w.snapshot(label, NOW).unwrap();
    }
    let ids: Vec<_> = w
        .list_snapshots()
        .unwrap()
        .into_iter()
        .map(|s| (s.id, s.label))
        .collect();
    assert_eq!(
        ids,
        [
            ("snap-0001".into(), "first".into()),
            ("snap-0002".into(), "second".into()),
            ("snap-0003".into(), "third".into())
        ]
    );
    assert_eq!(w.usage().unwrap().snapshots, 3);
}

#[test]
fn a_corrupted_snapshot_is_refused_and_the_overlay_is_left_untouched() {
    let (t, w) = ws("corrupt");
    w.overlay().write("opt/file", b"original").unwrap();
    w.snapshot("s", NOW).unwrap();
    w.overlay().write("opt/file", b"current work").unwrap();
    std::fs::write(
        t.0.join("ws/snapshots/snap-0001/files/opt/file"),
        b"tampered",
    )
    .unwrap();
    assert!(matches!(
        w.verify_snapshot("snap-0001"),
        Err(StorageError::SnapshotCorrupted { .. })
    ));
    assert!(matches!(
        w.restore("snap-0001"),
        Err(StorageError::SnapshotCorrupted { .. })
    ));
    assert_eq!(
        w.overlay().read("opt/file").unwrap(),
        b"current work",
        "a refused restore must not change the overlay"
    );
    assert!(matches!(
        w.restore("snap-0099"),
        Err(StorageError::SnapshotNotFound(_))
    ));
}

#[test]
fn a_tampered_manifest_is_detected() {
    let (t, w) = ws("manifest");
    w.overlay().write("a", b"1").unwrap();
    w.snapshot("s", NOW).unwrap();
    let path = t.0.join("ws/snapshots/snap-0001/manifest.json");
    let text = std::fs::read_to_string(&path).unwrap();
    let altered = text.replace(&sha256_hex(b"1"), &sha256_hex(b"2"));
    std::fs::write(&path, altered).unwrap();
    assert!(w.verify_snapshot("snap-0001").is_err());
}

#[test]
fn nothing_is_deleted_without_explicit_confirmation() {
    let (_t, w) = ws("confirm");
    w.overlay().write("a", b"1").unwrap();
    w.snapshot("s", NOW).unwrap();
    assert_eq!(
        w.delete_snapshot("snap-0001", false),
        Err(StorageError::ConfirmationRequired("deleting a snapshot"))
    );
    assert_eq!(
        w.discard_overlay(false),
        Err(StorageError::ConfirmationRequired("discarding the overlay"))
    );
    assert_eq!(w.list_snapshots().unwrap().len(), 1);
    assert!(w.overlay().exists("a"));
    w.delete_snapshot("snap-0001", true).unwrap();
    w.discard_overlay(true).unwrap();
    assert!(w.list_snapshots().unwrap().is_empty());
    assert!(w.overlay_files().unwrap().is_empty());
}

#[test]
fn size_limits_stop_a_snapshot_before_anything_is_written() {
    let (_t, mut w) = ws("limits");
    w.set_limits(Limits {
        max_overlay_bytes: 10,
        max_snapshot_total_bytes: 100,
    })
    .unwrap();
    w.overlay().write("big", &[0u8; 50]).unwrap();
    assert!(matches!(
        w.snapshot("s", NOW),
        Err(StorageError::LimitExceeded {
            what: "overlay",
            ..
        })
    ));
    assert!(w.list_snapshots().unwrap().is_empty());
    w.set_limits(Limits {
        max_overlay_bytes: 1000,
        max_snapshot_total_bytes: 60,
    })
    .unwrap();
    w.snapshot("one", NOW).unwrap();
    assert!(matches!(
        w.snapshot("two", NOW),
        Err(StorageError::LimitExceeded {
            what: "snapshots",
            ..
        })
    ));
    assert_eq!(w.list_snapshots().unwrap().len(), 1);
}

#[test]
fn reopening_a_workspace_keeps_its_base_and_limits() {
    let t = TempDir::new("reopen");
    let base = t.0.join("b");
    std::fs::write(&base, b"x").unwrap();
    {
        let mut w = Workspace::init(&t.0.join("ws"), NOW).unwrap();
        w.register_base(&base, "b", NOW).unwrap();
        w.overlay().write("keep", b"1").unwrap();
    }
    let w = Workspace::init(&t.0.join("ws"), "2099-01-01T00:00:00Z").unwrap();
    assert_eq!(w.base().unwrap().name, "b");
    assert!(w.overlay().exists("keep"));
}
