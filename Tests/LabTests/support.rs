//! Shared helpers for the lab tests. Everything here is a test double or a test-only key.
//! The signing keys are derived from fixed bytes and are public on purpose: they sign nothing real.
#![allow(dead_code)]

use ed25519_dalek::{Signer, SigningKey};
use homei_lab::hash::{sha256_hex, to_hex};
use homei_lab::packages::{
    build_archive, index_entry_for, Dependency, IndexEntry, PackageFile, PackageHeader,
    PackageIndex, PkgError,
};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

static COUNTER: AtomicU32 = AtomicU32::new(0);

pub struct TempDir(pub PathBuf);

impl TempDir {
    pub fn new(tag: &str) -> TempDir {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let p =
            std::env::temp_dir().join(format!("homei-lab-test-{tag}-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        TempDir(p)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub const NOW: &str = "2026-11-14T00:00:00Z";

pub fn key(seed: u8) -> SigningKey {
    SigningKey::from_bytes(&[seed; 32])
}

pub fn pub_hex(k: &SigningKey) -> String {
    to_hex(&k.verifying_key().to_bytes())
}

pub fn header(
    id: &str,
    version: &str,
    deps: &[(&str, &str)],
    conflicts: &[&str],
    files: &[(&str, &[u8])],
) -> (PackageHeader, Vec<Vec<u8>>) {
    let h = PackageHeader {
        format: 1,
        id: id.to_string(),
        version: version.to_string(),
        description: format!("test package {id}"),
        license: "MIT".to_string(),
        source: "test fixture".to_string(),
        depends: deps
            .iter()
            .map(|(i, r)| Dependency {
                id: i.to_string(),
                requirement: r.to_string(),
            })
            .collect(),
        conflicts: conflicts.iter().map(|s| s.to_string()).collect(),
        scripts: Vec::new(),
        files: files
            .iter()
            .map(|(p, d)| PackageFile {
                path: p.to_string(),
                size: d.len() as u64,
                sha256: sha256_hex(d),
                executable: false,
            })
            .collect(),
    };
    (h, files.iter().map(|(_, d)| d.to_vec()).collect())
}

pub fn package(
    id: &str,
    version: &str,
    deps: &[(&str, &str)],
    conflicts: &[&str],
    files: &[(&str, &[u8])],
) -> Vec<u8> {
    let (h, p) = header(id, version, deps, conflicts, files);
    build_archive(&h, &p)
}

/// A small repository: the archives by name and an index. Each entry is signed by `signer` if given.
pub struct Repo {
    pub index: PackageIndex,
    pub archives: BTreeMap<String, Vec<u8>>,
}

impl Repo {
    pub fn new() -> Repo {
        Repo {
            index: PackageIndex {
                schema_version: 1,
                source_name: "test fixture repository".to_string(),
                packages: Vec::new(),
            },
            archives: BTreeMap::new(),
        }
    }

    pub fn add(&mut self, bytes: Vec<u8>, signer: Option<&SigningKey>) -> IndexEntry {
        let tmp = index_entry_for("x", &bytes, None, "test fixture").unwrap();
        let name = format!("{}-{}.hpk", tmp.id, tmp.version);
        let signature = signer.map(|k| {
            to_hex(
                &k.sign(&homei_lab::packages::signing_message(
                    &tmp.id,
                    &tmp.version,
                    &tmp.sha256,
                ))
                .to_bytes(),
            )
        });
        let entry = index_entry_for(&name, &bytes, signature, "test fixture").unwrap();
        self.archives.insert(name, bytes);
        self.index.packages.push(entry.clone());
        entry
    }

    pub fn fetch(&self) -> impl Fn(&IndexEntry) -> Result<Vec<u8>, PkgError> + '_ {
        move |e| {
            self.archives
                .get(&e.archive)
                .cloned()
                .ok_or_else(|| PkgError::NotInIndex(e.archive.clone()))
        }
    }
}
