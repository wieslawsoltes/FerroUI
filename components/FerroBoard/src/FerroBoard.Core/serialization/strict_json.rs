//! JsonDocument/Utf8JsonWriter bridge needed by the original persistence algorithms.
//! Ordered duplicate members, raw number lexemes, and delayed UTF-16 validation are intentional.
use crate::text::Utf16Text;
use std::fmt::Write;
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JsonBridgeError {
    Syntax { byte_offset: usize },
    InvalidOperation,
    InvalidString,
    NumberFormat,
}
impl std::fmt::Display for JsonBridgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Syntax { byte_offset } => write!(f, "Invalid JSON at byte {byte_offset}"),
            Self::InvalidOperation => f.write_str("Unexpected JSON value type"),
            Self::InvalidString => f.write_str("Invalid JSON string encoding"),
            Self::NumberFormat => f.write_str("Invalid or out-of-range JSON number"),
        }
    }
}
impl std::error::Error for JsonBridgeError {}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JsonPropertyName {
    pub text: Utf16Text,
    raw: Vec<u8>,
}
impl JsonPropertyName {
    /// JsonDocument compares the raw prefix and byte lengths before decoding an escaped
    /// name. An unrelated malformed name may therefore be skipped without throwing.
    pub fn matches(&self, name: &str) -> Result<bool, JsonBridgeError> {
        if let Some(escape) = self.raw.iter().position(|&b| b == b'\\') {
            Ok(self.raw.len() > name.len()
                && name.len() > escape
                && self.raw[..escape] == name.as_bytes()[..escape]
                && StrictJsonValue::valid_string(&self.text)? == Utf16Text::from(name))
        } else {
            Ok(self.raw == name.as_bytes())
        }
    }
}
#[derive(Clone, Debug)]
pub enum StrictJsonValue {
    Null,
    Bool(bool),
    Number(String),
    String(Utf16Text),
    Array(Vec<Self>),
    Object(Vec<(JsonPropertyName, Self)>),
}
impl StrictJsonValue {
    /// The native theme source opts into comments and trailing commas; strict callers stay strict.
    pub fn parse_with_comments(text: &str) -> Result<Self, JsonBridgeError> {
        let text = crate::layouts::normalize_json_options(text)
            .map_err(|_| JsonBridgeError::Syntax { byte_offset: 0 })?;
        Self::parse(&text)
    }
    pub fn double(&self) -> Result<f64, JsonBridgeError> {
        match self {
            Self::Number(raw) => raw.parse().map_err(|_| JsonBridgeError::NumberFormat),
            _ => Err(JsonBridgeError::InvalidOperation),
        }
    }
    pub fn parse(text: &str) -> Result<Self, JsonBridgeError> {
        let mut parser = Parser {
            bytes: text.as_bytes(),
            at: 0,
        };
        let value = parser.value(0)?;
        parser.space();
        if parser.at != parser.bytes.len() {
            return parser.fail();
        }
        Ok(value)
    }
    pub fn object(&self) -> Result<&[(JsonPropertyName, Self)], JsonBridgeError> {
        if let Self::Object(v) = self {
            Ok(v)
        } else {
            Err(JsonBridgeError::InvalidOperation)
        }
    }
    pub fn array(&self) -> Result<&[Self], JsonBridgeError> {
        if let Self::Array(v) = self {
            Ok(v)
        } else {
            Err(JsonBridgeError::InvalidOperation)
        }
    }
    pub fn get(&self, name: &str) -> Result<Option<&Self>, JsonBridgeError> {
        for (candidate, value) in self.object()?.iter().rev() {
            if candidate.matches(name)? {
                return Ok(Some(value));
            }
        }
        Ok(None)
    }
    pub fn string(&self) -> Result<Option<Utf16Text>, JsonBridgeError> {
        match self {
            Self::Null => Ok(None),
            Self::String(v) => Ok(Some(Self::valid_string(v)?)),
            _ => Err(JsonBridgeError::InvalidOperation),
        }
    }
    pub fn valid_string(value: &Utf16Text) -> Result<Utf16Text, JsonBridgeError> {
        if char::decode_utf16(value.units().iter().copied()).any(|c| c.is_err()) {
            Err(JsonBridgeError::InvalidString)
        } else {
            Ok(value.clone())
        }
    }
    pub fn boolean(&self) -> Result<bool, JsonBridgeError> {
        if let Self::Bool(value) = self {
            Ok(*value)
        } else {
            Err(JsonBridgeError::InvalidOperation)
        }
    }
    pub fn int32(&self) -> Result<i32, JsonBridgeError> {
        match self {
            Self::Number(raw) => raw.parse().map_err(|_| JsonBridgeError::NumberFormat),
            _ => Err(JsonBridgeError::InvalidOperation),
        }
    }
}
struct Parser<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl Parser<'_> {
    fn fail<T>(&self) -> Result<T, JsonBridgeError> {
        Err(JsonBridgeError::Syntax {
            byte_offset: self.at,
        })
    }
    fn space(&mut self) {
        while self
            .bytes
            .get(self.at)
            .is_some_and(|b| matches!(b, b' ' | b'\t' | b'\n' | b'\r'))
        {
            self.at += 1;
        }
    }
    fn take(&mut self, b: u8) -> bool {
        if self.bytes.get(self.at) == Some(&b) {
            self.at += 1;
            true
        } else {
            false
        }
    }
    fn literal(&mut self, text: &[u8]) -> Result<(), JsonBridgeError> {
        if self.bytes.get(self.at..self.at + text.len()) == Some(text) {
            self.at += text.len();
            Ok(())
        } else {
            self.fail()
        }
    }
    fn value(&mut self, depth: usize) -> Result<StrictJsonValue, JsonBridgeError> {
        self.space();
        match self.bytes.get(self.at).copied() {
            Some(b'n') => {
                self.literal(b"null")?;
                Ok(StrictJsonValue::Null)
            }
            Some(b't') => {
                self.literal(b"true")?;
                Ok(StrictJsonValue::Bool(true))
            }
            Some(b'f') => {
                self.literal(b"false")?;
                Ok(StrictJsonValue::Bool(false))
            }
            Some(b'"') => Ok(StrictJsonValue::String(self.string()?)),
            Some(b'{') => {
                if depth >= 64 {
                    return self.fail();
                }
                self.at += 1;
                let mut values = Vec::new();
                self.space();
                if self.take(b'}') {
                    return Ok(StrictJsonValue::Object(values));
                }
                loop {
                    self.space();
                    let start = self.at + 1;
                    let text = self.string()?;
                    let name = JsonPropertyName {
                        text,
                        raw: self.bytes[start..self.at - 1].to_vec(),
                    };
                    self.space();
                    if !self.take(b':') {
                        return self.fail();
                    }
                    values.push((name, self.value(depth + 1)?));
                    self.space();
                    if self.take(b'}') {
                        break;
                    }
                    if !self.take(b',') {
                        return self.fail();
                    }
                }
                Ok(StrictJsonValue::Object(values))
            }
            Some(b'[') => {
                if depth >= 64 {
                    return self.fail();
                }
                self.at += 1;
                let mut values = Vec::new();
                self.space();
                if self.take(b']') {
                    return Ok(StrictJsonValue::Array(values));
                }
                loop {
                    values.push(self.value(depth + 1)?);
                    self.space();
                    if self.take(b']') {
                        break;
                    }
                    if !self.take(b',') {
                        return self.fail();
                    }
                }
                Ok(StrictJsonValue::Array(values))
            }
            Some(b'-' | b'0'..=b'9') => {
                let start = self.at;
                self.take(b'-');
                if !self.take(b'0') {
                    if !self
                        .bytes
                        .get(self.at)
                        .is_some_and(|b| matches!(b, b'1'..=b'9'))
                    {
                        return self.fail();
                    }
                    self.digits();
                }
                if self.take(b'.') {
                    let at = self.at;
                    self.digits();
                    if self.at == at {
                        return self.fail();
                    }
                }
                if self.take(b'e') || self.take(b'E') {
                    if !self.take(b'+') {
                        self.take(b'-');
                    }
                    let at = self.at;
                    self.digits();
                    if self.at == at {
                        return self.fail();
                    }
                }
                Ok(StrictJsonValue::Number(
                    std::str::from_utf8(&self.bytes[start..self.at])
                        .expect("ASCII number")
                        .into(),
                ))
            }
            _ => self.fail(),
        }
    }
    fn digits(&mut self) {
        while self.bytes.get(self.at).is_some_and(u8::is_ascii_digit) {
            self.at += 1;
        }
    }
    fn string(&mut self) -> Result<Utf16Text, JsonBridgeError> {
        if !self.take(b'"') {
            return self.fail();
        }
        let mut units = Vec::new();
        let mut start = self.at;
        loop {
            let Some(&b) = self.bytes.get(self.at) else {
                return self.fail();
            };
            if b == b'"' || b == b'\\' {
                units.extend(
                    std::str::from_utf8(&self.bytes[start..self.at])
                        .expect("input is UTF-8")
                        .encode_utf16(),
                );
                self.at += 1;
                if b == b'"' {
                    return Ok(Utf16Text::from_units(units));
                }
                let Some(&escape) = self.bytes.get(self.at) else {
                    return self.fail();
                };
                self.at += 1;
                let unit = match escape {
                    b'"' => 34,
                    b'\\' => 92,
                    b'/' => 47,
                    b'b' => 8,
                    b'f' => 12,
                    b'n' => 10,
                    b'r' => 13,
                    b't' => 9,
                    b'u' => {
                        let mut unit = 0u16;
                        for _ in 0..4 {
                            let Some(&digit) = self.bytes.get(self.at) else {
                                return self.fail();
                            };
                            let value = match digit {
                                b'0'..=b'9' => digit - b'0',
                                b'a'..=b'f' => digit - b'a' + 10,
                                b'A'..=b'F' => digit - b'A' + 10,
                                _ => return self.fail(),
                            };
                            unit = (unit << 4) | u16::from(value);
                            self.at += 1;
                        }
                        unit
                    }
                    _ => return self.fail(),
                };
                units.push(unit);
                start = self.at;
            } else if b < 32 {
                return self.fail();
            } else {
                self.at += 1;
            }
        }
    }
}
/// Default JavaScriptEncoder BasicLatin policy used by Utf8JsonWriter. Invalid UTF-16 is replaced.
pub fn write_json_string(output: &mut String, value: &Utf16Text) {
    output.push('"');
    for scalar in char::decode_utf16(value.units().iter().copied()) {
        let c = scalar.unwrap_or('\u{fffd}');
        match c {
            '\u{8}' => output.push_str("\\b"),
            '\t' => output.push_str("\\t"),
            '\n' => output.push_str("\\n"),
            '\u{c}' => output.push_str("\\f"),
            '\r' => output.push_str("\\r"),
            '\\' => output.push_str("\\\\"),
            ' '..='~' if !matches!(c, '"' | '&' | '\'' | '+' | '<' | '>' | '`') => output.push(c),
            _ => {
                let mut units = [0; 2];
                for unit in c.encode_utf16(&mut units) {
                    write!(output, "\\u{unit:04X}").expect("String writes cannot fail");
                }
            }
        }
    }
    output.push('"');
}
