//! Reader for Apple's flattened DeviceTree (the payload of a `dtre` IM4P, little-endian).
//!
//! Layout per node: `u32` property count, `u32` child count, then each property (32-byte NUL
//! padded name, `u32` length whose top bit is a placeholder flag, value padded to 4 bytes), then
//! the children in order. The node name is the `name` property. The reader is read-only and
//! rejects any structure that does not fit the buffer exactly.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Property {
    pub name: String,
    pub value: Vec<u8>,
    pub placeholder: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub properties: Vec<Property>,
    pub children: Vec<Node>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DtError {
    Truncated(usize),
    TooDeep,
    TrailingBytes(usize),
}

impl std::fmt::Display for DtError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DtError::Truncated(at) => write!(f, "device tree truncated at byte {at}"),
            DtError::TooDeep => write!(f, "device tree nests too deeply"),
            DtError::TrailingBytes(n) => write!(f, "{n} bytes after the root node"),
        }
    }
}

impl std::error::Error for DtError {}

const MAX_DEPTH: usize = 64;

fn read_u32(b: &[u8], pos: &mut usize) -> Result<u32, DtError> {
    let s = b.get(*pos..*pos + 4).ok_or(DtError::Truncated(*pos))?;
    *pos += 4;
    Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

fn parse_node(b: &[u8], pos: &mut usize, depth: usize) -> Result<Node, DtError> {
    if depth > MAX_DEPTH {
        return Err(DtError::TooDeep);
    }
    let nprops = read_u32(b, pos)? as usize;
    let nchildren = read_u32(b, pos)? as usize;
    let mut properties = Vec::with_capacity(nprops.min(4096));
    for _ in 0..nprops {
        let name_bytes = b.get(*pos..*pos + 32).ok_or(DtError::Truncated(*pos))?;
        let end = name_bytes.iter().position(|&c| c == 0).unwrap_or(32);
        let name = String::from_utf8_lossy(&name_bytes[..end]).into_owned();
        *pos += 32;
        let raw_len = read_u32(b, pos)?;
        let placeholder = raw_len & 0x8000_0000 != 0;
        let len = (raw_len & 0x7FFF_FFFF) as usize;
        let value = b
            .get(*pos..*pos + len)
            .ok_or(DtError::Truncated(*pos))?
            .to_vec();
        *pos += len.div_ceil(4) * 4;
        if *pos > b.len() {
            return Err(DtError::Truncated(*pos));
        }
        properties.push(Property {
            name,
            value,
            placeholder,
        });
    }
    let mut children = Vec::with_capacity(nchildren.min(4096));
    for _ in 0..nchildren {
        children.push(parse_node(b, pos, depth + 1)?);
    }
    Ok(Node {
        properties,
        children,
    })
}

/// Parse a whole flattened device tree. The root must consume the buffer exactly.
pub fn parse(bytes: &[u8]) -> Result<Node, DtError> {
    let mut pos = 0usize;
    let root = parse_node(bytes, &mut pos, 0)?;
    if pos != bytes.len() {
        return Err(DtError::TrailingBytes(bytes.len() - pos));
    }
    Ok(root)
}

impl Node {
    pub fn name(&self) -> &str {
        self.prop_str("name").unwrap_or("")
    }

    pub fn prop(&self, name: &str) -> Option<&Property> {
        self.properties.iter().find(|p| p.name == name)
    }

    /// A string property, up to its first NUL.
    pub fn prop_str(&self, name: &str) -> Option<&str> {
        let v = &self.prop(name)?.value;
        let end = v.iter().position(|&c| c == 0).unwrap_or(v.len());
        std::str::from_utf8(&v[..end]).ok()
    }

    /// A property holding several NUL-separated strings, such as `compatible`.
    pub fn prop_str_list(&self, name: &str) -> Vec<String> {
        match self.prop(name) {
            None => Vec::new(),
            Some(p) => p
                .value
                .split(|&c| c == 0)
                .filter(|s| !s.is_empty())
                .map(|s| String::from_utf8_lossy(s).into_owned())
                .collect(),
        }
    }

    /// A single 32-bit little-endian value.
    pub fn prop_u32(&self, name: &str) -> Option<u32> {
        let v = &self.prop(name)?.value;
        (v.len() == 4).then(|| u32::from_le_bytes([v[0], v[1], v[2], v[3]]))
    }

    pub fn child(&self, name: &str) -> Option<&Node> {
        self.children.iter().find(|c| c.name() == name)
    }

    /// Total number of nodes in this subtree, including this one.
    pub fn count_nodes(&self) -> usize {
        1 + self.children.iter().map(Node::count_nodes).sum::<usize>()
    }

    /// Walk a `/`-separated path of child names. An empty path returns this node.
    pub fn at(&self, path: &str) -> Option<&Node> {
        path.split('/')
            .filter(|s| !s.is_empty())
            .try_fold(self, |n, part| n.child(part))
    }

    /// Visit every node with its path (`/a/b`). The root has path `/`.
    pub fn walk<'a>(&'a self, path: &str, out: &mut Vec<(String, &'a Node)>) {
        out.push((path.to_string(), self));
        for c in &self.children {
            let p = if path == "/" {
                format!("/{}", c.name())
            } else {
                format!("{path}/{}", c.name())
            };
            c.walk(&p, out);
        }
    }
}

/// Decode a `reg`-style or `ranges`-style list of cells. Each entry is `addr_cells` 32-bit words
/// (most significant first) followed by `size_cells` words. Returns (address, size) pairs.
pub fn decode_cells(value: &[u8], addr_cells: usize, size_cells: usize) -> Vec<(u64, u64)> {
    let stride = (addr_cells + size_cells) * 4;
    if stride == 0 || !value.len().is_multiple_of(stride) {
        return Vec::new();
    }
    let word = |i: usize| {
        u64::from(u32::from_le_bytes([
            value[i * 4],
            value[i * 4 + 1],
            value[i * 4 + 2],
            value[i * 4 + 3],
        ]))
    };
    // Cells are least-significant first in Apple's tree: the first word is the low 32 bits.
    let fold = |start: usize, cells: usize| {
        (0..cells).fold(0u64, |acc, k| acc | (word(start + k) << (32 * k)))
    };
    (0..value.len() / stride)
        .map(|e| {
            let base = e * (addr_cells + size_cells);
            (fold(base, addr_cells), fold(base + addr_cells, size_cells))
        })
        .collect()
}

/// Decode a `ranges` property as (child base, parent base, size) triples.
pub fn decode_ranges(
    value: &[u8],
    child_addr_cells: usize,
    parent_addr_cells: usize,
    size_cells: usize,
) -> Vec<(u64, u64, u64)> {
    let stride = (child_addr_cells + parent_addr_cells + size_cells) * 4;
    if stride == 0 || !value.len().is_multiple_of(stride) {
        return Vec::new();
    }
    let word = |i: usize| {
        u64::from(u32::from_le_bytes([
            value[i * 4],
            value[i * 4 + 1],
            value[i * 4 + 2],
            value[i * 4 + 3],
        ]))
    };
    // Cells are least-significant first in Apple's tree: the first word is the low 32 bits.
    let fold = |start: usize, cells: usize| {
        (0..cells).fold(0u64, |acc, k| acc | (word(start + k) << (32 * k)))
    };
    let per = child_addr_cells + parent_addr_cells + size_cells;
    (0..value.len() / stride)
        .map(|e| {
            let base = e * per;
            (
                fold(base, child_addr_cells),
                fold(base + child_addr_cells, parent_addr_cells),
                fold(base + child_addr_cells + parent_addr_cells, size_cells),
            )
        })
        .collect()
}

/// Translate a child address through a `ranges` list. Returns None when no entry contains the
/// whole window, so a partial match is never reported as a translation.
pub fn translate(ranges: &[(u64, u64, u64)], child_addr: u64, size: u64) -> Option<u64> {
    ranges.iter().find_map(|&(child, parent, span)| {
        let end = child.checked_add(span)?;
        let win_end = child_addr.checked_add(size)?;
        (child_addr >= child && win_end <= end).then(|| parent + (child_addr - child))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prop(name: &str, value: &[u8]) -> Vec<u8> {
        let mut out = vec![0u8; 32];
        out[..name.len()].copy_from_slice(name.as_bytes());
        out.extend((value.len() as u32).to_le_bytes());
        out.extend(value);
        while out.len() % 4 != 0 {
            out.push(0);
        }
        out
    }

    #[test]
    fn parses_nested_nodes_and_properties() {
        let mut b = Vec::new();
        // root: 2 properties, 1 child
        b.extend(2u32.to_le_bytes());
        b.extend(1u32.to_le_bytes());
        b.extend(prop("compatible", b"D84AP\0iPhone16,2\0"));
        b.extend(prop("name", b"root\0"));
        // child: 1 property, 0 children
        b.extend(1u32.to_le_bytes());
        b.extend(0u32.to_le_bytes());
        b.extend(prop("name", b"arm-io\0"));
        let root = parse(&b).unwrap();
        assert_eq!(
            root.prop_str_list("compatible"),
            vec!["D84AP", "iPhone16,2"]
        );
        assert_eq!(root.count_nodes(), 2);
        assert_eq!(root.at("/arm-io").unwrap().name(), "arm-io");
        assert!(root.at("/missing").is_none());
    }

    #[test]
    fn rejects_truncated_and_trailing_data() {
        let mut b = Vec::new();
        b.extend(1u32.to_le_bytes());
        b.extend(0u32.to_le_bytes());
        assert!(matches!(parse(&b), Err(DtError::Truncated(_))));
        let mut ok = Vec::new();
        ok.extend(0u32.to_le_bytes());
        ok.extend(0u32.to_le_bytes());
        ok.extend([0u8; 4]);
        assert_eq!(parse(&ok), Err(DtError::TrailingBytes(4)));
    }

    #[test]
    fn placeholder_flag_is_masked_out_of_the_length() {
        let mut b = Vec::new();
        b.extend(1u32.to_le_bytes());
        b.extend(0u32.to_le_bytes());
        let mut name = vec![0u8; 32];
        name[..3].copy_from_slice(b"abc");
        b.extend(name);
        b.extend((0x8000_0002u32).to_le_bytes());
        b.extend([7u8, 0, 0, 0]);
        let root = parse(&b).unwrap();
        let p = root.prop("abc").unwrap();
        assert!(p.placeholder);
        assert_eq!(p.value, vec![7, 0]);
    }

    #[test]
    fn cells_and_ranges_decode_with_declared_widths() {
        // reg: address 0x1_C100_0000 (low cell first) with two address cells, size 0x1CC000.
        let mut reg = Vec::new();
        for w in [0xC100_0000u32, 0x0000_0001, 0x001C_C000, 0] {
            reg.extend(w.to_le_bytes());
        }
        assert_eq!(decode_cells(&reg, 2, 2), vec![(0x1_C100_0000, 0x1CC000)]);
        assert!(decode_cells(&reg[..12], 2, 2).is_empty());

        let mut one = Vec::new();
        for w in [0u32, 0, 0x1000_0000, 0x2, 0x1000, 0] {
            one.extend(w.to_le_bytes());
        }
        assert!(
            decode_ranges(&one[..20], 2, 2, 2).is_empty(),
            "a partial entry must not decode"
        );
        let r = decode_ranges(&one, 2, 2, 2);
        assert_eq!(r, vec![(0, 0x2_1000_0000, 0x1000)]);
        assert_eq!(translate(&r, 0x100, 0x10), Some(0x2_1000_0100));
        assert_eq!(
            translate(&r, 0xFFF, 0x10),
            None,
            "a window crossing the entry end is refused"
        );
    }
}
