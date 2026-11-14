//! BuildManifest reader and target identity checks.
//!
//! The manifest is the authority for which components a restore uses. Components are listed
//! exactly as the manifest states them. A component is never assumed to exist because its name
//! looks familiar. Digests are kept as recorded; the manifest's digest is SHA-384 of the file.

use super::plist::{self, Value};
use crate::hash::sha256_hex;
use serde::{Deserialize, Serialize};

pub const TARGET_PRODUCT_TYPE: &str = "iPhone16,2";
pub const TARGET_BOARD_CONFIG: &str = "D84AP";
pub const TARGET_CHIP: &str = "T8130";
pub const TARGET_CHIP_ID: u32 = 0x8130;
pub const TARGET_AP_BOARD_ID: u32 = 0x06;
pub const TARGET_PRODUCT_VERSION: &str = "27.0.1";
pub const TARGET_BUILD: &str = "24A446";
pub const TARGET_ARCHITECTURE: &str = "arm64e";
/// The identity used for a full erase install. Its component list is the one HOMEI reports.
pub const ERASE_VARIANT: &str = "Customer Erase Install (IPSW)";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentitySummary {
    pub variant: String,
    pub restore_behavior: String,
    pub board_id_hex: String,
    pub chip_id_hex: String,
    pub device_class: String,
    pub component_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Component {
    pub name: String,
    /// Path relative to the archive root, as the manifest states it.
    pub path: Option<String>,
    pub trusted: bool,
    /// Lower-case hex of the manifest digest. The manifest states it is SHA-384.
    pub digest_hex: String,
    pub loaded_by_iboot: bool,
    pub fud_firmware: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub product_build_version: String,
    pub product_version: String,
    pub supported_product_types: Vec<String>,
    pub identities: Vec<IdentitySummary>,
    pub erase_identity_device_class: String,
    pub erase_identity_chip_id: u32,
    pub erase_identity_board_id: u32,
    pub components: Vec<Component>,
    pub manifest_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestError {
    Plist(String),
    Missing(&'static str),
    NoEraseIdentity,
}

impl std::fmt::Display for ManifestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ManifestError::Plist(m) => write!(f, "manifest is not readable: {m}"),
            ManifestError::Missing(k) => write!(f, "manifest key missing: {k}"),
            ManifestError::NoEraseIdentity => write!(f, "no identity with the erase variant"),
        }
    }
}

impl std::error::Error for ManifestError {}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn str_key<'a>(v: &'a Value, key: &'static str) -> Result<&'a str, ManifestError> {
    v.get(key)
        .and_then(Value::as_str)
        .ok_or(ManifestError::Missing(key))
}

/// Manifests state identifiers either as integers or as strings such as `0x06`; both are accepted.
fn int_key(v: &Value, key: &'static str) -> Result<u32, ManifestError> {
    let parsed = match v.get(key) {
        Some(Value::Integer(i)) => u32::try_from(*i).ok(),
        Some(Value::String(s)) => match s.strip_prefix("0x") {
            Some(hex) => u32::from_str_radix(hex, 16).ok(),
            None => s.parse::<u32>().ok(),
        },
        _ => None,
    };
    parsed.ok_or(ManifestError::Missing(key))
}

fn parse_components(identity: &Value) -> Vec<Component> {
    let Some(manifest) = identity.get("Manifest").and_then(Value::as_dict) else {
        return Vec::new();
    };
    manifest
        .iter()
        .map(|(name, entry)| {
            let info = entry.get("Info");
            let digest = match entry.get("Digest") {
                Some(Value::Data(d)) => hex(d),
                _ => String::new(),
            };
            Component {
                name: name.clone(),
                path: info
                    .and_then(|i| i.get("Path"))
                    .and_then(Value::as_str)
                    .map(str::to_string),
                trusted: entry
                    .get("Trusted")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                digest_hex: digest,
                loaded_by_iboot: info
                    .and_then(|i| i.get("IsLoadedByiBoot"))
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                fud_firmware: info
                    .and_then(|i| i.get("IsFUDFirmware"))
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            }
        })
        .collect()
}

/// Parse a BuildManifest.plist from its text. The raw bytes are hashed for provenance.
pub fn parse(text: &str) -> Result<Manifest, ManifestError> {
    let root = plist::parse(text).map_err(|e| ManifestError::Plist(e.to_string()))?;
    let identities_v = root
        .get("BuildIdentities")
        .and_then(Value::as_array)
        .ok_or(ManifestError::Missing("BuildIdentities"))?;

    let mut identities = Vec::new();
    let mut erase: Option<&Value> = None;
    for id in identities_v {
        let info = id.get("Info").ok_or(ManifestError::Missing("Info"))?;
        let variant = str_key(info, "Variant")?.to_string();
        let device_class = str_key(info, "DeviceClass")?.to_string();
        let component_count = id
            .get("Manifest")
            .and_then(Value::as_dict)
            .map_or(0, <[(String, Value)]>::len);
        if variant == ERASE_VARIANT && erase.is_none() {
            erase = Some(id);
        }
        identities.push(IdentitySummary {
            variant,
            restore_behavior: str_key(info, "RestoreBehavior")?.to_string(),
            board_id_hex: format!("0x{:02x}", int_key(id, "ApBoardID")?),
            chip_id_hex: format!("0x{:04x}", int_key(id, "ApChipID")?),
            device_class,
            component_count,
        });
    }
    let erase = erase.ok_or(ManifestError::NoEraseIdentity)?;
    let erase_info = erase.get("Info").ok_or(ManifestError::Missing("Info"))?;
    let supported = root
        .get("SupportedProductTypes")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();

    Ok(Manifest {
        product_build_version: str_key(&root, "ProductBuildVersion")?.to_string(),
        product_version: str_key(&root, "ProductVersion")?.to_string(),
        supported_product_types: supported,
        identities,
        erase_identity_device_class: str_key(erase_info, "DeviceClass")?.to_string(),
        erase_identity_chip_id: int_key(erase, "ApChipID")?,
        erase_identity_board_id: int_key(erase, "ApBoardID")?,
        components: parse_components(erase),
        manifest_sha256: sha256_hex(text.as_bytes()),
    })
}

/// One identity check: what was expected, what was found and whether they agree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentityCheck {
    pub id: String,
    pub expected: String,
    pub observed: String,
    pub passed: bool,
}

/// Check the manifest against the fixed target. Every check is reported, including passes.
pub fn check_identity(m: &Manifest) -> Vec<IdentityCheck> {
    let mut out = Vec::new();
    let mut push = |id: &str, expected: String, observed: String| {
        let passed = expected == observed;
        out.push(IdentityCheck {
            id: id.to_string(),
            expected,
            observed,
            passed,
        });
    };
    push(
        "product_type_supported",
        TARGET_PRODUCT_TYPE.to_string(),
        if m.supported_product_types
            .iter()
            .any(|p| p == TARGET_PRODUCT_TYPE)
        {
            TARGET_PRODUCT_TYPE.to_string()
        } else {
            "absent".to_string()
        },
    );
    push(
        "build_version",
        TARGET_BUILD.to_string(),
        m.product_build_version.clone(),
    );
    push(
        "product_version",
        TARGET_PRODUCT_VERSION.to_string(),
        m.product_version.clone(),
    );
    push(
        "board_config",
        TARGET_BOARD_CONFIG.to_lowercase(),
        m.erase_identity_device_class.to_lowercase(),
    );
    push(
        "chip_id",
        format!("0x{TARGET_CHIP_ID:04x}"),
        format!("0x{:04x}", m.erase_identity_chip_id),
    );
    push(
        "board_id",
        format!("0x{TARGET_AP_BOARD_ID:02x}"),
        format!("0x{:02x}", m.erase_identity_board_id),
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"<?xml version="1.0"?>
<plist version="1.0"><dict>
<key>BuildIdentities</key><array>
 <dict>
  <key>ApBoardID</key><integer>6</integer>
  <key>ApChipID</key><integer>33072</integer>
  <key>Info</key><dict>
    <key>DeviceClass</key><string>d84ap</string>
    <key>RestoreBehavior</key><string>Erase</string>
    <key>Variant</key><string>Customer Erase Install (IPSW)</string>
  </dict>
  <key>Manifest</key><dict>
    <key>KernelCache</key><dict>
      <key>Digest</key><data>AQI=</data>
      <key>Trusted</key><true/>
      <key>Info</key><dict><key>Path</key><string>kernelcache.release.iphone16</string></dict>
    </dict>
  </dict>
 </dict>
</array>
<key>ProductBuildVersion</key><string>24A446</string>
<key>ProductVersion</key><string>27.0.1</string>
<key>SupportedProductTypes</key><array><string>iPhone16,2</string></array>
</dict></plist>"#;

    #[test]
    fn reads_erase_identity_components_and_identity_checks() {
        let m = parse(SAMPLE).unwrap();
        assert_eq!(m.components.len(), 1);
        assert_eq!(
            m.components[0].path.as_deref(),
            Some("kernelcache.release.iphone16")
        );
        assert_eq!(m.components[0].digest_hex, "0102");
        assert!(m.components[0].trusted);
        assert_eq!(m.identities[0].chip_id_hex, "0x8130");
        let checks = check_identity(&m);
        assert!(checks.iter().all(|c| c.passed), "{checks:?}");
        assert_eq!(checks.len(), 6);
    }

    #[test]
    fn a_wrong_build_fails_its_check_instead_of_passing_silently() {
        let wrong = SAMPLE.replace("24A446", "24B119");
        let m = parse(&wrong).unwrap();
        let failed: Vec<String> = check_identity(&m)
            .iter()
            .filter(|c| !c.passed)
            .map(|c| c.id.clone())
            .collect();
        assert_eq!(failed, vec!["build_version".to_string()]);
    }

    #[test]
    fn requires_the_erase_identity() {
        let no_erase = SAMPLE.replace("Customer Erase Install (IPSW)", "Other");
        assert_eq!(parse(&no_erase), Err(ManifestError::NoEraseIdentity));
    }

    #[test]
    fn target_build_manifest_when_present() {
        let Some(root) = std::env::var("HOMEI_T8130_INPUT").ok() else {
            return;
        };
        let text = std::fs::read_to_string(
            std::path::Path::new(&root).join("manifest/BuildManifest.plist"),
        )
        .unwrap();
        let m = parse(&text).unwrap();
        assert_eq!(m.components.len(), 97);
        assert!(check_identity(&m).iter().all(|c| c.passed));
    }
}
