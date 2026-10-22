use crate::{Error, Result};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawGrammar {
    pub name: Option<String>,
    pub scope_name: String,
    #[serde(default)]
    pub file_types: Vec<String>,
    pub first_line_match: Option<String>,
    #[serde(default)]
    pub patterns: Vec<RawRule>,
    #[serde(default)]
    pub repository: BTreeMap<String, RawRule>,
    #[serde(default)]
    pub injections: BTreeMap<String, RawRule>,
    pub injection_selector: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawRule {
    pub name: Option<String>,
    pub content_name: Option<String>,
    #[serde(rename = "match")]
    pub match_pattern: Option<String>,
    pub begin: Option<String>,
    pub end: Option<String>,
    #[serde(rename = "while")]
    pub while_pattern: Option<String>,
    pub include: Option<String>,
    #[serde(default)]
    pub patterns: Vec<RawRule>,
    #[serde(default)]
    pub repository: BTreeMap<String, RawRule>,
    #[serde(default)]
    pub captures: BTreeMap<String, RawRule>,
    #[serde(default)]
    pub begin_captures: BTreeMap<String, RawRule>,
    #[serde(default)]
    pub end_captures: BTreeMap<String, RawRule>,
    #[serde(default)]
    pub while_captures: BTreeMap<String, RawRule>,
    #[serde(default, deserialize_with = "bool_or_number")]
    pub apply_end_pattern_last: bool,
    #[serde(default)]
    pub disabled: Option<serde_json::Value>,
}

fn bool_or_number<'de, D: serde::Deserializer<'de>>(d: D) -> std::result::Result<bool, D::Error> {
    let value = serde_json::Value::deserialize(d)?;
    Ok(value
        .as_bool()
        .unwrap_or_else(|| value.as_i64().is_some_and(|n| n != 0)))
}

impl RawGrammar {
    pub fn from_json(text: &str) -> Result<Self> {
        let grammar: Self = serde_json::from_value(if text.trim_start().starts_with('<') {
            read_plist(text)?
        } else {
            read_json(text)?
        })
        .map_err(|e| Error::Json(e.to_string()))?;
        if grammar.scope_name.trim().is_empty() {
            return Err(Error::InvalidGrammar("scopeName is empty".into()));
        }
        Ok(grammar)
    }
}

fn read_plist(source: &str) -> Result<serde_json::Value> {
    let doc = roxmltree::Document::parse_with_options(
        source,
        roxmltree::ParsingOptions {
            allow_dtd: true,
            ..Default::default()
        },
    )
    .map_err(|e| Error::Json(e.to_string()))?;
    fn convert(node: roxmltree::Node<'_, '_>) -> Result<serde_json::Value> {
        use serde_json::Value;
        Ok(match node.tag_name().name() {
            "plist" => convert(
                node.children()
                    .find(roxmltree::Node::is_element)
                    .ok_or_else(|| Error::Json("empty plist".into()))?,
            )?,
            "dict" => {
                let mut entries = node.children().filter(roxmltree::Node::is_element);
                let mut result = serde_json::Map::new();
                while let Some(key) = entries.next() {
                    if key.tag_name().name() != "key" {
                        return Err(Error::Json("plist dictionary requires a key".into()));
                    }
                    let value = entries
                        .next()
                        .ok_or_else(|| Error::Json("plist dictionary key has no value".into()))?;
                    result.insert(key.text().unwrap_or_default().into(), convert(value)?);
                }
                Value::Object(result)
            }
            "array" => Value::Array(
                node.children()
                    .filter(roxmltree::Node::is_element)
                    .map(convert)
                    .collect::<Result<_>>()?,
            ),
            "true" => Value::Bool(true),
            "false" => Value::Bool(false),
            "integer" => Value::Number(
                node.text()
                    .unwrap_or_default()
                    .parse::<i64>()
                    .map_err(|e| Error::Json(e.to_string()))?
                    .into(),
            ),
            "string" | "key" => Value::String(node.text().unwrap_or_default().into()),
            other => return Err(Error::Json(format!("unsupported plist value {other}"))),
        })
    }
    convert(doc.root_element())
}

/// Reads JSON with line/block comments and trailing commas used by VS Code assets.
/// Comments are replaced with spaces so JSON error locations remain meaningful.
pub fn read_json(text: &str) -> Result<serde_json::Value> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut bytes = text.as_bytes().to_vec();
    let mut pos = 0;
    let mut string = false;
    while pos < bytes.len() {
        if string {
            if bytes[pos] == b'\\' {
                pos += 2;
                continue;
            }
            if bytes[pos] == b'"' {
                string = false;
            }
            pos += 1;
            continue;
        }
        if bytes[pos] == b'"' {
            string = true;
            pos += 1;
            continue;
        }
        if bytes.get(pos..pos + 2) == Some(b"//") {
            while pos < bytes.len() && bytes[pos] != b'\n' {
                bytes[pos] = b' ';
                pos += 1;
            }
        } else if bytes.get(pos..pos + 2) == Some(b"/*") {
            bytes[pos] = b' ';
            bytes[pos + 1] = b' ';
            pos += 2;
            let mut closed = false;
            while pos < bytes.len() {
                if bytes.get(pos..pos + 2) == Some(b"*/") {
                    bytes[pos] = b' ';
                    bytes[pos + 1] = b' ';
                    pos += 2;
                    closed = true;
                    break;
                }
                if bytes[pos] != b'\n' && bytes[pos] != b'\r' {
                    bytes[pos] = b' ';
                }
                pos += 1;
            }
            if !closed {
                return Err(Error::Json("unterminated block comment".into()));
            }
        } else {
            pos += 1;
        }
    }
    pos = 0;
    string = false;
    while pos < bytes.len() {
        if string {
            if bytes[pos] == b'\\' {
                pos += 2;
                continue;
            }
            if bytes[pos] == b'"' {
                string = false;
            }
        } else if bytes[pos] == b'"' {
            string = true;
        } else if bytes[pos] == b',' {
            let mut next = pos + 1;
            while next < bytes.len() && bytes[next].is_ascii_whitespace() {
                next += 1;
            }
            if matches!(bytes.get(next), Some(b'}' | b']')) {
                bytes[pos] = b' ';
            }
        }
        pos += 1;
    }
    serde_json::from_slice(&bytes).map_err(|e| Error::Json(e.to_string()))
}
