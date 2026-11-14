//! Read-only Mach-O 64-bit metadata: header, segments, fileset entries and the build platform.
//!
//! It reads the load commands of a file that is already on disk. It does not load, relocate or
//! execute anything. The build UUID is deliberately not reported.

use serde::{Deserialize, Serialize};

const MH_MAGIC_64: u32 = 0xFEED_FACF;
const CPU_TYPE_ARM64: u32 = 0x0100_000C;
const CPU_SUBTYPE_MASK: u32 = 0x00FF_FFFF;
const CPU_SUBTYPE_ARM64E: u32 = 2;
const MH_FILESET: u32 = 0x0C;
const LC_SEGMENT_64: u32 = 0x19;
const LC_BUILD_VERSION: u32 = 0x32;
const LC_FILESET_ENTRY: u32 = 0x8000_0035;
const MAX_LOAD_COMMANDS: u32 = 100_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Segment {
    pub name: String,
    pub vmaddr: u64,
    pub vmsize: u64,
    pub fileoff: u64,
    pub filesize: u64,
    pub sections: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MachoSummary {
    pub cputype: String,
    pub cpusubtype_hex: String,
    pub architecture: String,
    pub filetype: String,
    pub load_command_count: u32,
    pub segments: Vec<Segment>,
    pub fileset_entries: Vec<String>,
    pub build_platform: Option<u32>,
    pub minimum_os: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MachoError {
    Truncated,
    NotMacho64,
    TooManyCommands(u32),
    BadCommand(u32),
}

impl std::fmt::Display for MachoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MachoError::Truncated => write!(f, "Mach-O file is truncated"),
            MachoError::NotMacho64 => write!(f, "not a 64-bit little-endian Mach-O file"),
            MachoError::TooManyCommands(n) => write!(f, "{n} load commands is not plausible"),
            MachoError::BadCommand(at) => write!(f, "malformed load command at offset {at}"),
        }
    }
}

impl std::error::Error for MachoError {}

fn u32_at(b: &[u8], at: usize) -> Result<u32, MachoError> {
    let s = b.get(at..at + 4).ok_or(MachoError::Truncated)?;
    Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

fn u64_at(b: &[u8], at: usize) -> Result<u64, MachoError> {
    let s = b.get(at..at + 8).ok_or(MachoError::Truncated)?;
    let mut a = [0u8; 8];
    a.copy_from_slice(s);
    Ok(u64::from_le_bytes(a))
}

fn cstr_at(b: &[u8], at: usize, end: usize) -> String {
    let end = end.min(b.len());
    if at >= end {
        return String::new();
    }
    let slice = &b[at..end];
    let n = slice.iter().position(|&c| c == 0).unwrap_or(slice.len());
    String::from_utf8_lossy(&slice[..n]).into_owned()
}

fn version_string(v: u32) -> String {
    format!("{}.{}.{}", v >> 16, (v >> 8) & 0xFF, v & 0xFF)
}

fn filetype_name(t: u32) -> String {
    match t {
        0x1 => "MH_OBJECT".into(),
        0x2 => "MH_EXECUTE".into(),
        0x6 => "MH_DYLIB".into(),
        MH_FILESET => "MH_FILESET".into(),
        other => format!("filetype 0x{other:x}"),
    }
}

/// Inspect the header and load commands of a 64-bit little-endian Mach-O image.
pub fn inspect(b: &[u8]) -> Result<MachoSummary, MachoError> {
    if u32_at(b, 0)? != MH_MAGIC_64 {
        return Err(MachoError::NotMacho64);
    }
    let cputype = u32_at(b, 4)?;
    let cpusubtype = u32_at(b, 8)?;
    let filetype = u32_at(b, 12)?;
    let ncmds = u32_at(b, 16)?;
    if ncmds > MAX_LOAD_COMMANDS {
        return Err(MachoError::TooManyCommands(ncmds));
    }
    let architecture =
        if cputype == CPU_TYPE_ARM64 && cpusubtype & CPU_SUBTYPE_MASK == CPU_SUBTYPE_ARM64E {
            "arm64e"
        } else if cputype == CPU_TYPE_ARM64 {
            "arm64"
        } else {
            "other"
        };

    let mut segments = Vec::new();
    let mut fileset_entries = Vec::new();
    let mut build_platform = None;
    let mut minimum_os = None;
    let mut at = 32usize;
    for _ in 0..ncmds {
        let cmd = u32_at(b, at)?;
        let size = u32_at(b, at + 4)? as usize;
        if size < 8 || at + size > b.len() {
            return Err(MachoError::BadCommand(at as u32));
        }
        match cmd {
            LC_SEGMENT_64 => {
                segments.push(Segment {
                    name: cstr_at(b, at + 8, at + 24),
                    vmaddr: u64_at(b, at + 24)?,
                    vmsize: u64_at(b, at + 32)?,
                    fileoff: u64_at(b, at + 40)?,
                    filesize: u64_at(b, at + 48)?,
                    sections: u32_at(b, at + 64)?,
                });
            }
            LC_FILESET_ENTRY => {
                // struct fileset_entry_command: cmd, cmdsize, vmaddr, fileoff, entry_id (offset).
                let name_off = u32_at(b, at + 24)? as usize;
                if name_off >= size {
                    return Err(MachoError::BadCommand(at as u32));
                }
                fileset_entries.push(cstr_at(b, at + name_off, at + size));
            }
            LC_BUILD_VERSION => {
                build_platform = Some(u32_at(b, at + 8)?);
                minimum_os = Some(version_string(u32_at(b, at + 12)?));
            }
            _ => {}
        }
        at += size;
    }

    Ok(MachoSummary {
        cputype: format!("0x{cputype:08x}"),
        cpusubtype_hex: format!("0x{cpusubtype:08x}"),
        architecture: architecture.to_string(),
        filetype: filetype_name(filetype),
        load_command_count: ncmds,
        segments,
        fileset_entries,
        build_platform,
        minimum_os,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(ncmds: u32, sizeofcmds: u32) -> Vec<u8> {
        let mut h = Vec::new();
        h.extend(MH_MAGIC_64.to_le_bytes());
        h.extend(CPU_TYPE_ARM64.to_le_bytes());
        h.extend(0x8000_0002u32.to_le_bytes());
        h.extend(MH_FILESET.to_le_bytes());
        h.extend(ncmds.to_le_bytes());
        h.extend(sizeofcmds.to_le_bytes());
        h.extend([0u8; 8]); // flags and reserved: a 64-bit header is 32 bytes
        h
    }

    #[test]
    fn reads_fileset_entries_segments_and_arm64e() {
        // One fileset entry whose name sits at offset 24 of the command.
        let name = b"com.example.kext\0";
        // Layout: cmd, cmdsize, vmaddr, fileoff, entry_id offset (at 24), reserved (at 28), string at 32.
        let fse_size = 32 + name.len().div_ceil(8) * 8;
        let mut cmds = Vec::new();
        cmds.extend(LC_FILESET_ENTRY.to_le_bytes());
        cmds.extend((fse_size as u32).to_le_bytes());
        cmds.extend(0u64.to_le_bytes());
        cmds.extend(0u64.to_le_bytes());
        cmds.extend(32u32.to_le_bytes());
        cmds.extend(0u32.to_le_bytes());
        cmds.extend(name);
        while cmds.len() % 8 != 0 {
            cmds.push(0);
        }
        let mut seg = Vec::new();
        seg.extend(LC_SEGMENT_64.to_le_bytes());
        seg.extend(72u32.to_le_bytes());
        let mut segname = [0u8; 16];
        segname[..6].copy_from_slice(b"__TEXT");
        seg.extend(segname);
        seg.extend(0x1000u64.to_le_bytes());
        seg.extend(0x4000u64.to_le_bytes());
        seg.extend(0u64.to_le_bytes());
        seg.extend(0x4000u64.to_le_bytes());
        seg.extend(7u32.to_le_bytes()); // maxprot
        seg.extend(5u32.to_le_bytes()); // initprot
        seg.extend(5u32.to_le_bytes()); // nsects
        seg.extend(0u32.to_le_bytes()); // flags
        let mut file = header(2, (cmds.len() + seg.len()) as u32);
        file.extend(seg);
        file.extend(cmds);
        let s = inspect(&file).unwrap();
        assert_eq!(s.architecture, "arm64e");
        assert_eq!(s.filetype, "MH_FILESET");
        assert_eq!(s.segments[0].name, "__TEXT");
        assert_eq!(s.segments[0].sections, 5);
        assert_eq!(s.fileset_entries, vec!["com.example.kext"]);
    }

    #[test]
    fn rejects_wrong_magic_and_impossible_command_counts() {
        assert_eq!(inspect(&[0u8; 64]), Err(MachoError::NotMacho64));
        let h = header(MAX_LOAD_COMMANDS + 1, 0);
        assert!(matches!(inspect(&h), Err(MachoError::TooManyCommands(_))));
        // One command announced, but the header ends before it.
        let short = header(1, 0);
        assert_eq!(inspect(&short), Err(MachoError::Truncated));
    }
}
