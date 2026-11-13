//! HOMEI core.
//!
//! Portable, offline logic for an independent, research-stage iOS security framework.
//! This crate reads bundled data and answers questions about it. It contains no exploit code,
//! no device communication and no privileged operation, and it cannot return a fake success
//! for one. See `Docs/SAFETY.md`.

pub mod compatibility;
pub mod data;
pub mod device_profile;
pub mod diagnostics;
pub mod evidence;
pub mod recovery;
pub mod research;
pub mod safety;

/// Product name and codename.
pub const PRODUCT_NAME: &str = "HOMEI Jailbreak";
pub const SHORT_NAME: &str = "HOMEI";
pub const CODENAME: &str = "HOMEI-A17";
pub const TAGLINE: &str = "Independent iOS Security and Jailbreak Framework";
pub const STAGE_NOTICE: &str =
    "Research-stage framework. It has no working jailbreak backend and does not modify any device.";
