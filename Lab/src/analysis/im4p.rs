//! Container classification for firmware files: IM4P (DER), AEA1, UDIF disk images and APFS.
//!
//! The reader looks at headers only. It never decompresses a payload and never reads the body of
//! a large file, so an encrypted 9 GB archive is classified without being loaded. A keybag makes
//! a payload "encrypted" for the purposes of the report, because its keys are not present here.

use serde::{Deserialize, Serialize};
use std::io::{Read, Seek, SeekFrom};

const HEAD_LEN: usize = 65_536;
const TAIL_LEN: u64 = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Compression {
    Lzfse,
    Lzvn,
    Complzss,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Im4p {
    pub fourcc: String,
    pub description: String,
    pub payload_bytes: u64,
    pub payload_compression: Compression,
    pub keybag_present: bool,
    pub keybag_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Container {
    Im4p(Im4p),
    Aea1,
    UdifDisk,
    ApfsContainer,
    Unclassified,
}

/// Read one DER element header at `pos`. Returns (tag, content start, content length).
fn der_header(head: &[u8], pos: usize) -> Option<(u8, usize, usize)> {
    let tag = *head.get(pos)?;
    let first = *head.get(pos + 1)?;
    if first & 0x80 == 0 {
        return Some((tag, pos + 2, usize::from(first)));
    }
    let n = usize::from(first & 0x7F);
    if n == 0 || n > 4 {
        return None;
    }
    let mut len = 0usize;
    for k in 0..n {
        len = (len << 8) | usize::from(*head.get(pos + 2 + k)?);
    }
    Some((tag, pos + 2 + n, len))
}

fn compression_of(first: &[u8]) -> Compression {
    if first.len() >= 4 && &first[..3] == b"bvx" && matches!(first[3], b'1' | b'2' | b'-') {
        Compression::Lzfse
    } else if first.len() >= 4 && &first[..3] == b"bvx" && first[3] == b'n' {
        Compression::Lzvn
    } else if first.len() >= 8 && &first[..8] == b"complzss" {
        Compression::Complzss
    } else {
        Compression::None
    }
}

/// Parse an IM4P from its leading bytes plus a reader for the keybag element that follows.
fn parse_im4p<R: Read + Seek>(head: &[u8], r: &mut R) -> Option<Im4p> {
    let (seq_tag, mut pos, seq_len) = der_header(head, 0)?;
    if seq_tag != 0x30 {
        return None;
    }
    let end = pos.checked_add(seq_len)?;
    let (t1, s1, l1) = der_header(head, pos)?;
    if t1 != 0x16 || head.get(s1..s1 + l1)? != b"IM4P" {
        return None;
    }
    pos = s1 + l1;
    let (t2, s2, l2) = der_header(head, pos)?;
    if t2 != 0x16 {
        return None;
    }
    let fourcc = String::from_utf8_lossy(head.get(s2..s2 + l2)?).into_owned();
    pos = s2 + l2;
    let (t3, s3, l3) = der_header(head, pos)?;
    if t3 != 0x16 {
        return None;
    }
    let description = String::from_utf8_lossy(head.get(s3..s3 + l3)?).into_owned();
    pos = s3 + l3;
    let (t4, s4, l4) = der_header(head, pos)?;
    if t4 != 0x04 || s4 + l4 > end {
        return None;
    }
    let payload_first = head.get(s4..(s4 + 8).min(head.len())).unwrap_or(&[]);
    let payload_compression = compression_of(payload_first);

    // The keybag, when present, is the next OCTET STRING after the payload.
    let after = s4 + l4;
    let mut keybag_present = false;
    let mut keybag_bytes = 0u64;
    if after < end {
        r.seek(SeekFrom::Start(after as u64)).ok()?;
        let mut hdr = [0u8; 8];
        let got = r.read(&mut hdr).ok()?;
        if let Some((t5, _, l5)) = der_header(&hdr[..got], 0) {
            if t5 == 0x04 {
                keybag_bytes = l5 as u64;
                keybag_present = l5 > 0;
            }
        }
    }
    Some(Im4p {
        fourcc,
        description,
        payload_bytes: l4 as u64,
        payload_compression,
        keybag_present,
        keybag_bytes,
    })
}

/// Classify a file by its header and, for IM4P, the keybag element. Read-only.
pub fn inspect<R: Read + Seek>(r: &mut R) -> std::io::Result<Container> {
    let size = r.seek(SeekFrom::End(0))?;
    r.seek(SeekFrom::Start(0))?;
    let mut head = vec![0u8; size.min(HEAD_LEN as u64) as usize];
    r.read_exact(&mut head)?;

    if head.len() >= 4 && &head[..4] == b"AEA1" {
        return Ok(Container::Aea1);
    }
    if head.first() == Some(&0x30) {
        if let Some(im4p) = parse_im4p(&head, r) {
            return Ok(Container::Im4p(im4p));
        }
    }
    if head.len() >= 36 && &head[32..36] == b"NXSB" {
        return Ok(Container::ApfsContainer);
    }
    if size >= TAIL_LEN {
        r.seek(SeekFrom::Start(size - TAIL_LEN))?;
        let mut tail = vec![0u8; TAIL_LEN as usize];
        r.read_exact(&mut tail)?;
        if &tail[..4] == b"koly" {
            return Ok(Container::UdifDisk);
        }
    }
    Ok(Container::Unclassified)
}

/// Classify a file on disk. Opens it read-only.
pub fn inspect_file(path: &std::path::Path) -> std::io::Result<Container> {
    let mut f = std::fs::File::open(path)?;
    inspect(&mut f)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    /// Build a DER IM4P with an optional keybag.
    fn im4p_bytes(fourcc: &str, payload: &[u8], keybag: Option<&[u8]>) -> Vec<u8> {
        fn el(tag: u8, body: &[u8]) -> Vec<u8> {
            let mut v = vec![tag];
            if body.len() < 128 {
                v.push(body.len() as u8);
            } else {
                let n = body.len().to_be_bytes();
                let lead = n.iter().position(|&b| b != 0).unwrap_or(7);
                v.push(0x80 | (8 - lead) as u8);
                v.extend(&n[lead..]);
            }
            v.extend(body);
            v
        }
        let mut inner = Vec::new();
        inner.extend(el(0x16, b"IM4P"));
        inner.extend(el(0x16, fourcc.as_bytes()));
        inner.extend(el(0x16, b"test description"));
        inner.extend(el(0x04, payload));
        if let Some(kb) = keybag {
            inner.extend(el(0x04, kb));
        }
        el(0x30, &inner)
    }

    #[test]
    fn unencrypted_lzfse_payload_is_not_marked_encrypted() {
        let mut payload = b"bvx2".to_vec();
        payload.extend([0u8; 64]);
        let bytes = im4p_bytes("dtre", &payload, Some(&[]));
        match inspect(&mut Cursor::new(bytes)).unwrap() {
            Container::Im4p(i) => {
                assert_eq!(i.fourcc, "dtre");
                assert_eq!(i.payload_compression, Compression::Lzfse);
                assert!(!i.keybag_present, "an empty keybag is not a key");
            }
            other => panic!("expected IM4P, got {other:?}"),
        }
    }

    #[test]
    fn keybag_marks_the_payload_as_encrypted() {
        let bytes = im4p_bytes("sepi", &[1, 2, 3, 4], Some(&[9u8; 300]));
        match inspect(&mut Cursor::new(bytes)).unwrap() {
            Container::Im4p(i) => {
                assert!(i.keybag_present);
                assert_eq!(i.keybag_bytes, 300);
                assert_eq!(i.payload_compression, Compression::None);
            }
            other => panic!("expected IM4P, got {other:?}"),
        }
    }

    #[test]
    fn recognises_aea_apfs_and_udif_containers() {
        assert_eq!(
            inspect(&mut Cursor::new(b"AEA1xxxx".to_vec())).unwrap(),
            Container::Aea1
        );
        let mut apfs = vec![0u8; 64];
        apfs[32..36].copy_from_slice(b"NXSB");
        assert_eq!(
            inspect(&mut Cursor::new(apfs)).unwrap(),
            Container::ApfsContainer
        );
        let mut dmg = vec![0u8; 1024];
        dmg[512..516].copy_from_slice(b"koly");
        assert_eq!(inspect(&mut Cursor::new(dmg)).unwrap(), Container::UdifDisk);
        assert_eq!(
            inspect(&mut Cursor::new(vec![0xAAu8; 16])).unwrap(),
            Container::Unclassified
        );
    }

    #[test]
    fn long_form_der_lengths_are_read() {
        let body = vec![5u8; 200];
        let bytes = im4p_bytes("krnl", &body, None);
        match inspect(&mut Cursor::new(bytes)).unwrap() {
            Container::Im4p(i) => assert_eq!(i.payload_bytes, 200),
            other => panic!("expected IM4P, got {other:?}"),
        }
    }
}
