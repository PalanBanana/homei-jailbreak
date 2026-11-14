//! Analysis provenance manager.
//!
//! A run records what it read (by role, file name, size and SHA-256), which tool and schema
//! produced the output, and which steps were done outside this code. Only file names are kept,
//! never a directory, so the record can be published.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

pub const SCHEMA_VERSION: u32 = 1;
pub const TOOL_NAME: &str = "homei-lab analyze";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputRecord {
    pub role: String,
    pub file_name: String,
    pub bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    pub tool: String,
    pub tool_version: String,
    pub schema_version: u32,
    pub analysed_on: String,
    pub inputs: Vec<InputRecord>,
    /// Steps that produced an input and were run by an external tool. They are recorded so a
    /// reader knows the input was not produced by this code.
    pub external_steps: Vec<String>,
}

/// Hash a file in streaming chunks and record its size. Read-only.
pub fn record_input(role: &str, path: &Path) -> io::Result<InputRecord> {
    let mut f = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    let mut total = 0u64;
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        total += n as u64;
    }
    let sha256 = hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    Ok(InputRecord {
        role: role.to_string(),
        file_name: path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        bytes: total,
        sha256,
    })
}

impl Provenance {
    pub fn new(analysed_on: &str) -> Self {
        Self {
            tool: TOOL_NAME.to_string(),
            tool_version: env!("CARGO_PKG_VERSION").to_string(),
            schema_version: SCHEMA_VERSION,
            analysed_on: analysed_on.to_string(),
            inputs: Vec::new(),
            external_steps: Vec::new(),
        }
    }
}
