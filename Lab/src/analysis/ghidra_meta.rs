//! Ghidra project metadata importer. Read-only.
//!
//! It reads the project file (`.gpr`), the repository folder (`.rep`) and the names stored in the
//! repository index. It never opens the Ghidra database for writing, never starts Ghidra, and
//! never reads the owner field of the project properties. A count that Ghidra did not export is
//! reported as unknown (`None`), never inferred from the index.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NamePresence {
    pub name: String,
    /// True when the exact byte string occurs in some index file of the repository.
    pub present_in_index: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GhidraRecord {
    pub project_name: String,
    pub project_file_bytes: u64,
    /// `empty` for a zero-byte project file, `present` otherwise.
    pub project_file_state: String,
    pub repository_present: bool,
    /// Presence of the project properties file. Its contents are not copied.
    pub project_properties_present: bool,
    pub index_files: usize,
    pub index_bytes_scanned: u64,
    pub names: Vec<NamePresence>,
    /// Function count from an export. None: no export exists for this project.
    pub function_count: Option<u64>,
    pub export_present: bool,
}

fn rep_path(gpr: &Path) -> PathBuf {
    gpr.with_extension("rep")
}

fn collect_files(dir: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    for e in fs::read_dir(dir)? {
        let p = e?.path();
        if p.is_dir() {
            collect_files(&p, out)?;
        } else if p.is_file() {
            out.push(p);
        }
    }
    Ok(())
}

/// Find which of `needles` occur in a file. Reads in chunks, keeping an overlap so a match across
/// a chunk boundary is found.
fn scan_file(path: &Path, needles: &[&[u8]], found: &mut [bool]) -> io::Result<u64> {
    let mut f = fs::File::open(path)?;
    let overlap = needles
        .iter()
        .map(|n| n.len())
        .max()
        .unwrap_or(1)
        .saturating_sub(1);
    let mut buf = vec![0u8; 8 << 20];
    let mut carry: Vec<u8> = Vec::new();
    let mut total = 0u64;
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        total += n as u64;
        let mut window = std::mem::take(&mut carry);
        window.extend_from_slice(&buf[..n]);
        for (i, needle) in needles.iter().enumerate() {
            if !found[i] && window.windows(needle.len()).any(|w| w == *needle) {
                found[i] = true;
            }
        }
        let keep = overlap.min(window.len());
        carry = window[window.len() - keep..].to_vec();
    }
    Ok(total)
}

/// Inspect a Ghidra project. `names` are the exact strings to look for in the repository index.
pub fn inspect(gpr: &Path, names: &[&str]) -> io::Result<GhidraRecord> {
    let project_name = gpr
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let project_file_bytes = fs::metadata(gpr)?.len();
    let rep = rep_path(gpr);
    let repository_present = rep.is_dir();
    let props = rep.join("project.prp");
    let project_properties_present = props.is_file();

    let mut files = Vec::new();
    collect_files(&rep.join("idata"), &mut files)?;
    files.sort();
    let needles: Vec<&[u8]> = names.iter().map(|n| n.as_bytes()).collect();
    let mut found = vec![false; names.len()];
    let mut bytes = 0u64;
    for f in &files {
        bytes += scan_file(f, &needles, &mut found)?;
    }

    Ok(GhidraRecord {
        project_name,
        project_file_bytes,
        project_file_state: if project_file_bytes == 0 {
            "empty"
        } else {
            "present"
        }
        .to_string(),
        repository_present,
        project_properties_present,
        index_files: files.len(),
        index_bytes_scanned: bytes,
        names: names
            .iter()
            .zip(found)
            .map(|(n, f)| NamePresence {
                name: (*n).to_string(),
                present_in_index: f,
            })
            .collect(),
        function_count: None,
        export_present: false,
    })
}
