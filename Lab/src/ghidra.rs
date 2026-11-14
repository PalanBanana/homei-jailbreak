//! Read-only Ghidra integration.
//!
//! It reads existing analysis metadata (JSON manifests, exports and logs) and notes whether a
//! project exists. It does not open or modify a Ghidra database, does not run Ghidra, and never
//! reimports a binary. The summary it returns contains names and numbers only, never a local path.

use homei_core::labstate::GhidraSummary;
use serde_json::Value;
use std::path::PathBuf;

/// How an import problem is classified from a log line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ImportIssue {
    NoLoadSpec,
    OutOfMemory,
    Exception,
    UnresolvedReference,
    Other,
}

impl ImportIssue {
    pub fn label(self) -> &'static str {
        match self {
            ImportIssue::NoLoadSpec => "no load specification found",
            ImportIssue::OutOfMemory => "out of memory",
            ImportIssue::Exception => "exception during import or analysis",
            ImportIssue::UnresolvedReference => "unresolved reference",
            ImportIssue::Other => "other error",
        }
    }
}

pub fn classify_line(line: &str) -> Option<ImportIssue> {
    let l = line.to_lowercase();
    if l.contains("no load spec")
        || l.contains("unable to find a loader")
        || l.contains("no loader")
    {
        Some(ImportIssue::NoLoadSpec)
    } else if l.contains("outofmemory")
        || l.contains("out of memory")
        || l.contains("java heap space")
    {
        Some(ImportIssue::OutOfMemory)
    } else if l.contains("exception") {
        Some(ImportIssue::Exception)
    } else if l.contains("unresolved") || l.contains("unable to resolve") {
        Some(ImportIssue::UnresolvedReference)
    } else if l.contains("error") && !l.contains("0 errors") {
        Some(ImportIssue::Other)
    } else {
        None
    }
}

/// Count classified issues in a log.
pub fn classify_log(text: &str) -> Vec<(ImportIssue, usize)> {
    let mut counts = std::collections::BTreeMap::new();
    for line in text.lines() {
        if let Some(i) = classify_line(line) {
            *counts.entry(i).or_insert(0usize) += 1;
        }
    }
    counts.into_iter().collect()
}

pub struct Inspector {
    pub project_dir: PathBuf,
    pub project_name: String,
    pub analysis_dir: PathBuf,
    pub exports_dir: PathBuf,
    pub logs_dir: PathBuf,
}

fn num(v: &Value, path: &[&str]) -> Option<u64> {
    let mut cur = v;
    for p in path {
        cur = cur.get(*p)?;
    }
    cur.as_u64()
}

impl Inspector {
    /// Does a project database exist? Only the existence of `<name>.gpr` and `<name>.rep` is checked.
    pub fn project_present(&self) -> bool {
        self.project_dir
            .join(format!("{}.gpr", self.project_name))
            .is_file()
            && self
                .project_dir
                .join(format!("{}.rep", self.project_name))
                .is_dir()
    }

    fn read_json(&self, name: &str) -> Option<Value> {
        std::fs::read_to_string(self.analysis_dir.join(name))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
    }

    /// Summarise what the existing analysis records say. Missing files are reported as missing, not invented.
    pub fn inspect(
        &self,
        ghidra_version: Option<String>,
        program: &str,
        architecture: &str,
        last_analysis_date: Option<String>,
    ) -> GhidraSummary {
        let counts = self.read_json("function-counts-by-fileset-entry.json");
        let coverage = self.read_json("kernel-fileset-coverage.json");
        let symbols = self.read_json("kernel-symbolication-metrics.json");
        let function_count = counts
            .as_ref()
            .and_then(|c| num(c, &["functions_total"]))
            .unwrap_or(0);
        let fileset_entries = coverage
            .as_ref()
            .and_then(|c| num(c, &["fileset_entries_total"]))
            .unwrap_or(0);
        let covered = coverage
            .as_ref()
            .and_then(|c| num(c, &["fileset_entries_start_address_covered"]))
            .unwrap_or(0);
        let firmware_sha256 = symbols
            .as_ref()
            .and_then(|s| s.get("kernelcache_sha256"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let basis_points = symbols
            .as_ref()
            .and_then(|s| s.get("ipsw_reported"))
            .and_then(|r| r.get("percent"))
            .and_then(|p| p.as_f64())
            .map(|p| (p * 100.0).round() as u32);

        let mut errors = Vec::new();
        if let Ok(rd) = std::fs::read_dir(&self.logs_dir) {
            let mut names: Vec<_> = rd
                .filter_map(|e| e.ok())
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .filter(|n| n.starts_with("ghidra-") && n.ends_with(".log"))
                .collect();
            names.sort();
            for n in names {
                let text = std::fs::read_to_string(self.logs_dir.join(&n)).unwrap_or_default();
                for (issue, count) in classify_log(&text) {
                    errors.push(format!("{n}: {count} x {}", issue.label()));
                }
            }
        }
        let present = self.project_present();
        let have_metrics = function_count > 0;
        let import_status = match (present, have_metrics, errors.is_empty()) {
            (false, _, _) => "project not found".to_string(),
            (true, false, _) => "project present, no analysis metrics found".to_string(),
            (true, true, true) => "imported and analysed, no classified log issues".to_string(),
            (true, true, false) => {
                "imported and analysed, with classified log lines (see import_errors)".to_string()
            }
        };
        GhidraSummary {
            ghidra_version,
            project_name: self.project_name.clone(),
            project_present: present,
            program: program.to_string(),
            firmware_sha256,
            architecture: architecture.to_string(),
            import_status,
            function_count,
            fileset_entries,
            fileset_entries_covered: covered,
            symbol_match_basis_points: basis_points,
            import_errors: errors,
            last_analysis_date,
            source_reference: "local analysis records of the research tracker (not published)"
                .to_string(),
            confidence: if have_metrics {
                "static observation, medium".to_string()
            } else {
                "none".to_string()
            },
        }
    }
}
