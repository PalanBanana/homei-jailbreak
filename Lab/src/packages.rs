//! Guest-only package manager.
//!
//! It installs user-space research utilities into the isolated guest overlay and nowhere else. It
//! is not an iOS package manager and an installation here is not an installation on any iPhone.
//!
//! Safety properties, each covered by a test:
//! - every write goes through `GuestRoot`, so a package cannot leave the overlay;
//! - an archive is checked against the hash in the index, and its signature against an explicitly
//!   supplied trust root. An unsigned package is refused unless the user opts in, and is then marked;
//! - maintainer scripts are rejected, and nothing in a package is ever executed;
//! - an install or removal is a journaled transaction that rolls back on failure or interruption;
//! - nothing is installed without an explicit `Approval`.

use crate::guard::{components, GuardError, GuestRoot};
use crate::hash::{from_hex, is_sha256_hex, sha256_hex, to_hex};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const MAGIC: &[u8; 5] = b"HPK1\n";
pub const MAX_HEADER_BYTES: usize = 1 << 20;
pub const MAX_FILE_BYTES: u64 = 64 << 20;
pub const MAX_TOTAL_BYTES: u64 = 256 << 20;
pub const MAX_FILES: usize = 10_000;

pub const PACKAGES_DIR: &str = "opt/homei-lab/packages";
pub const RECORDS_DIR: &str = "opt/homei-lab/manifests/packages";
pub const TX_DIR: &str = "opt/homei-lab/manifests/transactions";
pub const STAGING_DIR: &str = "opt/homei-lab/.staging";
pub const TRASH_DIR: &str = "opt/homei-lab/.trash";

/// File names that mark a maintainer script. A package containing one is rejected.
const SCRIPT_NAMES: &[&str] = &[
    "preinst",
    "postinst",
    "prerm",
    "postrm",
    "preinstall",
    "postinstall",
    "install.sh",
    "uninstall.sh",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PkgError {
    BadMagic,
    HeaderTooLarge(usize),
    BadHeader(String),
    BadId(String),
    BadVersion(String),
    BadRequirement(String),
    UnsafePath(String),
    TooManyFiles(usize),
    FileTooLarge {
        path: String,
        bytes: u64,
    },
    PayloadSizeMismatch {
        expected: u64,
        found: u64,
    },
    FileHashMismatch(String),
    MaintainerScript(String),
    IntegrityMismatch {
        id: String,
        expected: String,
        found: String,
    },
    MissingSignature(String),
    InvalidSignature(String),
    UntrustedKey,
    IndexMismatch(String),
    NotInIndex(String),
    NoVersionSatisfies {
        id: String,
        requirement: String,
    },
    Conflict {
        a: String,
        b: String,
    },
    DependencyCycle(String),
    NotInstalled(String),
    RequiredBy {
        id: String,
        dependents: Vec<String>,
    },
    NothingToRollBack,
    Io(String),
    Aborted,
}

impl fmt::Display for PkgError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for PkgError {}

impl From<GuardError> for PkgError {
    fn from(e: GuardError) -> Self {
        match e {
            GuardError::Io(m) => PkgError::Io(m),
            other => PkgError::UnsafePath(other.to_string()),
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Versions and requirements

#[derive(Debug, Clone)]
pub struct Version(pub Vec<u64>);

/// Equality follows the ordering, so `1.0` and `1.0.0` are the same version.
impl PartialEq for Version {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Version {}

impl Version {
    pub fn parse(s: &str) -> Result<Version, PkgError> {
        let parts: Result<Vec<u64>, _> = s.split('.').map(|p| p.parse::<u64>()).collect();
        match parts {
            Ok(v) if !v.is_empty() && v.len() <= 6 && !s.contains(' ') => Ok(Version(v)),
            _ => Err(PkgError::BadVersion(s.to_string())),
        }
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        let n = self.0.len().max(other.0.len());
        for i in 0..n {
            let (a, b) = (
                self.0.get(i).copied().unwrap_or(0),
                other.0.get(i).copied().unwrap_or(0),
            );
            if a != b {
                return a.cmp(&b);
            }
        }
        Ordering::Equal
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// A comma-separated list of comparators such as `>=1.0,<2.0`, `=1.2.3` or `*`.
pub fn satisfies(version: &Version, requirement: &str) -> Result<bool, PkgError> {
    let req = requirement.trim();
    if req.is_empty() || req == "*" {
        return Ok(true);
    }
    for part in req.split(',') {
        let p = part.trim();
        let (op, rest) = if let Some(r) = p.strip_prefix(">=") {
            (">=", r)
        } else if let Some(r) = p.strip_prefix("<=") {
            ("<=", r)
        } else if let Some(r) = p.strip_prefix('>') {
            (">", r)
        } else if let Some(r) = p.strip_prefix('<') {
            ("<", r)
        } else if let Some(r) = p.strip_prefix('=') {
            ("=", r)
        } else {
            return Err(PkgError::BadRequirement(requirement.to_string()));
        };
        let v = Version::parse(rest.trim())
            .map_err(|_| PkgError::BadRequirement(requirement.to_string()))?;
        let ok = match op {
            ">=" => *version >= v,
            "<=" => *version <= v,
            ">" => *version > v,
            "<" => *version < v,
            _ => *version == v,
        };
        if !ok {
            return Ok(false);
        }
    }
    Ok(true)
}

pub fn valid_id(id: &str) -> bool {
    let b = id.as_bytes();
    !b.is_empty()
        && b.len() <= 64
        && (b[0].is_ascii_lowercase() || b[0].is_ascii_digit())
        && b.iter().all(|c| {
            c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, b'.' | b'-' | b'_')
        })
}

// ---------------------------------------------------------------------------------------------
// Archive format

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dependency {
    pub id: String,
    pub requirement: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageFile {
    pub path: String,
    pub size: u64,
    pub sha256: String,
    pub executable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageHeader {
    pub format: u32,
    pub id: String,
    pub version: String,
    pub description: String,
    pub license: String,
    pub source: String,
    pub depends: Vec<Dependency>,
    pub conflicts: Vec<String>,
    /// Must be empty. A package that declares a script is rejected.
    pub scripts: Vec<String>,
    pub files: Vec<PackageFile>,
}

/// Build an archive. Used by tests and by the sample repository command.
pub fn build_archive(header: &PackageHeader, payloads: &[Vec<u8>]) -> Vec<u8> {
    let json = serde_json::to_vec(header).expect("header serialises");
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&(json.len() as u64).to_be_bytes());
    out.extend_from_slice(&json);
    for p in payloads {
        out.extend_from_slice(p);
    }
    out
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedArchive {
    pub header: PackageHeader,
    pub files: Vec<(String, Vec<u8>, bool)>,
}

/// Parse and fully validate an archive. Nothing is written.
pub fn parse_archive(bytes: &[u8]) -> Result<ParsedArchive, PkgError> {
    if bytes.len() < MAGIC.len() + 8 || &bytes[..MAGIC.len()] != MAGIC {
        return Err(PkgError::BadMagic);
    }
    let len = u64::from_be_bytes(
        bytes[MAGIC.len()..MAGIC.len() + 8]
            .try_into()
            .expect("8 bytes"),
    ) as usize;
    if len > MAX_HEADER_BYTES {
        return Err(PkgError::HeaderTooLarge(len));
    }
    let start = MAGIC.len() + 8;
    let header_bytes = bytes
        .get(start..start + len)
        .ok_or_else(|| PkgError::BadHeader("truncated header".into()))?;
    let header: PackageHeader =
        serde_json::from_slice(header_bytes).map_err(|e| PkgError::BadHeader(e.to_string()))?;
    if header.format != 1 {
        return Err(PkgError::BadHeader(format!(
            "unsupported format {}",
            header.format
        )));
    }
    if !valid_id(&header.id) {
        return Err(PkgError::BadId(header.id));
    }
    Version::parse(&header.version)?;
    if !header.scripts.is_empty() {
        return Err(PkgError::MaintainerScript(header.scripts.join(",")));
    }
    if header.files.len() > MAX_FILES {
        return Err(PkgError::TooManyFiles(header.files.len()));
    }
    let payload = &bytes[start + len..];
    let mut offset = 0usize;
    let mut total = 0u64;
    let mut files = Vec::new();
    let mut seen = BTreeSet::new();
    for f in &header.files {
        let parts =
            components(&f.path).map_err(|e| PkgError::UnsafePath(format!("{}: {e}", f.path)))?;
        if parts
            .iter()
            .any(|p| SCRIPT_NAMES.contains(p) || *p == ".scripts")
        {
            return Err(PkgError::MaintainerScript(f.path.clone()));
        }
        if !seen.insert(f.path.clone()) {
            return Err(PkgError::UnsafePath(format!("duplicate path {}", f.path)));
        }
        if f.size > MAX_FILE_BYTES {
            return Err(PkgError::FileTooLarge {
                path: f.path.clone(),
                bytes: f.size,
            });
        }
        total += f.size;
        if total > MAX_TOTAL_BYTES {
            return Err(PkgError::FileTooLarge {
                path: "total".into(),
                bytes: total,
            });
        }
        let end = offset + f.size as usize;
        let data = payload
            .get(offset..end)
            .ok_or(PkgError::PayloadSizeMismatch {
                expected: total,
                found: payload.len() as u64,
            })?;
        if sha256_hex(data) != f.sha256 {
            return Err(PkgError::FileHashMismatch(f.path.clone()));
        }
        files.push((f.path.clone(), data.to_vec(), f.executable));
        offset = end;
    }
    if offset != payload.len() {
        return Err(PkgError::PayloadSizeMismatch {
            expected: offset as u64,
            found: payload.len() as u64,
        });
    }
    Ok(ParsedArchive { header, files })
}

// ---------------------------------------------------------------------------------------------
// Index, trust and signatures

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IndexEntry {
    pub id: String,
    pub version: String,
    pub archive: String,
    pub sha256: String,
    pub size: u64,
    pub description: String,
    pub depends: Vec<Dependency>,
    pub conflicts: Vec<String>,
    /// Hex Ed25519 signature over `signing_message`, or absent for an unsigned package.
    pub signature: Option<String>,
    pub provenance: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageIndex {
    pub schema_version: u32,
    pub source_name: String,
    pub packages: Vec<IndexEntry>,
}

/// The exact bytes that are signed for one package.
pub fn signing_message(id: &str, version: &str, sha256: &str) -> Vec<u8> {
    format!("HOMEI-PKG-V1\n{id}\n{version}\n{sha256}").into_bytes()
}

#[derive(Debug, Clone, Default)]
pub struct TrustRoot {
    keys: Vec<VerifyingKey>,
}

impl TrustRoot {
    pub fn empty() -> Self {
        TrustRoot { keys: Vec::new() }
    }

    /// Add a trusted public key from 64 hex characters. Keys are only ever added explicitly.
    pub fn with_key_hex(mut self, hex: &str) -> Result<Self, PkgError> {
        let bytes = from_hex(hex)
            .filter(|b| b.len() == 32)
            .ok_or(PkgError::UntrustedKey)?;
        let arr: [u8; 32] = bytes.try_into().map_err(|_| PkgError::UntrustedKey)?;
        self.keys
            .push(VerifyingKey::from_bytes(&arr).map_err(|_| PkgError::UntrustedKey)?);
        Ok(self)
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    fn verify(&self, message: &[u8], signature_hex: &str) -> bool {
        let Some(bytes) = from_hex(signature_hex).filter(|b| b.len() == 64) else {
            return false;
        };
        let Ok(arr) = <[u8; 64]>::try_from(bytes) else {
            return false;
        };
        let sig = Signature::from_bytes(&arr);
        self.keys.iter().any(|k| k.verify(message, &sig).is_ok())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignatureState {
    /// Verified against a key in the explicit trust root.
    Valid,
    /// No signature, installed only because the user opted in. Provenance is unverified.
    UnsignedAccepted,
}

/// Check an archive against its index entry and the trust root. It does not write anything.
pub fn verify_archive(
    entry: &IndexEntry,
    bytes: &[u8],
    trust: &TrustRoot,
    allow_unsigned: bool,
) -> Result<SignatureState, PkgError> {
    if !is_sha256_hex(&entry.sha256) {
        return Err(PkgError::IndexMismatch(format!(
            "{}: malformed digest",
            entry.id
        )));
    }
    let found = sha256_hex(bytes);
    if found != entry.sha256 {
        return Err(PkgError::IntegrityMismatch {
            id: entry.id.clone(),
            expected: entry.sha256.clone(),
            found,
        });
    }
    match &entry.signature {
        Some(sig) => {
            if trust.is_empty() {
                return Err(PkgError::UntrustedKey);
            }
            if trust.verify(
                &signing_message(&entry.id, &entry.version, &entry.sha256),
                sig,
            ) {
                Ok(SignatureState::Valid)
            } else {
                Err(PkgError::InvalidSignature(entry.id.clone()))
            }
        }
        None if allow_unsigned => Ok(SignatureState::UnsignedAccepted),
        None => Err(PkgError::MissingSignature(entry.id.clone())),
    }
}

// ---------------------------------------------------------------------------------------------
// Resolution

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub id: String,
    pub requirement: String,
}

/// Choose the versions to install, dependencies first. Already installed packages that satisfy a
/// requirement are kept. A requirement no version satisfies, a conflict or a cycle is an error.
pub fn resolve(
    index: &PackageIndex,
    requests: &[Request],
    installed: &BTreeMap<String, String>,
) -> Result<Vec<IndexEntry>, PkgError> {
    let mut chosen: BTreeMap<String, IndexEntry> = BTreeMap::new();
    let mut order: Vec<String> = Vec::new();
    let mut stack: Vec<String> = Vec::new();

    fn pick(index: &PackageIndex, id: &str, requirement: &str) -> Result<IndexEntry, PkgError> {
        let mut best: Option<(Version, &IndexEntry)> = None;
        for e in index.packages.iter().filter(|e| e.id == id) {
            let v = Version::parse(&e.version)?;
            if satisfies(&v, requirement)? && best.as_ref().is_none_or(|(b, _)| v > *b) {
                best = Some((v, e));
            }
        }
        best.map(|(_, e)| e.clone()).ok_or_else(|| {
            if index.packages.iter().any(|e| e.id == id) {
                PkgError::NoVersionSatisfies {
                    id: id.to_string(),
                    requirement: requirement.to_string(),
                }
            } else {
                PkgError::NotInIndex(id.to_string())
            }
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn visit(
        index: &PackageIndex,
        id: &str,
        requirement: &str,
        installed: &BTreeMap<String, String>,
        chosen: &mut BTreeMap<String, IndexEntry>,
        order: &mut Vec<String>,
        stack: &mut Vec<String>,
    ) -> Result<(), PkgError> {
        if stack.iter().any(|s| s == id) {
            return Err(PkgError::DependencyCycle(format!(
                "{} -> {id}",
                stack.join(" -> ")
            )));
        }
        if let Some(existing) = chosen.get(id) {
            let v = Version::parse(&existing.version)?;
            return if satisfies(&v, requirement)? {
                Ok(())
            } else {
                Err(PkgError::NoVersionSatisfies {
                    id: id.to_string(),
                    requirement: requirement.to_string(),
                })
            };
        }
        if let Some(have) = installed.get(id) {
            if satisfies(&Version::parse(have)?, requirement)? {
                return Ok(());
            }
        }
        let entry = pick(index, id, requirement)?;
        stack.push(id.to_string());
        for d in &entry.depends {
            visit(
                index,
                &d.id,
                &d.requirement,
                installed,
                chosen,
                order,
                stack,
            )?;
        }
        stack.pop();
        chosen.insert(id.to_string(), entry);
        order.push(id.to_string());
        Ok(())
    }

    for r in requests {
        visit(
            index,
            &r.id,
            &r.requirement,
            installed,
            &mut chosen,
            &mut order,
            &mut stack,
        )?;
    }
    let plan: Vec<IndexEntry> = order
        .iter()
        .filter_map(|id| chosen.get(id).cloned())
        .collect();
    // Declared conflicts, among the chosen packages and against what is installed.
    let present: BTreeSet<&str> = plan
        .iter()
        .map(|e| e.id.as_str())
        .chain(installed.keys().map(|s| s.as_str()))
        .collect();
    for e in &plan {
        for c in &e.conflicts {
            if present.contains(c.as_str()) && c != &e.id {
                return Err(PkgError::Conflict {
                    a: e.id.clone(),
                    b: c.clone(),
                });
            }
        }
    }
    Ok(plan)
}

// ---------------------------------------------------------------------------------------------
// Installed state and transactions

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledFile {
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledRecord {
    pub id: String,
    pub version: String,
    pub source: String,
    pub provenance: String,
    pub archive_sha256: String,
    pub signature: SignatureState,
    pub depends: Vec<Dependency>,
    pub files: Vec<InstalledFile>,
    pub transaction: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TxState {
    InProgress,
    Committed,
    RolledBack,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TxOp {
    pub action: String,
    pub id: String,
    pub version: String,
    /// The previous version that was moved aside, if any.
    pub replaced: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transaction {
    pub id: String,
    pub state: TxState,
    pub started_on: String,
    pub ops: Vec<TxOp>,
}

/// Proof that a person explicitly approved a change. Only the caller can create it.
#[derive(Debug, Clone, Copy)]
pub struct Approval(());

impl Approval {
    /// Create an approval. The command line does this only when `--approve` was passed.
    pub fn user_approved() -> Approval {
        Approval(())
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct InstallOptions {
    pub allow_unsigned: bool,
    /// Simulate an interruption after this many packages were placed. For tests of the rollback path.
    pub abort_after_packages: Option<usize>,
}

pub struct PackageManager {
    root: GuestRoot,
    pub recovered: Vec<String>,
}

fn j<T: Serialize>(v: &T) -> Vec<u8> {
    serde_json::to_vec_pretty(v).expect("serialises")
}

impl PackageManager {
    /// Open the manager on a guest root (the overlay). Any transaction left in progress is rolled back.
    pub fn open(root: GuestRoot) -> Result<PackageManager, PkgError> {
        let mut pm = PackageManager {
            root,
            recovered: Vec::new(),
        };
        for rel in pm.root.list_files(TX_DIR).unwrap_or_default() {
            if let Ok(tx) = serde_json::from_slice::<Transaction>(&pm.root.read(&rel)?) {
                if tx.state == TxState::InProgress {
                    pm.roll_back_tx(&tx.id)?;
                    pm.recovered.push(tx.id);
                }
            }
        }
        Ok(pm)
    }

    pub fn root(&self) -> &GuestRoot {
        &self.root
    }

    pub fn installed(&self) -> Result<Vec<InstalledRecord>, PkgError> {
        let mut out = Vec::new();
        for rel in self.root.list_files(RECORDS_DIR).unwrap_or_default() {
            out.push(
                serde_json::from_slice::<InstalledRecord>(&self.root.read(&rel)?)
                    .map_err(|e| PkgError::Io(e.to_string()))?,
            );
        }
        out.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(out)
    }

    pub fn installed_versions(&self) -> Result<BTreeMap<String, String>, PkgError> {
        Ok(self
            .installed()?
            .into_iter()
            .map(|r| (r.id, r.version))
            .collect())
    }

    pub fn transactions(&self) -> Result<Vec<Transaction>, PkgError> {
        let mut out = Vec::new();
        for rel in self.root.list_files(TX_DIR).unwrap_or_default() {
            out.push(
                serde_json::from_slice::<Transaction>(&self.root.read(&rel)?)
                    .map_err(|e| PkgError::Io(e.to_string()))?,
            );
        }
        out.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(out)
    }

    fn next_tx_id(&self) -> Result<String, PkgError> {
        Ok(format!("tx-{:04}", self.transactions()?.len() + 1))
    }

    fn save_tx(&self, tx: &Transaction) -> Result<(), PkgError> {
        self.root
            .write(&format!("{TX_DIR}/{}.json", tx.id), &j(tx))?;
        Ok(())
    }

    /// Install the packages the request resolves to. `fetch` returns the archive bytes for an entry.
    /// Every archive is verified and parsed before anything is written.
    #[allow(clippy::too_many_arguments)]
    pub fn install(
        &mut self,
        _approval: Approval,
        index: &PackageIndex,
        requests: &[Request],
        fetch: &dyn Fn(&IndexEntry) -> Result<Vec<u8>, PkgError>,
        trust: &TrustRoot,
        options: InstallOptions,
        now: &str,
    ) -> Result<Transaction, PkgError> {
        let plan = resolve(index, requests, &self.installed_versions()?)?;
        let mut verified = Vec::new();
        for entry in &plan {
            let bytes = fetch(entry)?;
            let sig = verify_archive(entry, &bytes, trust, options.allow_unsigned)?;
            let parsed = parse_archive(&bytes)?;
            if parsed.header.id != entry.id
                || parsed.header.version != entry.version
                || parsed.header.depends != entry.depends
                || parsed.header.conflicts != entry.conflicts
            {
                return Err(PkgError::IndexMismatch(entry.id.clone()));
            }
            verified.push((entry.clone(), parsed, sig));
        }
        let id = self.next_tx_id()?;
        let mut tx = Transaction {
            id: id.clone(),
            state: TxState::InProgress,
            started_on: now.to_string(),
            ops: Vec::new(),
        };
        self.save_tx(&tx)?;
        let result = self.apply_install(&mut tx, &verified, options);
        match result {
            Ok(()) => {
                tx.state = TxState::Committed;
                self.save_tx(&tx)?;
                self.root.remove_dir_all(&format!("{STAGING_DIR}/{id}"))?;
                Ok(tx)
            }
            Err(e) => {
                self.roll_back_tx(&id)?;
                Err(e)
            }
        }
    }

    fn apply_install(
        &self,
        tx: &mut Transaction,
        verified: &[(IndexEntry, ParsedArchive, SignatureState)],
        options: InstallOptions,
    ) -> Result<(), PkgError> {
        for (n, (entry, parsed, sig)) in verified.iter().enumerate() {
            let dest = format!("{PACKAGES_DIR}/{}", entry.id);
            let replaced = self.installed_versions()?.get(&entry.id).cloned();
            // Journal first, then change. If anything fails from here on, rollback knows what to undo.
            tx.ops.push(TxOp {
                action: "install".into(),
                id: entry.id.clone(),
                version: entry.version.clone(),
                replaced,
            });
            self.save_tx(tx)?;

            let staged = format!("{STAGING_DIR}/{}/{}", tx.id, entry.id);
            let mut files = Vec::new();
            for (path, data, _exec) in &parsed.files {
                self.root.write(&format!("{staged}/{path}"), data)?;
                files.push(InstalledFile {
                    path: path.clone(),
                    sha256: sha256_hex(data),
                });
            }
            if self.root.exists(&dest) {
                self.root
                    .rename(&dest, &format!("{TRASH_DIR}/{}/{}", tx.id, entry.id))?;
            }
            let record_rel = format!("{RECORDS_DIR}/{}.json", entry.id);
            if self.root.exists(&record_rel) {
                self.root.rename(
                    &record_rel,
                    &format!("{TRASH_DIR}/{}/{}.record.json", tx.id, entry.id),
                )?;
            }
            if parsed.files.is_empty() {
                self.root.mkdir_all(&dest)?;
            } else {
                self.root.rename(&staged, &dest)?;
            }
            let record = InstalledRecord {
                id: entry.id.clone(),
                version: entry.version.clone(),
                source: parsed.header.source.clone(),
                provenance: entry.provenance.clone(),
                archive_sha256: entry.sha256.clone(),
                signature: *sig,
                depends: entry.depends.clone(),
                files,
                transaction: tx.id.clone(),
            };
            self.root.write(&record_rel, &j(&record))?;
            if options.abort_after_packages == Some(n + 1) {
                return Err(PkgError::Aborted);
            }
        }
        Ok(())
    }

    /// Remove a package. Refused while another installed package depends on it.
    pub fn remove(
        &mut self,
        _approval: Approval,
        id: &str,
        now: &str,
    ) -> Result<Transaction, PkgError> {
        let installed = self.installed()?;
        let target = installed
            .iter()
            .find(|r| r.id == id)
            .ok_or_else(|| PkgError::NotInstalled(id.to_string()))?;
        let dependents: Vec<String> = installed
            .iter()
            .filter(|r| r.id != id && r.depends.iter().any(|d| d.id == id))
            .map(|r| r.id.clone())
            .collect();
        if !dependents.is_empty() {
            return Err(PkgError::RequiredBy {
                id: id.to_string(),
                dependents,
            });
        }
        let txid = self.next_tx_id()?;
        let mut tx = Transaction {
            id: txid.clone(),
            state: TxState::InProgress,
            started_on: now.to_string(),
            ops: Vec::new(),
        };
        self.save_tx(&tx)?;
        let step = (|| -> Result<(), PkgError> {
            self.root.rename(
                &format!("{PACKAGES_DIR}/{id}"),
                &format!("{TRASH_DIR}/{txid}/{id}"),
            )?;
            self.root.rename(
                &format!("{RECORDS_DIR}/{id}.json"),
                &format!("{TRASH_DIR}/{txid}/{id}.record.json"),
            )?;
            Ok(())
        })();
        if let Err(e) = step {
            self.roll_back_tx(&txid)?;
            return Err(e);
        }
        tx.ops.push(TxOp {
            action: "remove".into(),
            id: id.to_string(),
            version: target.version.clone(),
            replaced: None,
        });
        tx.state = TxState::Committed;
        self.save_tx(&tx)?;
        Ok(tx)
    }

    /// Undo the most recent committed transaction.
    pub fn rollback_last(&mut self, _approval: Approval) -> Result<Transaction, PkgError> {
        let last = self
            .transactions()?
            .into_iter()
            .rev()
            .find(|t| t.state == TxState::Committed)
            .ok_or(PkgError::NothingToRollBack)?;
        self.roll_back_tx(&last.id)?;
        Ok(self
            .transactions()?
            .into_iter()
            .find(|t| t.id == last.id)
            .expect("transaction exists"))
    }

    /// Restore what a transaction moved aside and delete what it placed.
    fn roll_back_tx(&self, txid: &str) -> Result<(), PkgError> {
        let rel = format!("{TX_DIR}/{txid}.json");
        let mut tx: Transaction = serde_json::from_slice(&self.root.read(&rel)?)
            .map_err(|e| PkgError::Io(e.to_string()))?;
        for op in tx.ops.iter().rev() {
            let (dest, record) = (
                format!("{PACKAGES_DIR}/{}", op.id),
                format!("{RECORDS_DIR}/{}.json", op.id),
            );
            if op.action == "install" {
                self.root.remove_dir_all(&dest)?;
                self.root.remove_file(&record)?;
            }
            let (old_dir, old_rec) = (
                format!("{TRASH_DIR}/{txid}/{}", op.id),
                format!("{TRASH_DIR}/{txid}/{}.record.json", op.id),
            );
            if self.root.exists(&old_dir) {
                self.root.rename(&old_dir, &dest)?;
            }
            if self.root.exists(&old_rec) {
                self.root.rename(&old_rec, &record)?;
            }
        }
        self.root.remove_dir_all(&format!("{STAGING_DIR}/{txid}"))?;
        self.root.remove_dir_all(&format!("{TRASH_DIR}/{txid}"))?;
        tx.state = TxState::RolledBack;
        self.save_tx(&tx)?;
        Ok(())
    }

    /// Check that every installed file still matches its recorded hash.
    pub fn verify_installed(&self) -> Result<Vec<String>, PkgError> {
        let mut bad = Vec::new();
        for r in self.installed()? {
            for f in &r.files {
                let rel = format!("{PACKAGES_DIR}/{}/{}", r.id, f.path);
                if self
                    .root
                    .read(&rel)
                    .map(|d| sha256_hex(&d) != f.sha256)
                    .unwrap_or(true)
                {
                    bad.push(rel);
                }
            }
        }
        Ok(bad)
    }

    /// The installation manifest, deterministic and free of host paths.
    pub fn export_manifest(&self) -> Result<InstallationManifest, PkgError> {
        Ok(InstallationManifest {
            schema_version: 1,
            filesystem_scope: "guest overlay (not a device)".to_string(),
            packages: self.installed()?,
            transactions: self.transactions()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallationManifest {
    pub schema_version: u32,
    pub filesystem_scope: String,
    pub packages: Vec<InstalledRecord>,
    pub transactions: Vec<Transaction>,
}

/// Build an index entry from an archive, signing it if a signature is supplied.
pub fn index_entry_for(
    archive_name: &str,
    bytes: &[u8],
    signature: Option<String>,
    provenance: &str,
) -> Result<IndexEntry, PkgError> {
    let parsed = parse_archive(bytes)?;
    Ok(IndexEntry {
        id: parsed.header.id.clone(),
        version: parsed.header.version.clone(),
        archive: archive_name.to_string(),
        sha256: sha256_hex(bytes),
        size: bytes.len() as u64,
        description: parsed.header.description.clone(),
        depends: parsed.header.depends.clone(),
        conflicts: parsed.header.conflicts.clone(),
        signature,
        provenance: provenance.to_string(),
    })
}

pub fn hex(bytes: &[u8]) -> String {
    to_hex(bytes)
}
