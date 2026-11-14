//! Read-only ZIP central-directory reader.
//!
//! It lists entry names, sizes and compression methods. It never decompresses an entry and never
//! writes. ZIP64 records are supported, because a full restore archive is larger than 4 GiB.

use serde::{Deserialize, Serialize};
use std::io::{Read, Seek, SeekFrom};

const EOCD_SIG: u32 = 0x0605_4b50;
const EOCD64_LOCATOR_SIG: u32 = 0x0706_4b50;
const EOCD64_SIG: u32 = 0x0606_4b50;
const CDH_SIG: u32 = 0x0201_4b50;
const EOCD_MIN: usize = 22;
const MAX_COMMENT: u64 = 65_535;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZipEntry {
    pub name: String,
    /// 0 = stored, 8 = deflate, anything else is recorded as is.
    pub method: u16,
    pub compressed_bytes: u64,
    pub uncompressed_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ZipError {
    Io(String),
    NotZip(&'static str),
    Corrupt(&'static str),
}

impl std::fmt::Display for ZipError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ZipError::Io(m) => write!(f, "read failed: {m}"),
            ZipError::NotZip(m) => write!(f, "not a zip archive: {m}"),
            ZipError::Corrupt(m) => write!(f, "corrupt central directory: {m}"),
        }
    }
}

impl std::error::Error for ZipError {}

fn io(e: std::io::Error) -> ZipError {
    ZipError::Io(e.kind().to_string())
}

fn u16_at(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(at..at + 2)?.try_into().ok()?))
}

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

fn u64_at(b: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(b.get(at..at + 8)?.try_into().ok()?))
}

/// Find the last occurrence of a 4-byte little-endian signature in a buffer.
fn rfind_sig(buf: &[u8], sig: u32) -> Option<usize> {
    let s = sig.to_le_bytes();
    (0..buf.len().saturating_sub(3))
        .rev()
        .find(|&i| buf[i..i + 4] == s)
}

/// List every entry of the archive from its central directory.
pub fn list_entries<R: Read + Seek>(r: &mut R) -> Result<Vec<ZipEntry>, ZipError> {
    let len = r.seek(SeekFrom::End(0)).map_err(io)?;
    let tail_len = len.min(MAX_COMMENT + EOCD_MIN as u64);
    r.seek(SeekFrom::Start(len - tail_len)).map_err(io)?;
    let mut tail = vec![0u8; tail_len as usize];
    r.read_exact(&mut tail).map_err(io)?;

    let eocd = rfind_sig(&tail, EOCD_SIG).ok_or(ZipError::NotZip("no end of central directory"))?;
    if tail.len() - eocd < EOCD_MIN {
        return Err(ZipError::Corrupt("end of central directory is truncated"));
    }
    let mut total = u64::from(u16_at(&tail, eocd + 10).unwrap_or(0));
    let mut cd_size = u64::from(u32_at(&tail, eocd + 12).unwrap_or(0));
    let mut cd_offset = u64::from(u32_at(&tail, eocd + 16).unwrap_or(0));

    if total == 0xFFFF || cd_size == 0xFFFF_FFFF || cd_offset == 0xFFFF_FFFF {
        if eocd < 20 {
            return Err(ZipError::Corrupt("ZIP64 locator is missing"));
        }
        let loc = &tail[eocd - 20..eocd];
        if u32_at(loc, 0) != Some(EOCD64_LOCATOR_SIG) {
            return Err(ZipError::Corrupt("ZIP64 locator signature"));
        }
        let rec_at = u64_at(loc, 8).ok_or(ZipError::Corrupt("ZIP64 locator offset"))?;
        r.seek(SeekFrom::Start(rec_at)).map_err(io)?;
        let mut rec = [0u8; 56];
        r.read_exact(&mut rec).map_err(io)?;
        if u32_at(&rec, 0) != Some(EOCD64_SIG) {
            return Err(ZipError::Corrupt("ZIP64 end record signature"));
        }
        total = u64_at(&rec, 32).ok_or(ZipError::Corrupt("ZIP64 entry count"))?;
        cd_size = u64_at(&rec, 40).ok_or(ZipError::Corrupt("ZIP64 directory size"))?;
        cd_offset = u64_at(&rec, 48).ok_or(ZipError::Corrupt("ZIP64 directory offset"))?;
    }

    if cd_size > len {
        return Err(ZipError::Corrupt("directory size exceeds the file"));
    }
    r.seek(SeekFrom::Start(cd_offset)).map_err(io)?;
    let mut cd = vec![0u8; cd_size as usize];
    r.read_exact(&mut cd).map_err(io)?;

    let mut out = Vec::with_capacity(total as usize);
    let mut pos = 0usize;
    for _ in 0..total {
        if u32_at(&cd, pos) != Some(CDH_SIG) {
            return Err(ZipError::Corrupt("entry signature"));
        }
        let method = u16_at(&cd, pos + 10).ok_or(ZipError::Corrupt("entry header"))?;
        let mut csize = u64::from(u32_at(&cd, pos + 20).ok_or(ZipError::Corrupt("entry header"))?);
        let mut usize_ = u64::from(u32_at(&cd, pos + 24).ok_or(ZipError::Corrupt("entry header"))?);
        let name_len = usize::from(u16_at(&cd, pos + 28).ok_or(ZipError::Corrupt("entry header"))?);
        let extra_len =
            usize::from(u16_at(&cd, pos + 30).ok_or(ZipError::Corrupt("entry header"))?);
        let comment_len =
            usize::from(u16_at(&cd, pos + 32).ok_or(ZipError::Corrupt("entry header"))?);
        let name_start = pos + 46;
        let name_end = name_start + name_len;
        let extra_end = name_end + extra_len;
        let next = extra_end + comment_len;
        if next > cd.len() {
            return Err(ZipError::Corrupt("entry runs past the directory"));
        }
        let name = String::from_utf8_lossy(&cd[name_start..name_end]).into_owned();

        if (usize_ == 0xFFFF_FFFF || csize == 0xFFFF_FFFF) && extra_len > 0 {
            // ZIP64 extra field (id 0x0001): fields appear in a fixed order, only when the 32-bit
            // value is saturated.
            let extra = &cd[name_end..extra_end];
            let mut p = 0usize;
            while p + 4 <= extra.len() {
                let id = u16_at(extra, p).unwrap_or(0);
                let size = usize::from(u16_at(extra, p + 2).unwrap_or(0));
                let body = p + 4;
                if id == 0x0001 {
                    let mut q = body;
                    if usize_ == 0xFFFF_FFFF {
                        usize_ = u64_at(extra, q).ok_or(ZipError::Corrupt("ZIP64 size"))?;
                        q += 8;
                    }
                    if csize == 0xFFFF_FFFF {
                        csize = u64_at(extra, q).ok_or(ZipError::Corrupt("ZIP64 size"))?;
                    }
                    break;
                }
                p = body + size;
            }
        }

        out.push(ZipEntry {
            name,
            method,
            compressed_bytes: csize,
            uncompressed_bytes: usize_,
        });
        pos = next;
    }
    Ok(out)
}

/// Convenience wrapper for a file on disk. Opens it read-only.
pub fn list_file(path: &std::path::Path) -> Result<Vec<ZipEntry>, ZipError> {
    let mut f = std::fs::File::open(path).map_err(io)?;
    list_entries(&mut f)
}
