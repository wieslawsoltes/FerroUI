// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::{error, JsonErrorKind};
use ferrodock_model::core::{DockSerializationError, SerializationResult};
use serde::{
    de::{MapAccess, Visitor},
    Deserialize, Deserializer,
};
use serde_json::value::RawValue;
use std::fmt;
/// A validated JSON number retaining its original lexical representation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JsonNumber(String);
impl JsonNumber {
    /// Validates a numeric JSON token without normalizing its digits or exponent.
    pub fn parse(text: &str) -> SerializationResult<Self> {
        let raw: &RawValue = serde_json::from_str(text).map_err(DockSerializationError::new)?;
        if !matches!(raw.get().as_bytes().first(), Some(b'-' | b'0'..=b'9')) {
            return Err(error(JsonErrorKind::InvalidJson, "Expected a JSON number."));
        }
        Ok(Self(raw.get().into()))
    }
    /// Gets the original number token.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
/// JSON DOM retaining object order, duplicate properties and number tokens.
#[derive(Clone, Debug, PartialEq)]
pub enum JsonValue {
    /// JSON null.
    Null,
    /// Boolean token.
    Bool(bool),
    /// Number token.
    Number(JsonNumber),
    /// Decoded string.
    String(String),
    /// Array contents.
    Array(Vec<JsonValue>),
    /// Ordered properties, including duplicates.
    Object(Vec<(String, JsonValue)>),
}
struct RawObject(Vec<(String, Box<RawValue>)>);
impl<'de> Deserialize<'de> for RawObject {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ObjectVisitor;
        impl<'de> Visitor<'de> for ObjectVisitor {
            type Value = RawObject;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a JSON object")
            }
            fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Self::Value, M::Error> {
                let mut items = Vec::new();
                while let Some(item) = map.next_entry()? {
                    items.push(item);
                }
                Ok(RawObject(items))
            }
        }
        deserializer.deserialize_map(ObjectVisitor)
    }
}
impl JsonValue {
    /// Creates a signed integral JSON value.
    pub fn number(value: i64) -> Self {
        Self::Number(JsonNumber(value.to_string()))
    }
    /// Parses source-compatible JSON while retaining duplicate names and number tokens.
    pub fn parse(text: &str) -> SerializationResult<Self> {
        Self::parse_at(text, 0)
    }
    fn parse_at(text: &str, depth: usize) -> SerializationResult<Self> {
        if depth > 64 {
            return Err(error(
                JsonErrorKind::InvalidJson,
                "The maximum JSON depth was exceeded.",
            ));
        }
        let raw: &RawValue = serde_json::from_str(text).map_err(DockSerializationError::new)?;
        let raw = raw.get();
        if depth >= 64 && matches!(raw.as_bytes()[0], b'[' | b'{') {
            return Err(error(
                JsonErrorKind::InvalidJson,
                "The maximum JSON depth was exceeded.",
            ));
        }
        Ok(match raw.as_bytes()[0] {
            b'n' => Self::Null,
            b't' => Self::Bool(true),
            b'f' => Self::Bool(false),
            b'"' => Self::String(serde_json::from_str(raw).map_err(DockSerializationError::new)?),
            b'[' => {
                let items: Vec<Box<RawValue>> =
                    serde_json::from_str(raw).map_err(DockSerializationError::new)?;
                Self::Array(
                    items
                        .iter()
                        .map(|v| Self::parse_at(v.get(), depth + 1))
                        .collect::<SerializationResult<_>>()?,
                )
            }
            b'{' => {
                let items: RawObject =
                    serde_json::from_str(raw).map_err(DockSerializationError::new)?;
                Self::Object(
                    items
                        .0
                        .iter()
                        .map(|(k, v)| Ok((k.clone(), Self::parse_at(v.get(), depth + 1)?)))
                        .collect::<SerializationResult<_>>()?,
                )
            }
            _ => Self::Number(JsonNumber(raw.into())),
        })
    }
    /// Gets the last property of this name, matching source JsonElement lookup.
    pub fn property(&self, name: &str) -> Option<&Self> {
        if let Self::Object(items) = self {
            items.iter().rev().find(|(k, _)| k == name).map(|(_, v)| v)
        } else {
            None
        }
    }
    /// Writes indented JSON using source default escaping and property order.
    pub fn to_json(&self) -> SerializationResult<String> {
        let mut out = String::new();
        self.write(&mut out, 0)?;
        Ok(out)
    }
    fn write(&self, out: &mut String, depth: usize) -> SerializationResult<()> {
        if depth > 64 {
            return Err(error(
                JsonErrorKind::InvalidJson,
                "The maximum JSON depth was exceeded.",
            ));
        }
        if depth >= 64 && matches!(self, Self::Array(_) | Self::Object(_)) {
            return Err(error(
                JsonErrorKind::InvalidJson,
                "The maximum JSON depth was exceeded.",
            ));
        }
        match self {
            Self::Null => out.push_str("null"),
            Self::Bool(v) => out.push_str(if *v { "true" } else { "false" }),
            Self::Number(v) => out.push_str(v.as_str()),
            Self::String(v) => quote(v, out),
            Self::Array(items) => {
                out.push('[');
                for (i, v) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    out.push('\n');
                    indent(out, depth + 1);
                    v.write(out, depth + 1)?;
                }
                if !items.is_empty() {
                    out.push('\n');
                    indent(out, depth);
                }
                out.push(']');
            }
            Self::Object(items) => {
                out.push('{');
                for (i, (k, v)) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    out.push('\n');
                    indent(out, depth + 1);
                    quote(k, out);
                    out.push_str(": ");
                    v.write(out, depth + 1)?;
                }
                if !items.is_empty() {
                    out.push('\n');
                    indent(out, depth);
                }
                out.push('}');
            }
        }
        Ok(())
    }
}
fn indent(out: &mut String, depth: usize) {
    for _ in 0..depth {
        out.push_str("  ");
    }
}
fn quote(value: &str, out: &mut String) {
    use std::fmt::Write;
    out.push('"');
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{0008}' => out.push_str("\\b"),
            '\u{000c}' => out.push_str("\\f"),
            c if (' '..='~').contains(&c)
                && !matches!(c, '"' | '\'' | '&' | '<' | '>' | '+' | '`') =>
            {
                out.push(c)
            }
            c => {
                let mut units = [0; 2];
                for unit in c.encode_utf16(&mut units) {
                    write!(out, "\\u{unit:04X}").expect("writing a String succeeds");
                }
            }
        }
    }
    out.push('"');
}
