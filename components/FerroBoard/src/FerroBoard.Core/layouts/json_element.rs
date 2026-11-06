//! Ordered JSON DOM bridge. Duplicate properties retain source enumeration order.
use crate::{
    serialization::{JsonBridgeError, JsonPropertyName, StrictJsonValue},
    text::Utf16Text,
};
use serde::{
    Deserialize, Serialize,
    de::{self, MapAccess, SeqAccess, Visitor},
    ser::{SerializeMap, SerializeSeq},
};
#[derive(Clone, Debug, PartialEq)]
pub enum JsonElement {
    Null,
    Bool(bool),
    Number(serde_json::Number),
    String(String),
    /// Escaped UTF-16 that must be retained until GetString is requested.
    DeferredString(Utf16Text),
    /// JsonDocument retains number lexemes, including values outside f64/i64.
    NumberLexeme(String),
    Array(Vec<Self>),
    Object(Vec<(String, Self)>),
    DeferredObject(Vec<(JsonPropertyName, Self)>),
}
impl JsonElement {
    pub fn is_object(&self) -> bool {
        matches!(self, Self::Object(_) | Self::DeferredObject(_))
    }
    pub fn is_string(&self) -> bool {
        matches!(self, Self::String(_) | Self::DeferredString(_))
    }
    pub fn get(&self, name: &str) -> Result<Option<&Self>, JsonBridgeError> {
        match self {
            Self::Object(p) => Ok(p.iter().rev().find(|(n, _)| n == name).map(|(_, v)| v)),
            Self::DeferredObject(p) => {
                for (candidate, value) in p.iter().rev() {
                    if candidate.matches(name)? {
                        return Ok(Some(value));
                    }
                }
                Ok(None)
            }
            _ => Err(JsonBridgeError::InvalidOperation),
        }
    }
    pub fn string(&self) -> Result<Option<Utf16Text>, JsonBridgeError> {
        match self {
            Self::String(s) => Ok(Some(s.as_str().into())),
            Self::DeferredString(s) => StrictJsonValue::valid_string(s).map(Some),
            Self::Null => Ok(None),
            _ => Err(JsonBridgeError::InvalidOperation),
        }
    }
    pub fn int32(&self) -> Result<i32, JsonBridgeError> {
        match self {
            Self::NumberLexeme(raw) => raw.parse().map_err(|_| JsonBridgeError::NumberFormat),
            Self::Number(value) => value
                .as_i64()
                .and_then(|n| i32::try_from(n).ok())
                .ok_or(JsonBridgeError::NumberFormat),
            _ => Err(JsonBridgeError::InvalidOperation),
        }
    }
    pub fn properties(&self) -> Result<JsonProperties<'_>, JsonBridgeError> {
        match self {
            Self::Object(v) => Ok(JsonProperties::Decoded(v.iter())),
            Self::DeferredObject(v) => Ok(JsonProperties::Deferred(v.iter())),
            _ => Err(JsonBridgeError::InvalidOperation),
        }
    }
}
pub enum JsonProperties<'a> {
    Decoded(std::slice::Iter<'a, (String, JsonElement)>),
    Deferred(std::slice::Iter<'a, (JsonPropertyName, JsonElement)>),
}
pub enum JsonName<'a> {
    Decoded(&'a str),
    Deferred(&'a JsonPropertyName),
}
impl JsonName<'_> {
    pub fn text(&self) -> Result<String, JsonBridgeError> {
        match self {
            Self::Decoded(v) => Ok((*v).to_owned()),
            Self::Deferred(v) => v.text.to_utf8().map_err(|_| JsonBridgeError::InvalidString),
        }
    }
}
impl<'a> Iterator for JsonProperties<'a> {
    type Item = (JsonName<'a>, &'a JsonElement);
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Decoded(v) => v
                .next()
                .map(|(name, value)| (JsonName::Decoded(name), value)),
            Self::Deferred(v) => v
                .next()
                .map(|(name, value)| (JsonName::Deferred(name), value)),
        }
    }
}
impl From<StrictJsonValue> for JsonElement {
    fn from(value: StrictJsonValue) -> Self {
        match value {
            StrictJsonValue::Null => Self::Null,
            StrictJsonValue::Bool(v) => Self::Bool(v),
            StrictJsonValue::Number(v) => Self::NumberLexeme(v),
            StrictJsonValue::String(v) => match v.to_utf8() {
                Ok(s) => Self::String(s),
                Err(_) => Self::DeferredString(v),
            },
            StrictJsonValue::Array(v) => Self::Array(v.into_iter().map(Self::from).collect()),
            StrictJsonValue::Object(v) => {
                if v.iter().any(|(name, _)| name.text.to_utf8().is_err()) {
                    Self::DeferredObject(
                        v.into_iter()
                            .map(|(name, value)| (name, Self::from(value)))
                            .collect(),
                    )
                } else {
                    Self::Object(
                        v.into_iter()
                            .map(|(name, value)| {
                                (
                                    name.text.to_utf8().expect("validated property name"),
                                    Self::from(value),
                                )
                            })
                            .collect(),
                    )
                }
            }
        }
    }
}
impl Serialize for JsonElement {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Null => s.serialize_none(),
            Self::Bool(v) => s.serialize_bool(*v),
            Self::Number(n) => n.serialize(s),
            Self::String(v) => s.serialize_str(v),
            Self::DeferredString(_) | Self::DeferredObject(_) => Err(serde::ser::Error::custom(
                "Deferred invalid UTF-16 requires explicit source string access",
            )),
            Self::NumberLexeme(v) => v
                .parse::<serde_json::Number>()
                .map_err(serde::ser::Error::custom)?
                .serialize(s),
            Self::Array(a) => {
                let mut seq = s.serialize_seq(Some(a.len()))?;
                for item in a {
                    seq.serialize_element(item)?;
                }
                seq.end()
            }
            Self::Object(a) => {
                let mut map = s.serialize_map(Some(a.len()))?;
                for (key, value) in a {
                    map.serialize_entry(key, value)?;
                }
                map.end()
            }
        }
    }
}
impl<'de> Deserialize<'de> for JsonElement {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct ElementVisitor;
        impl<'de> Visitor<'de> for ElementVisitor {
            type Value = JsonElement;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("JSON value")
            }
            fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(JsonElement::Null)
            }
            fn visit_bool<E: de::Error>(self, v: bool) -> Result<Self::Value, E> {
                Ok(JsonElement::Bool(v))
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
                Ok(JsonElement::Number(v.into()))
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
                Ok(JsonElement::Number(v.into()))
            }
            fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
                serde_json::Number::from_f64(v)
                    .map(JsonElement::Number)
                    .ok_or_else(|| E::custom("non-finite number"))
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(JsonElement::String(v.into()))
            }
            fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
                Ok(JsonElement::String(v))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut a: A) -> Result<Self::Value, A::Error> {
                let mut items = Vec::new();
                while let Some(v) = a.next_element()? {
                    items.push(v);
                }
                Ok(JsonElement::Array(items))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut a: A) -> Result<Self::Value, A::Error> {
                let mut items = Vec::new();
                while let Some(v) = a.next_entry()? {
                    items.push(v);
                }
                Ok(JsonElement::Object(items))
            }
        }
        d.deserialize_any(ElementVisitor)
    }
}
/// Enable precisely the two source JsonDocument options, leaving other syntax to serde.
pub fn parse(json: &str) -> Result<JsonElement, JsonBridgeError> {
    StrictJsonValue::parse_with_comments(json).map(JsonElement::from)
}
/// Normalize only the source comment/trailing-comma options, preserving string contents and offsets.
pub fn normalize_json_options(json: &str) -> Result<String, String> {
    let mut bytes = json.as_bytes().to_vec();
    let mut i = 0;
    let mut quoted = false;
    let mut depth = 0i32;
    while i < bytes.len() {
        if quoted {
            if bytes[i] == b'\\' {
                i += 2;
                continue;
            }
            if bytes[i] == b'"' {
                quoted = false;
            }
            i += 1;
            continue;
        }
        match bytes[i] {
            b'"' => quoted = true,
            b'{' | b'[' => {
                depth += 1;
                if depth > 64 {
                    return Err("JSON depth exceeds 64".into());
                }
            }
            b'}' | b']' => depth -= 1,
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                while i < bytes.len() && !matches!(bytes[i], b'\r' | b'\n') {
                    bytes[i] = b' ';
                    i += 1;
                }
                continue;
            }
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                bytes[i] = b' ';
                bytes[i + 1] = b' ';
                i += 2;
                while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                    if !matches!(bytes[i], b'\r' | b'\n') {
                        bytes[i] = b' ';
                    }
                    i += 1;
                }
                if i + 1 >= bytes.len() {
                    return Err("Unterminated JSON comment".into());
                }
                bytes[i] = b' ';
                bytes[i + 1] = b' ';
                i += 2;
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    i = 0;
    quoted = false;
    while i < bytes.len() {
        if quoted {
            if bytes[i] == b'\\' {
                i += 2;
                continue;
            }
            if bytes[i] == b'"' {
                quoted = false;
            }
        } else if bytes[i] == b'"' {
            quoted = true;
        } else if bytes[i] == b',' {
            let mut j = i + 1;
            while j < bytes.len() && matches!(bytes[j], b' ' | b'\t' | b'\r' | b'\n') {
                j += 1;
            }
            let mut k = i;
            while k > 0 && matches!(bytes[k - 1], b' ' | b'\t' | b'\r' | b'\n') {
                k -= 1;
            }
            if j < bytes.len()
                && matches!(bytes[j], b']' | b'}')
                && k > 0
                && !matches!(bytes[k - 1], b'[' | b'{' | b':' | b',')
            {
                bytes[i] = b' ';
            }
        }
        i += 1;
    }
    Ok(String::from_utf8(bytes).expect("normalization preserves UTF-8"))
}
