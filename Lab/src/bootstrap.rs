//! VM-only research bootstrap.
//!
//! This provisions a predictable research workspace inside the isolated guest overlay. It is **not**
//! an iOS jailbreak bootstrap and does not touch a physical device, the host, the original firmware
//! or the immutable base image. It is repeatable and idempotent, versioned, logged, and reversible
//! by discarding the overlay.

use crate::guard::GuestRoot;
use crate::packages::{
    Approval, IndexEntry, InstallOptions, PackageIndex, PackageManager, PkgError, Request,
    TrustRoot,
};
use serde::{Deserialize, Serialize};

pub const BOOTSTRAP_VERSION: &str = "1";
pub const BASE: &str = "opt/homei-lab";
pub const LAYOUT: &[&str] = &[
    "bin",
    "packages",
    "profiles",
    "analysis",
    "logs",
    "manifests",
    "workspace",
];
const MANIFEST: &str = "opt/homei-lab/manifests/bootstrap.json";
const LOG: &str = "opt/homei-lab/logs/bootstrap.log";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BootstrapManifest {
    pub bootstrap_version: String,
    pub filesystem_scope: String,
    pub layout: Vec<String>,
    pub profiles: Vec<String>,
    /// `complete`, or `layout_only` if a requested package step failed.
    pub state: String,
    pub packages_requested: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Fresh,
    Unchanged,
    Upgraded,
    /// A requested step failed. The layout may exist, and the package step did not apply.
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub outcome: Outcome,
    pub created: Vec<String>,
    pub manifest: BootstrapManifest,
    pub package_transaction: Option<String>,
    pub error: Option<String>,
}

pub struct PackageJob<'a> {
    pub index: &'a PackageIndex,
    pub requests: Vec<Request>,
    pub fetch: &'a dyn Fn(&IndexEntry) -> Result<Vec<u8>, PkgError>,
    pub trust: &'a TrustRoot,
    pub options: InstallOptions,
    pub approval: Approval,
}

fn default_profile() -> &'static str {
    "{\n  \"profile\": \"default\",\n  \"scope\": \"isolated research guest\",\n  \"note\": \"Guest root is not physical device root.\"\n}\n"
}

fn log(root: &GuestRoot, now: &str, line: &str) {
    let _ = root.append(LOG, format!("{now} {line}\n").as_bytes());
}

/// Provision the layout, the profile and the manifest, then run the optional package step.
pub fn run(root: &GuestRoot, now: &str, job: Option<PackageJob<'_>>) -> Report {
    let mut created = Vec::new();
    for d in LAYOUT {
        let rel = format!("{BASE}/{d}");
        if !root.exists(&rel) {
            if root.mkdir_all(&rel).is_err() {
                return failed(root, now, "could not create the layout", created, None);
            }
            created.push(rel);
        }
    }
    let profile = format!("{BASE}/profiles/default.json");
    if !root.exists(&profile) {
        if root.write(&profile, default_profile().as_bytes()).is_err() {
            return failed(
                root,
                now,
                "could not write the default profile",
                created,
                None,
            );
        }
        created.push(profile);
    }
    let previous: Option<BootstrapManifest> = root
        .read(MANIFEST)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok());
    // A repeat run without a package step keeps what an earlier run recorded.
    let requested: Vec<String> = match &job {
        Some(j) => j.requests.iter().map(|r| r.id.clone()).collect(),
        None => previous
            .as_ref()
            .map(|p| p.packages_requested.clone())
            .unwrap_or_default(),
    };
    let mut manifest = BootstrapManifest {
        bootstrap_version: BOOTSTRAP_VERSION.to_string(),
        filesystem_scope: "isolated guest overlay (not a device)".to_string(),
        layout: LAYOUT.iter().map(|s| s.to_string()).collect(),
        profiles: vec!["default".to_string()],
        state: "complete".to_string(),
        packages_requested: requested,
    };

    let mut package_transaction = None;
    if let Some(job) = job {
        match PackageManager::open(root.clone()).and_then(|mut pm| {
            pm.install(
                job.approval,
                job.index,
                &job.requests,
                job.fetch,
                job.trust,
                job.options,
                now,
            )
        }) {
            Ok(tx) => package_transaction = Some(tx.id),
            Err(e) => {
                manifest.state = "layout_only".to_string();
                let _ = root.write(
                    MANIFEST,
                    &serde_json::to_vec_pretty(&manifest).unwrap_or_default(),
                );
                return failed(
                    root,
                    now,
                    &format!("package step failed: {e}"),
                    created,
                    Some(manifest),
                );
            }
        }
    }
    let outcome = match &previous {
        None => Outcome::Fresh,
        Some(p) if *p == manifest && created.is_empty() && package_transaction.is_none() => {
            Outcome::Unchanged
        }
        Some(_) => Outcome::Upgraded,
    };
    if outcome != Outcome::Unchanged
        && root
            .write(
                MANIFEST,
                &serde_json::to_vec_pretty(&manifest).unwrap_or_default(),
            )
            .is_err()
    {
        return failed(
            root,
            now,
            "could not write the bootstrap manifest",
            created,
            Some(manifest),
        );
    }
    log(
        root,
        now,
        &format!(
            "bootstrap v{BOOTSTRAP_VERSION}: {outcome:?}, {} item(s) created",
            created.len()
        ),
    );
    Report {
        outcome,
        created,
        manifest,
        package_transaction,
        error: None,
    }
}

fn failed(
    root: &GuestRoot,
    now: &str,
    why: &str,
    created: Vec<String>,
    manifest: Option<BootstrapManifest>,
) -> Report {
    log(root, now, &format!("bootstrap FAILED: {why}"));
    Report {
        outcome: Outcome::Failed,
        created,
        manifest: manifest.unwrap_or(BootstrapManifest {
            bootstrap_version: BOOTSTRAP_VERSION.to_string(),
            filesystem_scope: "isolated guest overlay (not a device)".to_string(),
            layout: Vec::new(),
            profiles: Vec::new(),
            state: "failed".to_string(),
            packages_requested: Vec::new(),
        }),
        package_transaction: None,
        error: Some(why.to_string()),
    }
}

/// The recorded state, read from the guest. A planned or failed step is never reported as installed.
pub fn state(root: &GuestRoot) -> Option<BootstrapManifest> {
    root.read(MANIFEST)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
}
