//! Isolated guest storage: a registered base image, a writable overlay and checksummed snapshots.
//!
//! - The base image is registered by hash and only ever read to verify that hash. It is never
//!   opened for writing, so a guest failure cannot corrupt it.
//! - Everything writable is the overlay, a `GuestRoot`. Discarding it returns to the base state.
//! - A snapshot is a copy of the overlay with a manifest of file hashes. Restoring verifies the
//!   copy first, so a corrupted snapshot is refused and nothing is half restored.
//! - Nothing is deleted without an explicit `confirm` argument.

use crate::guard::{GuardError, GuestRoot};
use crate::hash::{sha256_file, sha256_hex};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageError {
    Guard(GuardError),
    Io(String),
    NoBaseRegistered,
    BaseChanged {
        expected: String,
        found: String,
    },
    SnapshotNotFound(String),
    SnapshotCorrupted {
        id: String,
        path: String,
    },
    OverlayCorrupted {
        expected: String,
        found: String,
    },
    ConfirmationRequired(&'static str),
    LimitExceeded {
        what: &'static str,
        limit: u64,
        would_be: u64,
    },
    Parse(String),
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::Guard(e) => write!(f, "{e}"),
            StorageError::Io(e) => write!(f, "io error: {e}"),
            StorageError::NoBaseRegistered => write!(f, "no base image is registered"),
            StorageError::BaseChanged { expected, found } => {
                write!(f, "base image changed: expected {expected}, found {found}")
            }
            StorageError::SnapshotNotFound(id) => write!(f, "snapshot {id} not found"),
            StorageError::SnapshotCorrupted { id, path } => {
                write!(f, "snapshot {id} is corrupted at {path}")
            }
            StorageError::OverlayCorrupted { expected, found } => write!(
                f,
                "overlay does not match: expected {expected}, found {found}"
            ),
            StorageError::ConfirmationRequired(what) => {
                write!(f, "{what} needs explicit confirmation")
            }
            StorageError::LimitExceeded {
                what,
                limit,
                would_be,
            } => write!(f, "{what} limit {limit} bytes exceeded ({would_be})"),
            StorageError::Parse(e) => write!(f, "could not parse stored metadata: {e}"),
        }
    }
}

impl std::error::Error for StorageError {}

impl From<GuardError> for StorageError {
    fn from(e: GuardError) -> Self {
        StorageError::Guard(e)
    }
}

impl From<std::io::Error> for StorageError {
    fn from(e: std::io::Error) -> Self {
        StorageError::Io(e.kind().to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaseImage {
    pub name: String,
    pub sha256: String,
    pub bytes: u64,
    pub registered_on: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Limits {
    pub max_overlay_bytes: u64,
    pub max_snapshot_total_bytes: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            max_overlay_bytes: 2 << 30,
            max_snapshot_total_bytes: 8 << 30,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: String,
    pub sha256: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotManifest {
    pub id: String,
    pub label: String,
    pub created_on: String,
    pub files: Vec<FileEntry>,
    pub tree_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Usage {
    pub overlay_bytes: u64,
    pub snapshot_bytes: u64,
    pub snapshots: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct WorkspaceMeta {
    schema_version: u32,
    created_on: String,
    base: Option<BaseImage>,
    limits: Limits,
}

pub struct Workspace {
    root: GuestRoot,
    overlay: GuestRoot,
    snaps: GuestRoot,
    meta: WorkspaceMeta,
}

/// Hash of a file listing, stable under ordering.
pub fn tree_hash(files: &[FileEntry]) -> String {
    let mut sorted = files.to_vec();
    sorted.sort_by(|a, b| a.path.cmp(&b.path));
    let mut text = String::new();
    for f in &sorted {
        text.push_str(&format!("{}\0{}\0{}\n", f.path, f.sha256, f.bytes));
    }
    sha256_hex(text.as_bytes())
}

impl Workspace {
    /// Create a workspace, or open it if it exists. It never touches anything outside `dir`.
    pub fn init(dir: &Path, now: &str) -> Result<Workspace, StorageError> {
        let root = GuestRoot::create(dir)?;
        root.mkdir_all("overlay")?;
        root.mkdir_all("snapshots")?;
        root.mkdir_all("logs")?;
        root.mkdir_all("experiments")?;
        let meta = if root.exists("workspace.json") {
            serde_json::from_slice(&root.read("workspace.json")?)
                .map_err(|e| StorageError::Parse(e.to_string()))?
        } else {
            let m = WorkspaceMeta {
                schema_version: 1,
                created_on: now.to_string(),
                base: None,
                limits: Limits::default(),
            };
            root.write(
                "workspace.json",
                &serde_json::to_vec_pretty(&m).map_err(|e| StorageError::Parse(e.to_string()))?,
            )?;
            m
        };
        let overlay = GuestRoot::open(&root.resolve("overlay")?)?;
        let snaps = GuestRoot::open(&root.resolve("snapshots")?)?;
        Ok(Workspace {
            root,
            overlay,
            snaps,
            meta,
        })
    }

    fn save_meta(&self) -> Result<(), StorageError> {
        self.root.write(
            "workspace.json",
            &serde_json::to_vec_pretty(&self.meta)
                .map_err(|e| StorageError::Parse(e.to_string()))?,
        )?;
        Ok(())
    }

    pub fn root(&self) -> &GuestRoot {
        &self.root
    }

    /// The writable guest filesystem. Every guest write goes through it.
    pub fn overlay(&self) -> &GuestRoot {
        &self.overlay
    }

    pub fn set_limits(&mut self, limits: Limits) -> Result<(), StorageError> {
        self.meta.limits = limits;
        self.save_meta()
    }

    pub fn base(&self) -> Option<&BaseImage> {
        self.meta.base.as_ref()
    }

    /// Register a base image by hash. The file is read to hash it and is never modified.
    pub fn register_base(
        &mut self,
        path: &Path,
        name: &str,
        now: &str,
    ) -> Result<BaseImage, StorageError> {
        let sha256 = sha256_file(path)?;
        let bytes = std::fs::metadata(path)?.len();
        let base = BaseImage {
            name: name.to_string(),
            sha256,
            bytes,
            registered_on: now.to_string(),
        };
        self.meta.base = Some(base.clone());
        self.save_meta()?;
        Ok(base)
    }

    /// Check a base image file against the registered hash.
    pub fn verify_base(&self, path: &Path) -> Result<(), StorageError> {
        let base = self
            .meta
            .base
            .as_ref()
            .ok_or(StorageError::NoBaseRegistered)?;
        let found = sha256_file(path)?;
        if found == base.sha256 {
            Ok(())
        } else {
            Err(StorageError::BaseChanged {
                expected: base.sha256.clone(),
                found,
            })
        }
    }

    pub fn overlay_files(&self) -> Result<Vec<FileEntry>, StorageError> {
        let mut out = Vec::new();
        for rel in self.overlay.list_all()? {
            let data = self.overlay.read(&rel)?;
            out.push(FileEntry {
                path: rel,
                sha256: sha256_hex(&data),
                bytes: data.len() as u64,
            });
        }
        Ok(out)
    }

    pub fn overlay_tree_hash(&self) -> Result<String, StorageError> {
        Ok(tree_hash(&self.overlay_files()?))
    }

    pub fn usage(&self) -> Result<Usage, StorageError> {
        let overlay_bytes = self.overlay_files()?.iter().map(|f| f.bytes).sum();
        let snapshot_bytes = self.snaps.disk_usage()?;
        Ok(Usage {
            overlay_bytes,
            snapshot_bytes,
            snapshots: self.list_snapshots()?.len() as u32,
        })
    }

    pub fn list_snapshots(&self) -> Result<Vec<SnapshotManifest>, StorageError> {
        let mut out = Vec::new();
        for rel in self.snaps.list_all()? {
            if rel.ends_with("/manifest.json") {
                out.push(
                    serde_json::from_slice::<SnapshotManifest>(&self.snaps.read(&rel)?)
                        .map_err(|e| StorageError::Parse(e.to_string()))?,
                );
            }
        }
        out.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(out)
    }

    /// Copy the overlay into a new snapshot with a manifest of hashes.
    pub fn snapshot(&self, label: &str, now: &str) -> Result<SnapshotManifest, StorageError> {
        let files = self.overlay_files()?;
        let total: u64 = files.iter().map(|f| f.bytes).sum();
        let used = self.usage()?;
        if used.overlay_bytes > self.meta.limits.max_overlay_bytes {
            return Err(StorageError::LimitExceeded {
                what: "overlay",
                limit: self.meta.limits.max_overlay_bytes,
                would_be: used.overlay_bytes,
            });
        }
        if used.snapshot_bytes + total > self.meta.limits.max_snapshot_total_bytes {
            return Err(StorageError::LimitExceeded {
                what: "snapshots",
                limit: self.meta.limits.max_snapshot_total_bytes,
                would_be: used.snapshot_bytes + total,
            });
        }
        let n = self.list_snapshots()?.len() + 1;
        let id = format!("snap-{n:04}");
        for f in &files {
            self.snaps.write(
                &format!("{id}/files/{}", f.path),
                &self.overlay.read(&f.path)?,
            )?;
        }
        let manifest = SnapshotManifest {
            id: id.clone(),
            label: label.to_string(),
            created_on: now.to_string(),
            tree_sha256: tree_hash(&files),
            files,
        };
        self.snaps.write(
            &format!("{id}/manifest.json"),
            &serde_json::to_vec_pretty(&manifest)
                .map_err(|e| StorageError::Parse(e.to_string()))?,
        )?;
        Ok(manifest)
    }

    fn load_snapshot(&self, id: &str) -> Result<SnapshotManifest, StorageError> {
        let rel = format!("{id}/manifest.json");
        if !self.snaps.exists(&rel) {
            return Err(StorageError::SnapshotNotFound(id.to_string()));
        }
        serde_json::from_slice(&self.snaps.read(&rel)?)
            .map_err(|e| StorageError::Parse(e.to_string()))
    }

    /// Check every file of a snapshot against its manifest.
    pub fn verify_snapshot(&self, id: &str) -> Result<SnapshotManifest, StorageError> {
        let m = self.load_snapshot(id)?;
        for f in &m.files {
            let rel = format!("{id}/files/{}", f.path);
            let ok = self
                .snaps
                .read(&rel)
                .map(|d| sha256_hex(&d) == f.sha256)
                .unwrap_or(false);
            if !ok {
                return Err(StorageError::SnapshotCorrupted {
                    id: id.to_string(),
                    path: f.path.clone(),
                });
            }
        }
        if tree_hash(&m.files) != m.tree_sha256 {
            return Err(StorageError::SnapshotCorrupted {
                id: id.to_string(),
                path: "manifest".to_string(),
            });
        }
        Ok(m)
    }

    /// Restore the overlay to a snapshot. The snapshot is verified before anything changes, and the
    /// result is verified afterwards.
    pub fn restore(&self, id: &str) -> Result<(), StorageError> {
        let m = self.verify_snapshot(id)?;
        for f in self.overlay_files()? {
            self.overlay.remove_file(&f.path)?;
        }
        for f in &m.files {
            self.overlay.write(
                &f.path,
                &self.snaps.read(&format!("{id}/files/{}", f.path))?,
            )?;
        }
        let found = self.overlay_tree_hash()?;
        if found != m.tree_sha256 {
            return Err(StorageError::OverlayCorrupted {
                expected: m.tree_sha256,
                found,
            });
        }
        Ok(())
    }

    /// Does the overlay still match a given tree hash?
    pub fn verify_overlay(&self, expected_tree_sha256: &str) -> Result<(), StorageError> {
        let found = self.overlay_tree_hash()?;
        if found == expected_tree_sha256 {
            Ok(())
        } else {
            Err(StorageError::OverlayCorrupted {
                expected: expected_tree_sha256.to_string(),
                found,
            })
        }
    }

    /// Delete a snapshot. Never without confirmation.
    pub fn delete_snapshot(&self, id: &str, confirm: bool) -> Result<(), StorageError> {
        if !confirm {
            return Err(StorageError::ConfirmationRequired("deleting a snapshot"));
        }
        self.load_snapshot(id)?;
        self.snaps.remove_dir_all(id)?;
        Ok(())
    }

    /// Discard the overlay, returning the guest to the base state. Never without confirmation.
    pub fn discard_overlay(&self, confirm: bool) -> Result<(), StorageError> {
        if !confirm {
            return Err(StorageError::ConfirmationRequired("discarding the overlay"));
        }
        for f in self.overlay_files()? {
            self.overlay.remove_file(&f.path)?;
        }
        Ok(())
    }

    pub fn host_root(&self) -> PathBuf {
        self.root.host_path().to_path_buf()
    }
}
