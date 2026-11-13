//! Read-only device profile model.
//!
//! A profile describes a device by six non-sensitive metadata fields plus a provenance label.
//! It is never produced by this crate talking to hardware: HOMEI has no USB, pairing or network
//! discovery code. Profiles are typed in, imported from a file, or taken from test fixtures.
//!
//! Unknown JSON keys are rejected, so identifiers such as UDIDs or serial numbers cannot be
//! smuggled in as extra fields, and every value is checked against a strict character and shape
//! allow-list.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Where a profile came from. A profile is never "a connected device".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provenance {
    /// Typed in or selected by the user.
    ManualProfile,
    /// Synthetic record used by tests and previews.
    TestFixture,
    /// Six-field result of a separately authorised, read-only observation, imported from a file.
    UserAuthorizedReadOnlyObservation,
    /// Taken from public firmware metadata.
    PublicFirmwareMetadata,
}

impl Provenance {
    /// Human-readable label. No label claims a live connection.
    pub fn label(self) -> &'static str {
        match self {
            Provenance::ManualProfile => "Manual profile",
            Provenance::TestFixture => "Test fixture (not a connected device)",
            Provenance::UserAuthorizedReadOnlyObservation => {
                "User-authorized read-only observation (imported, not live)"
            }
            Provenance::PublicFirmwareMetadata => "Public firmware metadata",
        }
    }

    /// True only for data that came from an authorised observation of a real device.
    pub fn is_observation(self) -> bool {
        matches!(self, Provenance::UserAuthorizedReadOnlyObservation)
    }
}

/// SoC families HOMEI has a sourced product-type mapping for.
///
/// The mapping is a lookup table, not a hardware probe. The tracker marks the chip pairing as
/// partially verified (claims C-159 and C-160), so [`SocFamily::basis`] states that limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SocFamily {
    A17Pro,
    A18Pro,
    A19Pro,
}

impl SocFamily {
    pub fn from_product_type(product_type: &str) -> Option<SocFamily> {
        match product_type {
            "iPhone16,2" => Some(SocFamily::A17Pro),
            "iPhone17,2" => Some(SocFamily::A18Pro),
            "iPhone18,1" => Some(SocFamily::A19Pro),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            SocFamily::A17Pro => "A17 Pro",
            SocFamily::A18Pro => "A18 Pro",
            SocFamily::A19Pro => "A19 Pro",
        }
    }

    /// Evidence limit for the product-type mapping.
    pub fn basis(self) -> &'static str {
        "Product-type to SoC pairing from public firmware manifests and an index; partially verified in the research tracker."
    }
}

/// Why a profile was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileError {
    pub field: &'static str,
    pub reason: &'static str,
}

impl fmt::Display for ProfileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid {}: {}", self.field, self.reason)
    }
}

impl std::error::Error for ProfileError {}

/// The six permitted metadata fields plus provenance. Nothing else can be stored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceProfile {
    pub product_type: String,
    pub product_version: String,
    pub build_version: String,
    pub device_class: Option<String>,
    pub hardware_model: Option<String>,
    pub cpu_architecture: Option<String>,
    pub provenance: Provenance,
}

impl DeviceProfile {
    pub fn soc_family(&self) -> Option<SocFamily> {
        SocFamily::from_product_type(&self.product_type)
    }

    /// Check every field. A rejected profile is evaluated as `unknown`, never as compatible.
    pub fn validate(&self) -> Result<(), ProfileError> {
        check_product_type(&self.product_type)?;
        check_version(&self.product_version)?;
        check_build(&self.build_version)?;
        check_optional("device_class", &self.device_class)?;
        check_optional("hardware_model", &self.hardware_model)?;
        check_optional("cpu_architecture", &self.cpu_architecture)?;
        Ok(())
    }
}

/// Characters allowed in any profile value.
fn charset_ok(s: &str) -> bool {
    s.chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, ',' | '.' | '_' | '-'))
}

/// Heuristic for values that look like device or account identifiers (UDID, ECID, serial, IMEI).
///
/// Used to reject profile values and to guard diagnostic exports. It errs on the side of
/// rejecting.
pub fn looks_like_identifier(s: &str) -> bool {
    let only_hex_40 = s.len() == 40 && s.chars().all(|c| c.is_ascii_hexdigit());
    let udid_shape = s.len() == 25
        && s.as_bytes().get(8) == Some(&b'-')
        && s.chars()
            .filter(|c| *c != '-')
            .all(|c| c.is_ascii_hexdigit());
    let mut run = 0usize;
    let mut longest = 0usize;
    for c in s.chars() {
        if c.is_ascii_digit() {
            run += 1;
            longest = longest.max(run);
        } else {
            run = 0;
        }
    }
    only_hex_40 || udid_shape || longest >= 14 || s.len() > 32
}

fn bad(field: &'static str, reason: &'static str) -> ProfileError {
    ProfileError { field, reason }
}

fn check_product_type(s: &str) -> Result<(), ProfileError> {
    const F: &str = "product_type";
    if s.is_empty() || !charset_ok(s) || looks_like_identifier(s) {
        return Err(bad(F, "unexpected characters or identifier-like value"));
    }
    let Some((name, minor)) = s.split_once(',') else {
        return Err(bad(F, "expected a value like iPhone16,2"));
    };
    let letters = name.trim_end_matches(|c: char| c.is_ascii_digit());
    let major = &name[letters.len()..];
    let ok = !letters.is_empty()
        && letters.chars().all(|c| c.is_ascii_alphabetic())
        && (1..=3).contains(&major.len())
        && (1..=3).contains(&minor.len())
        && minor.chars().all(|c| c.is_ascii_digit());
    if ok {
        Ok(())
    } else {
        Err(bad(F, "expected a value like iPhone16,2"))
    }
}

fn check_version(s: &str) -> Result<(), ProfileError> {
    let parts: Vec<&str> = s.split('.').collect();
    let ok = (2..=3).contains(&parts.len())
        && parts
            .iter()
            .all(|p| (1..=3).contains(&p.len()) && p.chars().all(|c| c.is_ascii_digit()));
    if ok {
        Ok(())
    } else {
        Err(bad("product_version", "expected a value like 27.0.1"))
    }
}

fn check_build(s: &str) -> Result<(), ProfileError> {
    // Shape: 1-3 digits, one uppercase letter, 1-5 digits, optional lowercase letter.
    let b = s.as_bytes();
    let mut i = 0;
    let take = |i: &mut usize, pred: fn(u8) -> bool| {
        let start = *i;
        while *i < b.len() && pred(b[*i]) {
            *i += 1;
        }
        *i - start
    };
    let d1 = take(&mut i, |c| c.is_ascii_digit());
    let up = take(&mut i, |c| c.is_ascii_uppercase());
    let d2 = take(&mut i, |c| c.is_ascii_digit());
    let lo = take(&mut i, |c| c.is_ascii_lowercase());
    let ok = i == b.len() && (1..=3).contains(&d1) && up == 1 && (1..=5).contains(&d2) && lo <= 1;
    if ok {
        Ok(())
    } else {
        Err(bad("build_version", "expected a value like 24A446"))
    }
}

fn check_optional(field: &'static str, value: &Option<String>) -> Result<(), ProfileError> {
    match value {
        None => Ok(()),
        Some(s) if s.is_empty() || s.len() > 32 || !charset_ok(s) || looks_like_identifier(s) => {
            Err(bad(field, "unexpected characters or identifier-like value"))
        }
        Some(_) => Ok(()),
    }
}
