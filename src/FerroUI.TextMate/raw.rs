use crate::{Error, Result};
use indexmap::IndexMap;
use serde::{
    de::{DeserializeOwned, MapAccess, SeqAccess, Visitor, value::MapAccessDeserializer},
    ser::{Error as _, SerializeMap, SerializeSeq},
    Deserialize, Serialize, Serializer,
};
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawGrammar {
    pub name: Option<String>,
    pub scope_name: String,
    #[serde(default, deserialize_with = "default_if_null")]
    pub file_types: Vec<String>,
    pub first_line_match: Option<String>,
    #[serde(default, deserialize_with = "default_if_null")]
    pub patterns: Vec<RawRule>,
    #[serde(default, deserialize_with = "default_if_null")]
    pub repository: BTreeMap<String, RawRule>,
    /// Tied injection selectors are evaluated in their declaration order.
    #[serde(default, deserialize_with = "default_if_null")]
    pub injections: IndexMap<String, RawRule>,
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
    #[serde(default, deserialize_with = "default_if_null")]
    pub patterns: Vec<RawRule>,
    #[serde(default, deserialize_with = "default_if_null")]
    pub repository: BTreeMap<String, RawRule>,
    #[serde(default, deserialize_with = "capture_map_or_default")]
    pub captures: BTreeMap<String, RawRule>,
    /// `None` inherits shared captures. An empty map suppresses them.
    #[serde(default, deserialize_with = "optional_capture_map")]
    pub begin_captures: Option<BTreeMap<String, RawRule>>,
    /// `None` inherits shared captures. An empty map suppresses them.
    #[serde(default, deserialize_with = "optional_capture_map")]
    pub end_captures: Option<BTreeMap<String, RawRule>>,
    /// `None` inherits shared captures. An empty map suppresses them.
    #[serde(default, deserialize_with = "optional_capture_map")]
    pub while_captures: Option<BTreeMap<String, RawRule>>,
    #[serde(default, deserialize_with = "read_end_pattern_precedence")]
    pub apply_end_pattern_last: bool,
    #[serde(default)]
    pub disabled: Option<serde_json::Value>,
}

fn default_if_null<'de, T: Deserialize<'de> + Default, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<T, D::Error> {
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}

fn read_end_pattern_precedence<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> std::result::Result<bool, D::Error> {
    let value = serde_json::Value::deserialize(d)?;
    Ok(value.as_bool().unwrap_or_else(|| value.as_i64() == Some(1)))
}

type CaptureMap = BTreeMap<String, RawRule>;

fn capture_map_or_default<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> std::result::Result<CaptureMap, D::Error> {
    Ok(optional_capture_map(d)?.unwrap_or_default())
}

fn optional_capture_map<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> std::result::Result<Option<CaptureMap>, D::Error> {
    Ok(Option::<CaptureEntries>::deserialize(d)?.map(|entries| entries.0))
}

struct CaptureEntries(CaptureMap);

impl<'de> Deserialize<'de> for CaptureEntries {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        struct CaptureVisitor;

        impl<'de> Visitor<'de> for CaptureVisitor {
            type Value = CaptureEntries;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a capture map or an ordered list of capture rules")
            }

            fn visit_map<A: MapAccess<'de>>(
                self,
                entries: A,
            ) -> std::result::Result<Self::Value, A::Error> {
                CaptureMap::deserialize(MapAccessDeserializer::new(entries)).map(CaptureEntries)
            }

            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut entries: A,
            ) -> std::result::Result<Self::Value, A::Error> {
                let mut captures = CaptureMap::new();
                while let Some(rule) = entries.next_element()? {
                    // The array form starts at the first group. A map can also name group 0.
                    captures.insert((captures.len() + 1).to_string(), rule);
                }
                Ok(CaptureEntries(captures))
            }
        }

        d.deserialize_any(CaptureVisitor)
    }
}

impl RawGrammar {
    /// External scope dependencies, including nested repositories and captures.
    pub fn dependencies(&self) -> Vec<String> {
        let mut scopes = BTreeSet::new();
        let mut pending: Vec<&RawRule> = self
            .patterns
            .iter()
            .chain(self.repository.values())
            .chain(self.injections.values())
            .collect();
        while let Some(rule) = pending.pop() {
            if let Some(include) = &rule.include {
                if !include.starts_with(['$', '#']) {
                    let scope = include.split('#').next().unwrap_or_default();
                    if !scope.is_empty() && scope != self.scope_name {
                        scopes.insert(scope.to_owned());
                    }
                }
            }
            pending.extend(rule.patterns.iter());
            pending.extend(rule.repository.values());
            pending.extend(rule.captures.values());
            for captures in [
                &rule.begin_captures,
                &rule.end_captures,
                &rule.while_captures,
            ]
            .into_iter()
            .flatten()
            {
                pending.extend(captures.values());
            }
        }
        scopes.into_iter().collect()
    }

    pub fn from_json(text: &str) -> Result<Self> {
        let grammar: Self = read_json_or_plist(text)?;
        if grammar.scope_name.trim().is_empty() {
            return Err(Error::InvalidGrammar("scopeName is empty".into()));
        }
        Ok(grammar)
    }
}

pub(crate) fn read_json_or_plist<T: DeserializeOwned>(text: &str) -> Result<T> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    if text.trim_start().starts_with('<') {
        read_plist(text)
    } else {
        parse_json(text)
    }
}

fn read_plist<T: DeserializeOwned>(source: &str) -> Result<T> {
    // The XML parser and the conversion below both recurse once per level.
    if !ferroui_system::xml_nesting_is_within(source, MAX_PLIST_DEPTH) {
        return Err(Error::Json(format!("plist nesting exceeds {MAX_PLIST_DEPTH} levels")));
    }
    let doc = roxmltree::Document::parse_with_options(
        source,
        roxmltree::ParsingOptions {
            allow_dtd: true,
            ..Default::default()
        },
    )
    .map_err(|e| Error::Json(e.to_string()))?;
    let json = serde_json::to_vec(&PlistValue(doc.root_element()))
        .map_err(|e| Error::Json(e.to_string()))?;
    serde_json::from_slice(&json).map_err(|e| Error::Json(e.to_string()))
}

// The nesting the JSON reader accepts.
const MAX_PLIST_DEPTH: usize = 128;

// A plist dictionary yields entries in source order, just like the JSON input.
struct PlistValue<'a, 'input>(roxmltree::Node<'a, 'input>);

fn plist_text<'a>(node: roxmltree::Node<'a, '_>) -> Cow<'a, str> {
    let mut parts = node
        .children()
        .filter(roxmltree::Node::is_text)
        .filter_map(|part| part.text());
    let mut text = Cow::Borrowed(parts.next().unwrap_or_default());
    for part in parts {
        text.to_mut().push_str(part);
    }
    text
}

impl Serialize for PlistValue<'_, '_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        let node = self.0;
        match node.tag_name().name() {
            "plist" => {
                let value = node
                    .children()
                    .find(roxmltree::Node::is_element)
                    .ok_or_else(|| S::Error::custom("empty plist"))?;
                PlistValue(value).serialize(serializer)
            }
            "dict" => {
                let mut entries = node.children().filter(roxmltree::Node::is_element);
                let mut dictionary = serializer.serialize_map(None)?;
                while let Some(key) = entries.next() {
                    if key.tag_name().name() != "key" {
                        return Err(S::Error::custom("plist dictionary requires a key"));
                    }
                    let value = entries
                        .next()
                        .ok_or_else(|| S::Error::custom("plist dictionary key has no value"))?;
                    dictionary.serialize_entry(plist_text(key).as_ref(), &PlistValue(value))?;
                }
                dictionary.end()
            }
            "array" => {
                let mut sequence = serializer.serialize_seq(None)?;
                for value in node.children().filter(roxmltree::Node::is_element) {
                    sequence.serialize_element(&PlistValue(value))?;
                }
                sequence.end()
            }
            "true" => serializer.serialize_bool(true),
            "false" => serializer.serialize_bool(false),
            "integer" => {
                let value = plist_text(node)
                    .trim()
                    .parse::<i64>()
                    .map_err(S::Error::custom)?;
                serializer.serialize_i64(value)
            }
            "real" => {
                let value = plist_text(node)
                    .trim()
                    .parse::<f64>()
                    .map_err(S::Error::custom)?;
                if !value.is_finite() {
                    return Err(S::Error::custom("non-finite plist real"));
                }
                serializer.serialize_f64(value)
            }
            "string" | "key" | "data" | "date" => serializer.serialize_str(&plist_text(node)),
            other => Err(S::Error::custom(format_args!(
                "unsupported plist value {other}"
            ))),
        }
    }
}

/// Reads JSON with line/block comments and trailing commas used by VS Code assets.
/// Comments are replaced with spaces so JSON error locations remain meaningful.
pub fn read_json(text: &str) -> Result<serde_json::Value> {
    parse_json(text)
}

fn parse_json<T: DeserializeOwned>(text: &str) -> Result<T> {
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
