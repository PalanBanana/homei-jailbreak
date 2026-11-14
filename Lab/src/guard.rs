//! The only way the lab writes to a guest filesystem.
//!
//! A `GuestRoot` is one directory. Every path given to it must be relative and plain: no empty,
//! `.` or `..` component, no NUL, no backslash, no colon, and no existing symbolic link anywhere on
//! the way. A package, a bootstrap step or a snapshot therefore cannot write outside the overlay,
//! into the host, or into the original base image, whatever its own contents say.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuardError {
    NotADirectory(String),
    EmptyPath,
    Absolute(String),
    Traversal(String),
    BadCharacter(String),
    Symlink(String),
    Io(String),
}

impl fmt::Display for GuardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GuardError::NotADirectory(p) => write!(f, "guest root is not a directory: {p}"),
            GuardError::EmptyPath => write!(f, "empty guest path"),
            GuardError::Absolute(p) => write!(f, "absolute guest path refused: {p}"),
            GuardError::Traversal(p) => write!(f, "path traversal refused: {p}"),
            GuardError::BadCharacter(p) => write!(f, "unsafe character in guest path: {p:?}"),
            GuardError::Symlink(p) => write!(f, "symbolic link on the way, refused: {p}"),
            GuardError::Io(e) => write!(f, "io error: {e}"),
        }
    }
}

impl std::error::Error for GuardError {}

impl From<std::io::Error> for GuardError {
    fn from(e: std::io::Error) -> Self {
        GuardError::Io(e.kind().to_string())
    }
}

/// Validate a relative guest path and return its components.
pub fn components(rel: &str) -> Result<Vec<&str>, GuardError> {
    if rel.is_empty() {
        return Err(GuardError::EmptyPath);
    }
    if rel.starts_with('/') {
        return Err(GuardError::Absolute(rel.to_string()));
    }
    if rel.contains(['\0', '\\', ':']) {
        return Err(GuardError::BadCharacter(rel.to_string()));
    }
    let mut out = Vec::new();
    for c in rel.split('/') {
        match c {
            "" | "." | ".." => return Err(GuardError::Traversal(rel.to_string())),
            other => out.push(other),
        }
    }
    Ok(out)
}

#[derive(Debug, Clone)]
pub struct GuestRoot {
    root: PathBuf,
}

impl GuestRoot {
    /// Open an existing directory as a guest root.
    pub fn open(root: &Path) -> Result<GuestRoot, GuardError> {
        let canonical = fs::canonicalize(root)
            .map_err(|_| GuardError::NotADirectory(root.display().to_string()))?;
        if !canonical.is_dir() {
            return Err(GuardError::NotADirectory(root.display().to_string()));
        }
        Ok(GuestRoot { root: canonical })
    }

    /// Create the directory if needed, then open it.
    pub fn create(root: &Path) -> Result<GuestRoot, GuardError> {
        fs::create_dir_all(root)?;
        GuestRoot::open(root)
    }

    /// The host path of the root. Used for display inside the host controller only.
    pub fn host_path(&self) -> &Path {
        &self.root
    }

    /// Resolve a relative guest path to a host path inside the root, refusing symlinks on the way.
    pub fn resolve(&self, rel: &str) -> Result<PathBuf, GuardError> {
        let parts = components(rel)?;
        let mut cur = self.root.clone();
        for part in parts {
            cur.push(part);
            if let Ok(meta) = fs::symlink_metadata(&cur) {
                if meta.file_type().is_symlink() {
                    return Err(GuardError::Symlink(rel.to_string()));
                }
            }
        }
        Ok(cur)
    }

    pub fn exists(&self, rel: &str) -> bool {
        self.resolve(rel).map(|p| p.exists()).unwrap_or(false)
    }

    pub fn mkdir_all(&self, rel: &str) -> Result<(), GuardError> {
        fs::create_dir_all(self.resolve(rel)?)?;
        Ok(())
    }

    pub fn write(&self, rel: &str, bytes: &[u8]) -> Result<(), GuardError> {
        let path = self.resolve(rel)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, bytes)?;
        Ok(())
    }

    pub fn append(&self, rel: &str, bytes: &[u8]) -> Result<(), GuardError> {
        use std::io::Write;
        let path = self.resolve(rel)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut f = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        f.write_all(bytes)?;
        Ok(())
    }

    pub fn read(&self, rel: &str) -> Result<Vec<u8>, GuardError> {
        Ok(fs::read(self.resolve(rel)?)?)
    }

    pub fn remove_file(&self, rel: &str) -> Result<(), GuardError> {
        let p = self.resolve(rel)?;
        if p.exists() {
            fs::remove_file(p)?;
        }
        Ok(())
    }

    pub fn remove_dir_all(&self, rel: &str) -> Result<(), GuardError> {
        let p = self.resolve(rel)?;
        if p.exists() {
            fs::remove_dir_all(p)?;
        }
        Ok(())
    }

    pub fn rename(&self, from: &str, to: &str) -> Result<(), GuardError> {
        let (a, b) = (self.resolve(from)?, self.resolve(to)?);
        if let Some(parent) = b.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::rename(a, b)?;
        Ok(())
    }

    /// Relative paths of every regular file below `rel`, sorted. Symlinks are never followed.
    pub fn list_files(&self, rel: &str) -> Result<Vec<String>, GuardError> {
        let base = self.resolve(rel)?;
        self.walk_from(&base)
    }

    /// Relative paths of every regular file in the whole root, sorted.
    pub fn list_all(&self) -> Result<Vec<String>, GuardError> {
        self.walk_from(&self.root)
    }

    fn walk_from(&self, base: &Path) -> Result<Vec<String>, GuardError> {
        let mut out = Vec::new();
        fn walk(dir: &Path, root: &Path, out: &mut Vec<String>) -> std::io::Result<()> {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let meta = fs::symlink_metadata(entry.path())?;
                if meta.is_dir() {
                    walk(&entry.path(), root, out)?;
                } else if meta.is_file() {
                    if let Ok(r) = entry.path().strip_prefix(root) {
                        out.push(r.to_string_lossy().replace('\\', "/"));
                    }
                }
            }
            Ok(())
        }
        if base.is_dir() {
            walk(base, &self.root, &mut out)?;
        }
        out.sort();
        Ok(out)
    }

    /// Total size in bytes of every regular file in the root.
    pub fn disk_usage(&self) -> Result<u64, GuardError> {
        let mut total = 0;
        for f in self.list_all()? {
            total += fs::metadata(self.resolve(&f)?)?.len();
        }
        Ok(total)
    }
}
