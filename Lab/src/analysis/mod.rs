//! Firmware analysis engine for the fixed T8130 target.
//!
//! Every parser here reads a file that is already on disk and writes nothing. Each one rejects
//! input it does not understand instead of guessing. Results carry an explicit confidence tag,
//! and every output names files relative to the input root, never by local path.

pub mod apple_dt;
pub mod confidence;
pub mod ghidra_meta;
pub mod im4p;
pub mod macho;
pub mod manifest;
pub mod plist;
pub mod provenance;
pub mod report;
pub mod research;
pub mod zipdir;
