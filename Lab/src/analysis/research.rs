//! Firmware analysis for the fixed target (iPhone16,2, D84AP, T8130, build 24A446).
//!
//! The entry point reads a restore archive's central directory, a BuildManifest, an extracted
//! tree (IM4P files and decompressed payloads, prepared earlier by an external tool) and an
//! optional Ghidra project. It writes nothing. Every output name is relative to the input root,
//! so the bundle contains no local path.

use super::apple_dt::{self, Node};
use super::confidence::{Claim, ClaimTracker, Confidence};
use super::ghidra_meta::{self, GhidraRecord};
use super::im4p::{self, Compression, Container};
use super::macho::{self, MachoSummary};
use super::manifest::{
    self, Component, IdentityCheck, Manifest, TARGET_BOARD_CONFIG, TARGET_BUILD,
    TARGET_PRODUCT_TYPE, TARGET_PRODUCT_VERSION,
};
use super::provenance::{self, Provenance};
use super::zipdir::{self, ZipEntry};
use crate::hash::sha256_file;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha384};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub const TARGET_LABEL: &str = "iPhone16,2 / D84AP / T8130 / iOS 27.0.1 (24A446)";

#[derive(Debug)]
pub enum AnalysisError {
    Io(String),
    Zip(String),
    Manifest(String),
}

impl std::fmt::Display for AnalysisError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnalysisError::Io(m) => write!(f, "file error: {m}"),
            AnalysisError::Zip(m) => write!(f, "archive error: {m}"),
            AnalysisError::Manifest(m) => write!(f, "manifest error: {m}"),
        }
    }
}

impl std::error::Error for AnalysisError {}

impl From<io::Error> for AnalysisError {
    fn from(e: io::Error) -> Self {
        AnalysisError::Io(e.kind().to_string())
    }
}

/// Availability of one manifest component, with its reason kept separate from its name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Availability {
    /// The file is present locally and its format is parsed.
    Analysed,
    /// The file is present and its outer container is recognised, but its contents are not parsed.
    ContainerOnly,
    /// The file is present but its content is encrypted, and its keys are not available here.
    Encrypted,
    /// The file is present but its format is not recognised.
    Unsupported,
    /// The archive lists the component, but no local copy exists in the input tree.
    Unavailable,
    /// The manifest names a path the restore archive does not contain.
    Missing,
    /// The manifest entry states no file path, so there is nothing to look up. This is not a
    /// missing file: the manifest gives no archive location for it.
    NoManifestPath,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetRecord {
    pub label: String,
    pub product_type: String,
    pub board_config: String,
    pub chip: String,
    pub architecture: String,
    pub product_version: String,
    pub build: String,
    pub erase_variant: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchiveRecord {
    pub entry_count: usize,
    pub stored_entries: usize,
    pub deflated_entries: usize,
    pub uncompressed_total_bytes: u64,
    pub entries: Vec<ZipEntry>,
    pub erase_components_missing_from_archive: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentRecord {
    pub name: String,
    pub path: Option<String>,
    pub trusted: bool,
    pub in_archive: bool,
    pub archive_uncompressed_bytes: Option<u64>,
    pub local_file: Option<String>,
    pub local_bytes: Option<u64>,
    pub local_sha256: Option<String>,
    /// Whether the local file's SHA-384 equals the manifest digest. None: no local file.
    pub manifest_digest_matches: Option<bool>,
    pub container: Option<Container>,
    pub availability: Availability,
    pub payload_file: Option<String>,
    pub payload_bytes: Option<u64>,
    pub payload_sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Summary {
    pub erase_components: usize,
    pub analysed: usize,
    pub container_only: usize,
    pub encrypted: usize,
    pub unsupported: usize,
    pub unavailable: usize,
    pub missing: usize,
    pub no_manifest_path: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityState {
    pub role: String,
    pub component: String,
    pub availability: Availability,
    /// `keybag`, `aea_external_keys`, `not_encrypted` or `not_applicable`.
    pub encryption: String,
    pub payload_compression: Option<Compression>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KernelcacheRecord {
    pub payload_file: String,
    pub payload_bytes: u64,
    pub macho: Result<MachoSummary, String>,
    pub fileset_entry_count: usize,
    pub fileset_entries: Vec<String>,
    pub xnu_version: Option<String>,
    pub configuration_token: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegEntry {
    pub address_hex: String,
    pub size_hex: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RangeEntry {
    pub child_base_hex: String,
    pub parent_base_hex: String,
    pub size_hex: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AicRecord {
    pub path: String,
    pub compatible: Vec<String>,
    pub interrupt_cells: Option<u32>,
    pub reg: Vec<RegEntry>,
    pub reg_names_present: bool,
    pub translated_base_hex: Option<String>,
    pub translation_source: Option<String>,
    pub offset_and_stride_properties: Vec<String>,
    pub aic_iack_offset_in_tree: bool,
    pub consumers_single_cell: usize,
    pub consumers_multi_element: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PmgrRecord {
    pub path: String,
    pub compatible: Vec<String>,
    pub reg_windows: usize,
    pub power_domain_names: Vec<String>,
    pub interrupts: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceTreeRecord {
    pub payload_file: String,
    pub payload_bytes: u64,
    pub payload_sha256: String,
    pub node_count: usize,
    pub root_compatible: Vec<String>,
    pub root_model: Option<String>,
    pub root_target_type: Option<String>,
    pub arm_io_compatible: Vec<String>,
    pub arm_io_ranges: Vec<RangeEntry>,
    pub aic: Option<AicRecord>,
    pub pmgr: Option<PmgrRecord>,
    pub aic_timebase_reg: Vec<RegEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bundle {
    pub schema_version: u32,
    pub target: TargetRecord,
    pub identity_checks: Vec<IdentityCheck>,
    pub manifest_identities: Vec<crate::analysis::manifest::IdentitySummary>,
    pub provenance: Provenance,
    pub archive: ArchiveRecord,
    pub components: Vec<ComponentRecord>,
    pub summary: Summary,
    pub security: Vec<SecurityState>,
    pub kernelcache: Option<KernelcacheRecord>,
    pub devicetree: Option<DeviceTreeRecord>,
    pub ghidra: Option<GhidraRecord>,
    pub claims: Vec<Claim>,
}

fn hex_of(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn sha384_file(path: &Path) -> io::Result<Vec<u8>> {
    let mut f = fs::File::open(path)?;
    let mut h = Sha384::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = io::Read::read(&mut f, &mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(h.finalize().to_vec())
}

/// Find a file below `root` by its base name, ignoring `payload` folders. Deterministic: the
/// lexically first match wins, and the search order is sorted.
fn find_by_name(root: &Path, name: &str) -> Option<PathBuf> {
    let mut stack = vec![root.to_path_buf()];
    let mut hits = Vec::new();
    while let Some(dir) = stack.pop() {
        let Ok(rd) = fs::read_dir(&dir) else { continue };
        let mut entries: Vec<PathBuf> = rd.filter_map(|e| e.ok().map(|e| e.path())).collect();
        entries.sort();
        for p in entries {
            if p.is_dir() {
                if p.file_name().is_some_and(|n| n != "payload") {
                    stack.push(p);
                }
            } else if p.file_name().is_some_and(|n| n == name) {
                hits.push(p);
            }
        }
    }
    hits.sort();
    hits.into_iter().next()
}

fn relative(root: &Path, p: &Path) -> String {
    p.strip_prefix(root)
        .unwrap_or(p)
        .to_string_lossy()
        .replace('\\', "/")
}

/// The decompressed payload of a local IM4P: `<dir>/payload/<name>.bin` or `<stem>.bin`.
fn payload_for(local: &Path) -> Option<PathBuf> {
    let dir = local.parent()?.join("payload");
    let base = local.file_name()?.to_string_lossy().into_owned();
    let stem = base.strip_suffix(".im4p").unwrap_or(&base).to_string();
    [format!("{base}.bin"), format!("{stem}.bin")]
        .into_iter()
        .map(|n| dir.join(n))
        .find(|p| p.is_file())
}

fn security_role(file_name: &str) -> Option<&'static str> {
    let n = file_name.to_lowercase();
    if n.starts_with("sep-") {
        Some("SEP firmware")
    } else if n.starts_with("sptm") {
        Some("secure page table monitor")
    } else if n.starts_with("txm") {
        Some("trusted execution monitor")
    } else if n.ends_with(".dmg.aea") {
        Some("system volume (encrypted image)")
    } else if n.ends_with(".dmg") && n.contains("-69915") {
        Some("restore ramdisk")
    } else {
        None
    }
}

/// Scan a kernel payload for its XNU version and configuration token, without reading the rest.
fn kernel_build_strings(bytes: &[u8]) -> (Option<String>, Option<String>) {
    let needle = b"Darwin Kernel Version";
    let Some(at) = bytes.windows(needle.len()).position(|w| w == needle) else {
        return (None, None);
    };
    let end = bytes[at..]
        .iter()
        .take(400)
        .position(|&c| c == 0)
        .unwrap_or(400);
    let line = String::from_utf8_lossy(&bytes[at..at + end]).into_owned();
    let xnu = line.find("xnu-").map(|i| {
        line[i..]
            .split(|c: char| c.is_whitespace() || c == '/')
            .next()
            .unwrap_or("")
            .to_string()
    });
    let token = line.find("RELEASE_").map(|i| {
        line[i..]
            .chars()
            .take_while(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || *c == '_')
            .collect::<String>()
    });
    (xnu, token)
}

fn kernel_record(root: &Path, payload: &Path) -> io::Result<KernelcacheRecord> {
    let bytes = fs::read(payload)?;
    let macho = macho::inspect(&bytes).map_err(|e| e.to_string());
    let (fileset_entries, xnu_version, configuration_token) = match &macho {
        Ok(m) => {
            let (x, t) = kernel_build_strings(&bytes);
            (m.fileset_entries.clone(), x, t)
        }
        Err(_) => {
            let (x, t) = kernel_build_strings(&bytes);
            (Vec::new(), x, t)
        }
    };
    Ok(KernelcacheRecord {
        payload_file: relative(root, payload),
        payload_bytes: bytes.len() as u64,
        fileset_entry_count: fileset_entries.len(),
        fileset_entries,
        macho,
        xnu_version,
        configuration_token,
    })
}

fn reg_entries(node: &Node, parent: &Node) -> Vec<RegEntry> {
    let ac = parent.prop_u32("#address-cells").unwrap_or(2) as usize;
    let sc = parent.prop_u32("#size-cells").unwrap_or(2) as usize;
    node.prop("reg")
        .map(|p| apple_dt::decode_cells(&p.value, ac, sc))
        .unwrap_or_default()
        .into_iter()
        .map(|(a, s)| RegEntry {
            address_hex: format!("0x{a:x}"),
            size_hex: format!("0x{s:x}"),
        })
        .collect()
}

/// Printable ASCII tokens of at least three characters, taken from a blob. Used only to list the
/// names a power-domain blob carries; no record layout is assumed.
fn ascii_tokens(blob: &[u8]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut cur = String::new();
    for &c in blob.iter().chain(std::iter::once(&0u8)) {
        if c.is_ascii_uppercase() || c.is_ascii_digit() || c == b'_' {
            cur.push(c as char);
        } else {
            if cur.len() >= 3 && !out.contains(&cur) {
                out.push(cur.clone());
            }
            cur.clear();
        }
    }
    out
}

fn devicetree_record(root: &Path, payload: &Path) -> Result<DeviceTreeRecord, String> {
    let bytes = fs::read(payload).map_err(|e| e.kind().to_string())?;
    let tree = apple_dt::parse(&bytes).map_err(|e| e.to_string())?;
    let mut nodes = Vec::new();
    tree.walk("/", &mut nodes);

    let root_addr = tree.prop_u32("#address-cells").unwrap_or(2) as usize;
    let arm_io = tree.child("arm-io");
    let arm_io_ac = arm_io
        .and_then(|n| n.prop_u32("#address-cells"))
        .unwrap_or(2) as usize;
    let arm_io_sc = arm_io.and_then(|n| n.prop_u32("#size-cells")).unwrap_or(2) as usize;
    let arm_io_ranges_raw = arm_io
        .and_then(|n| n.prop("ranges"))
        .map(|p| apple_dt::decode_ranges(&p.value, arm_io_ac, root_addr, arm_io_sc))
        .unwrap_or_default();
    let arm_io_ranges = arm_io_ranges_raw
        .iter()
        .map(|&(c, p, s)| RangeEntry {
            child_base_hex: format!("0x{c:x}"),
            parent_base_hex: format!("0x{p:x}"),
            size_hex: format!("0x{s:x}"),
        })
        .collect();

    let aic_path = nodes
        .iter()
        .find(|(p, _)| p == "/arm-io/aic")
        .map(|(_, n)| *n);
    let aic = aic_path.and_then(|n| {
        let aic_phandle = n.prop_u32("AAPL,phandle");
        let reg = reg_entries(n, arm_io?);
        let translated = arm_io.and_then(|a| {
            let reg0 = apple_dt::decode_cells(&n.prop("reg")?.value, arm_io_ac, arm_io_sc)
                .into_iter()
                .next()?;
            apple_dt::translate(&arm_io_ranges_raw, reg0.0, reg0.1).map(|t| (t, a))
        });
        let consumers: Vec<&Node> = nodes
            .iter()
            .map(|(_, c)| *c)
            .filter(|c| {
                aic_phandle.is_some()
                    && c.prop_u32("interrupt-parent") == aic_phandle
                    && c.prop("interrupts").is_some()
            })
            .collect();
        let single = consumers
            .iter()
            .filter(|c| c.prop("interrupts").is_some_and(|p| p.value.len() == 4))
            .count();
        Some(AicRecord {
            path: "/arm-io/aic".to_string(),
            compatible: n.prop_str_list("compatible"),
            interrupt_cells: n.prop_u32("#interrupt-cells"),
            reg,
            reg_names_present: n.prop("reg-names").is_some(),
            translated_base_hex: translated.as_ref().map(|(t, _)| format!("0x{t:x}")),
            translation_source: translated
                .as_ref()
                .map(|_| "first /arm-io ranges entry containing the AIC window".to_string()),
            offset_and_stride_properties: n
                .properties
                .iter()
                .map(|p| p.name.clone())
                .filter(|s| {
                    s.ends_with("-offset")
                        || s.ends_with("-stride")
                        || s.ends_with("-baseaddress")
                        || s.ends_with("-mask")
                })
                .collect(),
            aic_iack_offset_in_tree: n.prop("aic-iack-offset").is_some(),
            consumers_single_cell: single,
            consumers_multi_element: consumers.len() - single,
        })
    });

    let pmgr = nodes
        .iter()
        .find(|(p, _)| p == "/arm-io/pmgr")
        .map(|(_, n)| PmgrRecord {
            path: "/arm-io/pmgr".to_string(),
            compatible: n.prop_str_list("compatible"),
            reg_windows: arm_io.map(|a| reg_entries(n, a).len()).unwrap_or(0),
            power_domain_names: n
                .prop("power-domains")
                .map(|p| ascii_tokens(&p.value))
                .unwrap_or_default(),
            interrupts: n
                .prop("interrupts")
                .map(|p| {
                    p.value
                        .as_chunks::<4>()
                        .0
                        .iter()
                        .map(|c| u32::from_le_bytes(*c))
                        .collect()
                })
                .unwrap_or_default(),
        });

    let timebase = nodes
        .iter()
        .find(|(p, _)| p == "/arm-io/aic-timebase")
        .map(|(_, n)| arm_io.map(|a| reg_entries(n, a)).unwrap_or_default())
        .unwrap_or_default();

    Ok(DeviceTreeRecord {
        payload_file: relative(root, payload),
        payload_bytes: bytes.len() as u64,
        payload_sha256: sha256_file(payload).map_err(|e| e.kind().to_string())?,
        node_count: tree.count_nodes(),
        root_compatible: tree.prop_str_list("compatible"),
        root_model: tree.prop_str("model").map(str::to_string),
        root_target_type: tree.prop_str("target-type").map(str::to_string),
        arm_io_compatible: arm_io
            .map(|a| a.prop_str_list("compatible"))
            .unwrap_or_default(),
        arm_io_ranges,
        aic,
        pmgr,
        aic_timebase_reg: timebase,
    })
}

fn availability_of(
    has_path: bool,
    in_archive: bool,
    local: Option<&Path>,
    container: Option<&Container>,
) -> Availability {
    if !has_path {
        return Availability::NoManifestPath;
    }
    if !in_archive {
        return Availability::Missing;
    }
    if local.is_none() {
        return Availability::Unavailable;
    }
    match container {
        Some(Container::Aea1) => Availability::Encrypted,
        Some(Container::Im4p(i)) if i.keybag_present => Availability::Encrypted,
        Some(Container::Im4p(_)) => Availability::Analysed,
        Some(Container::UdifDisk | Container::ApfsContainer) => Availability::ContainerOnly,
        _ => Availability::Unsupported,
    }
}

fn encryption_label(container: Option<&Container>) -> &'static str {
    match container {
        Some(Container::Aea1) => "aea_external_keys",
        Some(Container::Im4p(i)) if i.keybag_present => "keybag",
        Some(Container::Im4p(_)) => "not_encrypted",
        Some(_) => "not_applicable",
        None => "not_applicable",
    }
}

/// Run the full analysis. `ipsw` is the restore archive, `root` the extracted tree. `ghidra`
/// is an optional project file. `analysed_on` is an ISO date supplied by the caller.
pub fn analyze(
    root: &Path,
    ipsw: &Path,
    ghidra: Option<&Path>,
    analysed_on: &str,
) -> Result<Bundle, AnalysisError> {
    let manifest_path = root.join("manifest/BuildManifest.plist");
    let manifest_text = fs::read_to_string(&manifest_path)?;
    let manifest: Manifest =
        manifest::parse(&manifest_text).map_err(|e| AnalysisError::Manifest(e.to_string()))?;
    let identity_checks = manifest::check_identity(&manifest);

    let mut claims = ClaimTracker::new();
    let mut prov = Provenance::new(analysed_on);
    prov.inputs
        .push(provenance::record_input("build manifest", &manifest_path)?);
    prov.inputs
        .push(provenance::record_input("restore archive", ipsw)?);
    prov.external_steps.push(
        "IM4P payloads were decompressed earlier with an external IM4P tool; the decompressed files are hashed here, not re-derived"
            .to_string(),
    );
    prov.external_steps.push(
        "The device tree payload was converted to JSON earlier with an external tool; this run parses the binary payload directly"
            .to_string(),
    );

    // Archive listing (central directory only, nothing is decompressed).
    let entries = zipdir::list_file(ipsw).map_err(|e| AnalysisError::Zip(e.to_string()))?;
    let names: std::collections::HashSet<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    let stored = entries.iter().filter(|e| e.method == 0).count();
    let deflated = entries.iter().filter(|e| e.method == 8).count();
    let uncompressed_total = entries.iter().map(|e| e.uncompressed_bytes).sum();
    let mut missing_from_archive = Vec::new();
    for c in &manifest.components {
        if let Some(p) = &c.path {
            if !names.contains(p.as_str()) {
                missing_from_archive.push(c.name.clone());
            }
        }
    }
    let archive = ArchiveRecord {
        entry_count: entries.len(),
        stored_entries: stored,
        deflated_entries: deflated,
        uncompressed_total_bytes: uncompressed_total,
        entries: entries.clone(),
        erase_components_missing_from_archive: missing_from_archive.clone(),
    };
    claims.add(
        format!(
            "The restore archive holds {} entries, and {} erase components are missing from it",
            entries.len(),
            missing_from_archive.len()
        ),
        Confidence::Target,
        "restore archive central directory",
        "Listing only. Entry contents were not read or decompressed",
    );

    // Components.
    let mut components = Vec::new();
    let mut security = Vec::new();
    let mut summary = Summary {
        erase_components: manifest.components.len(),
        ..Summary::default()
    };
    for c in &manifest.components {
        let rec = component_record(root, ipsw, &names, &entries, c)?;
        match rec.availability {
            Availability::Analysed => summary.analysed += 1,
            Availability::ContainerOnly => summary.container_only += 1,
            Availability::Encrypted => summary.encrypted += 1,
            Availability::Unsupported => summary.unsupported += 1,
            Availability::Unavailable => summary.unavailable += 1,
            Availability::Missing => summary.missing += 1,
            Availability::NoManifestPath => summary.no_manifest_path += 1,
        }
        if let Some(local) = &rec.local_file {
            let file_name = Path::new(local)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            if let Some(role) = security_role(&file_name) {
                let payload_compression = match &rec.container {
                    Some(Container::Im4p(i)) => Some(i.payload_compression),
                    _ => None,
                };
                security.push(SecurityState {
                    role: role.to_string(),
                    component: c.name.clone(),
                    availability: rec.availability,
                    encryption: encryption_label(rec.container.as_ref()).to_string(),
                    payload_compression,
                });
            }
        }
        components.push(rec);
    }
    security.sort_by(|a, b| a.role.cmp(&b.role).then(a.component.cmp(&b.component)));

    // Component-level claims. Each one is a count or a name list read from the components above.
    let checked: Vec<&ComponentRecord> = components
        .iter()
        .filter(|c| c.manifest_digest_matches.is_some())
        .collect();
    let failed_digest: Vec<&str> = checked
        .iter()
        .filter(|c| c.manifest_digest_matches == Some(false))
        .map(|c| c.name.as_str())
        .collect();
    claims.add(
        format!(
            "{} of {} manifest digests match the local file; {} do not: {}",
            checked.len() - failed_digest.len(),
            checked.len(),
            failed_digest.len(),
            failed_digest.join(", ")
        ),
        Confidence::Target,
        "manifest Digest (SHA-384) compared with the local file of the same path",
        "The mismatching entries name the same path as a non-restore entry. The reason for the difference is not established",
    );
    let encrypted: Vec<&str> = components
        .iter()
        .filter(|c| c.availability == Availability::Encrypted)
        .map(|c| c.name.as_str())
        .collect();
    claims.add(
        format!(
            "{} components carry a keybag and are not analysed: {}",
            encrypted.len(),
            encrypted.join(", ")
        ),
        Confidence::Target,
        "IM4P keybag element of each local file",
        "The keys are not in the restore archive. No decryption was attempted",
    );
    let no_path: Vec<&str> = components
        .iter()
        .filter(|c| c.availability == Availability::NoManifestPath)
        .map(|c| c.name.as_str())
        .collect();
    claims.add(
        format!(
            "{} erase components state no file path in the manifest: {}",
            no_path.len(),
            no_path.join(", ")
        ),
        Confidence::Manifest,
        "BuildManifest.plist, erase identity",
        "Without a path they cannot be located in the archive. Their content is not established",
    );

    // Kernelcache and device tree.
    let kc_comp = manifest.components.iter().find(|c| c.name == "KernelCache");
    let kernelcache = kc_comp
        .and_then(|c| c.path.as_deref())
        .and_then(|p| find_by_name(root, Path::new(p).file_name()?.to_str()?))
        .and_then(|local| payload_for(&local))
        .map(|p| kernel_record(root, &p))
        .transpose()?;
    let dt_comp = manifest.components.iter().find(|c| c.name == "DeviceTree");
    let devicetree = dt_comp
        .and_then(|c| c.path.as_deref())
        .and_then(|p| find_by_name(root, Path::new(p).file_name()?.to_str()?))
        .and_then(|local| payload_for(&local))
        .map(|p| devicetree_record(root, &p))
        .transpose()
        .map_err(AnalysisError::Manifest)?;

    if let Some(k) = &kernelcache {
        match &k.macho {
            Ok(m) => {
                claims.add(
                    format!(
                        "The kernel payload is a Mach-O {} {} image with {} fileset entries",
                        m.architecture, m.filetype, k.fileset_entry_count
                    ),
                    Confidence::Target,
                    "decompressed kernel payload, Mach-O load commands",
                    "Header and load commands only. No code was analysed",
                );
            }
            Err(e) => {
                claims.add(
                    format!("The kernel payload could not be parsed: {e}"),
                    Confidence::Target,
                    "decompressed kernel payload",
                    "The record is kept as an error",
                );
            }
        }
    }

    if let Some(d) = &devicetree {
        claims.add(
            format!(
                "The device tree has {} nodes; its root compatible is {}",
                d.node_count,
                d.root_compatible.join(", ")
            ),
            Confidence::Target,
            "decompressed device tree payload (binary)",
            "Static template. Values that iBoot fills at boot are not present",
        );
        if let Some(aic) = &d.aic {
            claims.add(
                format!(
                    "The AIC node has compatible {} and {} interrupt cell(s)",
                    aic.compatible.join(", "),
                    aic.interrupt_cells
                        .map_or("no".to_string(), |c| c.to_string())
                ),
                Confidence::Target,
                "device tree /arm-io/aic",
                "The one-cell convention is Apple's (see the kernel diagnostic claim)",
            );
            if let Some(t) = &aic.translated_base_hex {
                claims.add(
                    format!(
                        "The AIC window translates to {t} through the first /arm-io ranges entry"
                    ),
                    Confidence::Inferred,
                    "device tree /arm-io/ranges and /arm-io/aic reg, cell widths from the nodes",
                    "The arithmetic is established. Whether iBoot maps the window there is not",
                );
            }
        }
    }

    // Ghidra metadata (optional).
    let ghidra_record = match ghidra {
        Some(g) => Some(ghidra_meta::inspect(
            g,
            &[
                "AppleInterruptControllerV3",
                "AppleT8130PMGR",
                "aic,3",
                "pmgr1,t8130",
            ],
        )?),
        None => None,
    };
    if let Some(g) = &ghidra_record {
        claims.add(
            format!(
                "The Ghidra project file is {} and no function export exists",
                g.project_file_state
            ),
            Confidence::Target,
            "Ghidra project file and repository index",
            "Index presence only. No function counts or cross-references were computed",
        );
    }

    let target = TargetRecord {
        label: TARGET_LABEL.to_string(),
        product_type: TARGET_PRODUCT_TYPE.to_string(),
        board_config: TARGET_BOARD_CONFIG.to_string(),
        chip: manifest::TARGET_CHIP.to_string(),
        architecture: manifest::TARGET_ARCHITECTURE.to_string(),
        product_version: TARGET_PRODUCT_VERSION.to_string(),
        build: TARGET_BUILD.to_string(),
        erase_variant: manifest::ERASE_VARIANT.to_string(),
    };

    Ok(Bundle {
        schema_version: provenance::SCHEMA_VERSION,
        target,
        identity_checks,
        manifest_identities: manifest.identities.clone(),
        provenance: prov,
        archive,
        components,
        summary,
        security,
        kernelcache,
        devicetree,
        ghidra: ghidra_record,
        claims: claims.into_claims(),
    })
}

fn component_record(
    root: &Path,
    ipsw: &Path,
    names: &std::collections::HashSet<&str>,
    entries: &[ZipEntry],
    c: &Component,
) -> Result<ComponentRecord, AnalysisError> {
    let _ = ipsw;
    let in_archive = c.path.as_deref().is_some_and(|p| names.contains(p));
    let archive_bytes = c.path.as_deref().and_then(|p| {
        entries
            .iter()
            .find(|e| e.name == p)
            .map(|e| e.uncompressed_bytes)
    });
    let local = c
        .path
        .as_deref()
        .and_then(|p| Path::new(p).file_name())
        .and_then(|n| n.to_str())
        .and_then(|n| find_by_name(root, n));
    let (local_file, local_bytes, local_sha256, digest_matches, container) = match &local {
        Some(p) => {
            let bytes = fs::metadata(p)?.len();
            let sha = sha256_file(p)?;
            let digest_ok = if c.digest_hex.is_empty() {
                None
            } else {
                Some(hex_of(&sha384_file(p)?) == c.digest_hex)
            };
            let container = im4p::inspect_file(p)?;
            (
                Some(relative(root, p)),
                Some(bytes),
                Some(sha),
                digest_ok,
                Some(container),
            )
        }
        None => (None, None, None, None, None),
    };
    let payload = local.as_ref().and_then(|p| payload_for(p));
    let (payload_file, payload_bytes, payload_sha256) = match &payload {
        Some(p) => (
            Some(relative(root, p)),
            Some(fs::metadata(p)?.len()),
            Some(sha256_file(p)?),
        ),
        None => (None, None, None),
    };
    Ok(ComponentRecord {
        name: c.name.clone(),
        path: c.path.clone(),
        trusted: c.trusted,
        in_archive,
        archive_uncompressed_bytes: archive_bytes,
        local_file,
        local_bytes,
        local_sha256,
        manifest_digest_matches: digest_matches,
        availability: availability_of(
            c.path.is_some(),
            in_archive,
            local.as_deref(),
            container.as_ref(),
        ),
        container,
        payload_file,
        payload_bytes,
        payload_sha256,
    })
}
