//! Minimal XML property-list reader for the subset used by Apple firmware manifests:
//! dict, array, string, integer, real, true, false, data and date.
//!
//! Binary property lists are rejected, not guessed. Entities are decoded. DOCTYPE, comments and
//! processing instructions are skipped.

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Dict(Vec<(String, Value)>),
    Array(Vec<Value>),
    String(String),
    Integer(i64),
    Real(f64),
    Bool(bool),
    Data(Vec<u8>),
    Date(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlistError {
    NotXml,
    Unexpected(String),
    BadNumber(String),
    BadBase64,
}

impl std::fmt::Display for PlistError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlistError::NotXml => write!(f, "not an XML property list"),
            PlistError::Unexpected(m) => write!(f, "unexpected element: {m}"),
            PlistError::BadNumber(m) => write!(f, "bad number: {m}"),
            PlistError::BadBase64 => write!(f, "bad base64 data"),
        }
    }
}

impl std::error::Error for PlistError {}

impl Value {
    /// Look up a key in a dictionary. Returns None for any other kind of value.
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Dict(items) => items.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) | Value::Date(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Integer(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Value::Array(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_dict(&self) -> Option<&[(String, Value)]> {
        match self {
            Value::Dict(d) => Some(d),
            _ => None,
        }
    }
}

struct Cursor<'a> {
    s: &'a str,
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn rest(&self) -> &'a str {
        &self.s[self.pos..]
    }

    /// Skip whitespace, comments, processing instructions and DOCTYPE declarations.
    fn skip_misc(&mut self) -> Result<(), PlistError> {
        loop {
            let rest = self.rest();
            let trimmed = rest.trim_start();
            self.pos += rest.len() - trimmed.len();
            if self.rest().starts_with("<!--") {
                let end = self.rest().find("-->").ok_or(PlistError::NotXml)?;
                self.pos += end + 3;
            } else if self.rest().starts_with("<?") || self.rest().starts_with("<!") {
                let end = self.rest().find('>').ok_or(PlistError::NotXml)?;
                self.pos += end + 1;
            } else {
                return Ok(());
            }
        }
    }

    /// Read an opening tag. Returns the name and whether it self-closes.
    fn open_tag(&mut self) -> Result<(String, bool), PlistError> {
        self.skip_misc()?;
        let rest = self.rest();
        if !rest.starts_with('<') || rest.starts_with("</") {
            return Err(PlistError::Unexpected(snippet(rest)));
        }
        let end = rest.find('>').ok_or(PlistError::NotXml)?;
        let inner = &rest[1..end];
        let self_closing = inner.ends_with('/');
        let inner = inner.trim_end_matches('/');
        let name = inner
            .split(|c: char| c.is_whitespace())
            .next()
            .unwrap_or("")
            .to_string();
        if name.is_empty() {
            return Err(PlistError::Unexpected(snippet(rest)));
        }
        self.pos += end + 1;
        Ok((name, self_closing))
    }

    fn expect_close(&mut self, name: &str) -> Result<(), PlistError> {
        self.skip_misc()?;
        let want = format!("</{name}>");
        if self.rest().starts_with(&want) {
            self.pos += want.len();
            Ok(())
        } else {
            Err(PlistError::Unexpected(snippet(self.rest())))
        }
    }

    /// Text up to the closing tag of `name`, with entities decoded.
    fn text_until_close(&mut self, name: &str) -> Result<String, PlistError> {
        let close = format!("</{name}>");
        let rest = self.rest();
        let end = rest.find(&close).ok_or(PlistError::NotXml)?;
        let raw = &rest[..end];
        self.pos += end + close.len();
        Ok(decode_entities(raw))
    }

    fn value(&mut self) -> Result<Value, PlistError> {
        let (name, self_closing) = self.open_tag()?;
        match name.as_str() {
            "true" | "false" if self_closing => Ok(Value::Bool(name == "true")),
            "dict" => {
                let mut items = Vec::new();
                if self_closing {
                    return Ok(Value::Dict(items));
                }
                loop {
                    self.skip_misc()?;
                    if self.rest().starts_with("</dict>") {
                        self.pos += "</dict>".len();
                        break;
                    }
                    let (key_name, key_closing) = self.open_tag()?;
                    if key_name != "key" || key_closing {
                        return Err(PlistError::Unexpected("dict key".into()));
                    }
                    let key = self.text_until_close("key")?;
                    let v = self.value()?;
                    items.push((key, v));
                }
                Ok(Value::Dict(items))
            }
            "array" => {
                let mut items = Vec::new();
                if self_closing {
                    return Ok(Value::Array(items));
                }
                loop {
                    self.skip_misc()?;
                    if self.rest().starts_with("</array>") {
                        self.pos += "</array>".len();
                        break;
                    }
                    items.push(self.value()?);
                }
                Ok(Value::Array(items))
            }
            "string" => {
                if self_closing {
                    return Ok(Value::String(String::new()));
                }
                Ok(Value::String(self.text_until_close("string")?))
            }
            "integer" => {
                let t = self.text_until_close("integer")?;
                let t = t.trim();
                let parsed = if let Some(hex) = t.strip_prefix("0x") {
                    i64::from_str_radix(hex, 16)
                } else {
                    t.parse::<i64>()
                };
                parsed
                    .map(Value::Integer)
                    .map_err(|_| PlistError::BadNumber(t.to_string()))
            }
            "real" => {
                let t = self.text_until_close("real")?;
                t.trim()
                    .parse::<f64>()
                    .map(Value::Real)
                    .map_err(|_| PlistError::BadNumber(t))
            }
            "date" => Ok(Value::Date(
                self.text_until_close("date")?.trim().to_string(),
            )),
            "data" => {
                if self_closing {
                    return Ok(Value::Data(Vec::new()));
                }
                let t = self.text_until_close("data")?;
                let cleaned: String = t.chars().filter(|c| !c.is_whitespace()).collect();
                base64_decode(&cleaned).map(Value::Data)
            }
            other => Err(PlistError::Unexpected(other.to_string())),
        }
    }
}

fn snippet(s: &str) -> String {
    s.chars().take(40).collect()
}

fn decode_entities(raw: &str) -> String {
    if !raw.contains('&') {
        return raw.to_string();
    }
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        let tail = &rest[i..];
        match tail.find(';') {
            Some(end) if end <= 10 => {
                let ent = &tail[1..end];
                let decoded = match ent {
                    "amp" => Some('&'),
                    "lt" => Some('<'),
                    "gt" => Some('>'),
                    "quot" => Some('"'),
                    "apos" => Some('\''),
                    _ if ent.starts_with("#x") => u32::from_str_radix(&ent[2..], 16)
                        .ok()
                        .and_then(char::from_u32),
                    _ if ent.starts_with('#') => {
                        ent[1..].parse::<u32>().ok().and_then(char::from_u32)
                    }
                    _ => None,
                };
                match decoded {
                    Some(c) => out.push(c),
                    None => out.push_str(&tail[..=end]),
                }
                rest = &tail[end + 1..];
            }
            _ => {
                out.push('&');
                rest = &tail[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

fn base64_decode(s: &str) -> Result<Vec<u8>, PlistError> {
    fn val(c: u8) -> Option<u32> {
        match c {
            b'A'..=b'Z' => Some(u32::from(c - b'A')),
            b'a'..=b'z' => Some(u32::from(c - b'a') + 26),
            b'0'..=b'9' => Some(u32::from(c - b'0') + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    let bytes: Vec<u8> = s.bytes().filter(|&c| c != b'=').collect();
    if bytes.len() % 4 == 1 {
        return Err(PlistError::BadBase64);
    }
    let mut out = Vec::with_capacity(bytes.len() * 3 / 4);
    for chunk in bytes.chunks(4) {
        let mut acc = 0u32;
        for (i, &c) in chunk.iter().enumerate() {
            acc |= val(c).ok_or(PlistError::BadBase64)? << (18 - 6 * i as u32);
        }
        let n = chunk.len();
        out.push((acc >> 16) as u8);
        if n > 2 {
            out.push((acc >> 8) as u8);
        }
        if n > 3 {
            out.push(acc as u8);
        }
    }
    Ok(out)
}

/// Parse an XML property list.
pub fn parse(text: &str) -> Result<Value, PlistError> {
    if text.trim_start().starts_with("bplist") {
        return Err(PlistError::NotXml);
    }
    let mut c = Cursor { s: text, pos: 0 };
    c.skip_misc()?;
    let (name, self_closing) = c.open_tag()?;
    if name != "plist" {
        return Err(PlistError::Unexpected(name));
    }
    if self_closing {
        return Err(PlistError::Unexpected("empty plist".into()));
    }
    let v = c.value()?;
    c.expect_close("plist")?;
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nested_dict_array_and_scalars() {
        let xml = r#"<?xml version="1.0"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "x.dtd">
<plist version="1.0"><dict>
  <!-- comment -->
  <key>Name</key><string>A &amp; B &#65;</string>
  <key>Count</key><integer>0x1F</integer>
  <key>Flag</key><true/>
  <key>Items</key><array><integer>7</integer><string/></array>
  <key>Blob</key><data>AQID</data>
</dict></plist>"#;
        let v = parse(xml).unwrap();
        assert_eq!(v.get("Name").and_then(Value::as_str), Some("A & B A"));
        assert_eq!(v.get("Count").and_then(Value::as_i64), Some(31));
        assert_eq!(v.get("Flag").and_then(Value::as_bool), Some(true));
        assert_eq!(v.get("Items").unwrap().as_array().unwrap().len(), 2);
        assert_eq!(v.get("Blob"), Some(&Value::Data(vec![1, 2, 3])));
    }

    #[test]
    fn rejects_binary_and_malformed_input() {
        assert_eq!(parse("bplist00xxxx"), Err(PlistError::NotXml));
        assert!(parse("<plist><dict><key>a</key></dict></plist>").is_err());
        assert!(parse("<html></html>").is_err());
    }

    #[test]
    fn parses_the_target_build_manifest_when_present() {
        let root = std::env::var("HOMEI_T8130_INPUT").ok();
        let Some(root) = root else { return };
        let text = std::fs::read_to_string(
            std::path::Path::new(&root).join("manifest/BuildManifest.plist"),
        )
        .unwrap();
        let v = parse(&text).unwrap();
        assert_eq!(
            v.get("ProductBuildVersion").and_then(Value::as_str),
            Some("24A446")
        );
    }
}
