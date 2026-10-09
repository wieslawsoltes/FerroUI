//! The JSON the type model is written in (`.xamlmeta`, RFC 8259): a value,
//! its reader and its writer. The writer is deterministic: the same value
//! gives the same text.
//!
//! The compiler has a reader of its own for the part of the file it reads
//! (`rust_emitter::XamlMetadata`, private to it); this one keeps numbers,
//! which the type model has (the values of enumeration members).

/// A JSON value. An object keeps the order of its members.
#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    /// A number without fraction and exponent that fits `i64`.
    Integer(i64),
    /// Any other number.
    Float(f64),
    String(String),
    Array(Vec<Json>),
    Object(Vec<(String, Json)>),
}

impl Json {
    /// A string value.
    pub fn string(text: &str) -> Json {
        Json::String(text.to_string())
    }

    /// An object of the members whose value is not left out (see [`Members`]).
    pub fn object(members: Members) -> Json {
        Json::Object(members.0)
    }

    /// Reads a value from `text`.
    pub fn parse(text: &str) -> Result<Json, String> {
        let mut parser = Parser { text: text.as_bytes(), at: 0 };
        let value = parser.value(0)?;
        parser.whitespace();
        if parser.at != parser.text.len() {
            return Err(parser.error("text after the value"));
        }
        Ok(value)
    }

    /// The value as text: two spaces of indentation, one member or item per line, a line
    /// feed at the end.
    pub fn to_text(&self) -> String {
        let mut text = String::new();
        self.write(&mut text, 0);
        text.push('\n');
        text
    }

    fn write(&self, text: &mut String, depth: usize) {
        match self {
            Json::Null => text.push_str("null"),
            Json::Bool(value) => text.push_str(if *value { "true" } else { "false" }),
            Json::Integer(value) => text.push_str(&value.to_string()),
            Json::Float(value) => {
                if value.is_finite() {
                    let written = value.to_string();
                    text.push_str(&written);
                    if !written.contains(['.', 'e', 'E']) {
                        text.push_str(".0");
                    }
                } else {
                    text.push_str("null");
                }
            }
            Json::String(value) => write_string(text, value),
            Json::Array(items) => {
                if items.is_empty() {
                    text.push_str("[]");
                    return;
                }
                text.push('[');
                for (index, item) in items.iter().enumerate() {
                    text.push_str(if index == 0 { "\n" } else { ",\n" });
                    indent(text, depth + 1);
                    item.write(text, depth + 1);
                }
                text.push('\n');
                indent(text, depth);
                text.push(']');
            }
            Json::Object(members) => {
                if members.is_empty() {
                    text.push_str("{}");
                    return;
                }
                text.push('{');
                for (index, (name, value)) in members.iter().enumerate() {
                    text.push_str(if index == 0 { "\n" } else { ",\n" });
                    indent(text, depth + 1);
                    write_string(text, name);
                    text.push_str(": ");
                    value.write(text, depth + 1);
                }
                text.push('\n');
                indent(text, depth);
                text.push('}');
            }
        }
    }
}

fn indent(text: &mut String, depth: usize) {
    for _ in 0..depth {
        text.push_str("  ");
    }
}

fn write_string(text: &mut String, value: &str) {
    text.push('"');
    for character in value.chars() {
        match character {
            '"' => text.push_str("\\\""),
            '\\' => text.push_str("\\\\"),
            '\n' => text.push_str("\\n"),
            '\r' => text.push_str("\\r"),
            '\t' => text.push_str("\\t"),
            c if (c as u32) < 0x20 => text.push_str(&format!("\\u{:04x}", c as u32)),
            c => text.push(c),
        }
    }
    text.push('"');
}

/// The members of an object being written: a member whose value is the default of its
/// kind (an empty list, `false`, nothing) is left out, and read back as that default.
#[derive(Default)]
pub struct Members(Vec<(String, Json)>);

impl Members {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    /// A member that is always written.
    pub fn always(mut self, name: &str, value: Json) -> Self {
        self.0.push((name.to_string(), value));
        self
    }

    /// A text member, always written.
    pub fn text(self, name: &str, value: &str) -> Self {
        self.always(name, Json::string(value))
    }

    /// A text member, left out when empty.
    pub fn text_or_empty(self, name: &str, value: &str) -> Self {
        if value.is_empty() {
            self
        } else {
            self.text(name, value)
        }
    }

    /// A member that is left out when there is no value.
    pub fn optional(self, name: &str, value: Option<Json>) -> Self {
        match value {
            Some(value) => self.always(name, value),
            None => self,
        }
    }

    /// A text member that is left out when there is no value.
    pub fn optional_text(self, name: &str, value: &Option<String>) -> Self {
        self.optional(name, value.as_deref().map(Json::string))
    }

    /// A flag, left out when `false`.
    pub fn flag(self, name: &str, value: bool) -> Self {
        if value {
            self.always(name, Json::Bool(true))
        } else {
            self
        }
    }

    /// A list, left out when empty.
    pub fn list<T>(self, name: &str, items: &[T], write: impl Fn(&T) -> Json) -> Self {
        if items.is_empty() {
            self
        } else {
            self.always(name, Json::Array(items.iter().map(write).collect()))
        }
    }
}

/// The members of an object being read.
pub struct Fields<'a> {
    what: &'a str,
    members: &'a [(String, Json)],
}

impl<'a> Fields<'a> {
    /// The members of `value`, which is `what` in error messages.
    pub fn of(value: &'a Json, what: &'a str) -> Result<Self, String> {
        match value {
            Json::Object(members) => Ok(Self { what, members }),
            _ => Err(format!("{what} is not an object")),
        }
    }

    /// The value of the member `name`; a member that is `null` is a member that is absent.
    pub fn get(&self, name: &str) -> Option<&'a Json> {
        self.members.iter().find(|(key, _)| key == name).map(|(_, value)| value).filter(|value| **value != Json::Null)
    }

    fn mismatch(&self, name: &str, expected: &str) -> String {
        format!("\"{name}\" of {} is not {expected}", self.what)
    }

    /// A text member that must be present.
    pub fn text(&self, name: &str) -> Result<String, String> {
        self.optional_text(name)?.ok_or_else(|| format!("\"{name}\" is missing in {}", self.what))
    }

    /// A text member; empty when absent.
    pub fn text_or_empty(&self, name: &str) -> Result<String, String> {
        Ok(self.optional_text(name)?.unwrap_or_default())
    }

    /// A text member that may be absent.
    pub fn optional_text(&self, name: &str) -> Result<Option<String>, String> {
        match self.get(name) {
            None => Ok(None),
            Some(Json::String(text)) => Ok(Some(text.clone())),
            Some(_) => Err(self.mismatch(name, "a string")),
        }
    }

    /// A flag; `false` when absent.
    pub fn flag(&self, name: &str) -> Result<bool, String> {
        match self.get(name) {
            None => Ok(false),
            Some(Json::Bool(value)) => Ok(*value),
            Some(_) => Err(self.mismatch(name, "a boolean")),
        }
    }

    /// An integer member that may be absent.
    pub fn optional_integer(&self, name: &str) -> Result<Option<i64>, String> {
        match self.get(name) {
            None => Ok(None),
            Some(Json::Integer(value)) => Ok(Some(*value)),
            Some(_) => Err(self.mismatch(name, "an integer")),
        }
    }

    /// A member that may be absent, read by `read`.
    pub fn optional<T>(&self, name: &str, read: impl Fn(&Json) -> Result<T, String>) -> Result<Option<T>, String> {
        self.get(name).map(read).transpose()
    }

    /// A list; empty when absent.
    pub fn list<T>(&self, name: &str, read: impl Fn(&Json) -> Result<T, String>) -> Result<Vec<T>, String> {
        match self.get(name) {
            None => Ok(Vec::new()),
            Some(Json::Array(items)) => items.iter().map(read).collect(),
            Some(_) => Err(self.mismatch(name, "an array")),
        }
    }
}

/// The text of a value that must be a string (`what` in the error message).
pub fn text_of(value: &Json, what: &str) -> Result<String, String> {
    match value {
        Json::String(text) => Ok(text.clone()),
        _ => Err(format!("{what} is not a string")),
    }
}

/// Arrays and objects nested deeper than this are an error: the model nests a few levels.
const MAX_DEPTH: usize = 64;

struct Parser<'a> {
    text: &'a [u8],
    at: usize,
}

impl Parser<'_> {
    fn error(&self, what: &str) -> String {
        let line = self.text[..self.at.min(self.text.len())].iter().filter(|byte| **byte == b'\n').count() + 1;
        format!("invalid JSON at line {line}: {what}")
    }

    fn whitespace(&mut self) {
        while self.at < self.text.len() && matches!(self.text[self.at], b' ' | b'\t' | b'\n' | b'\r') {
            self.at += 1;
        }
    }

    fn keyword(&mut self, keyword: &str, value: Json) -> Result<Json, String> {
        if self.text[self.at..].starts_with(keyword.as_bytes()) {
            self.at += keyword.len();
            Ok(value)
        } else {
            Err(self.error("unexpected character"))
        }
    }

    fn value(&mut self, depth: usize) -> Result<Json, String> {
        if depth > MAX_DEPTH {
            return Err(self.error("nested too deeply"));
        }
        self.whitespace();
        match self.text.get(self.at) {
            Some(b'{') => {
                self.at += 1;
                let mut members = Vec::new();
                self.whitespace();
                if self.text.get(self.at) == Some(&b'}') {
                    self.at += 1;
                    return Ok(Json::Object(members));
                }
                loop {
                    self.whitespace();
                    let name = self.string()?;
                    self.whitespace();
                    if self.text.get(self.at) != Some(&b':') {
                        return Err(self.error("expected ':'"));
                    }
                    self.at += 1;
                    members.push((name, self.value(depth + 1)?));
                    self.whitespace();
                    match self.text.get(self.at) {
                        Some(b',') => self.at += 1,
                        Some(b'}') => {
                            self.at += 1;
                            return Ok(Json::Object(members));
                        }
                        _ => return Err(self.error("expected ',' or '}'")),
                    }
                }
            }
            Some(b'[') => {
                self.at += 1;
                let mut items = Vec::new();
                self.whitespace();
                if self.text.get(self.at) == Some(&b']') {
                    self.at += 1;
                    return Ok(Json::Array(items));
                }
                loop {
                    items.push(self.value(depth + 1)?);
                    self.whitespace();
                    match self.text.get(self.at) {
                        Some(b',') => self.at += 1,
                        Some(b']') => {
                            self.at += 1;
                            return Ok(Json::Array(items));
                        }
                        _ => return Err(self.error("expected ',' or ']'")),
                    }
                }
            }
            Some(b'"') => Ok(Json::String(self.string()?)),
            Some(b't') => self.keyword("true", Json::Bool(true)),
            Some(b'f') => self.keyword("false", Json::Bool(false)),
            Some(b'n') => self.keyword("null", Json::Null),
            Some(b'-' | b'0'..=b'9') => {
                let start = self.at;
                while self.at < self.text.len() && matches!(self.text[self.at], b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9') {
                    self.at += 1;
                }
                let number = std::str::from_utf8(&self.text[start..self.at]).map_err(|_| self.error("invalid number"))?;
                if let Ok(integer) = number.parse::<i64>() {
                    return Ok(Json::Integer(integer));
                }
                number.parse::<f64>().map(Json::Float).map_err(|_| self.error("invalid number"))
            }
            Some(_) => Err(self.error("unexpected character")),
            None => Err(self.error("unexpected end")),
        }
    }

    fn hex4(&mut self) -> Result<u32, String> {
        let digits = self.text.get(self.at..self.at + 4).ok_or_else(|| self.error("incomplete escape"))?;
        let digits = std::str::from_utf8(digits).map_err(|_| self.error("invalid escape"))?;
        let value = u32::from_str_radix(digits, 16).map_err(|_| self.error("invalid escape"))?;
        self.at += 4;
        Ok(value)
    }

    fn string(&mut self) -> Result<String, String> {
        if self.text.get(self.at) != Some(&b'"') {
            return Err(self.error("expected a string"));
        }
        self.at += 1;
        let mut bytes: Vec<u8> = Vec::new();
        loop {
            let Some(&byte) = self.text.get(self.at) else {
                return Err(self.error("unterminated string"));
            };
            self.at += 1;
            match byte {
                b'"' => break,
                b'\\' => {
                    let Some(&escape) = self.text.get(self.at) else {
                        return Err(self.error("unterminated string"));
                    };
                    self.at += 1;
                    let character = match escape {
                        b'"' => '"',
                        b'\\' => '\\',
                        b'/' => '/',
                        b'b' => '\u{8}',
                        b'f' => '\u{c}',
                        b'n' => '\n',
                        b'r' => '\r',
                        b't' => '\t',
                        b'u' => {
                            let first = self.hex4()?;
                            let code = if (0xd800..0xdc00).contains(&first) && self.text[self.at..].starts_with(b"\\u") {
                                self.at += 2;
                                let second = self.hex4()?;
                                0x10000 + ((first - 0xd800) << 10) + (second.wrapping_sub(0xdc00) & 0x3ff)
                            } else {
                                first
                            };
                            char::from_u32(code).ok_or_else(|| self.error("invalid escape"))?
                        }
                        _ => return Err(self.error("invalid escape")),
                    };
                    let mut buffer = [0u8; 4];
                    bytes.extend_from_slice(character.encode_utf8(&mut buffer).as_bytes());
                }
                byte if byte < 0x20 => return Err(self.error("control character in a string")),
                byte => bytes.push(byte),
            }
        }
        String::from_utf8(bytes).map_err(|_| self.error("invalid UTF-8"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Not from upstream: a value is read back as it was written, and the text is stable.
    #[test]
    fn value_round_trips_through_its_text() {
        let value = Json::Object(vec![
            ("name".to_string(), Json::string("A \"quoted\"\n\u{30a2}\u{1}")),
            ("empty".to_string(), Json::Array(Vec::new())),
            ("none".to_string(), Json::Object(Vec::new())),
            (
                "items".to_string(),
                Json::Array(vec![Json::Integer(-3), Json::Integer(i64::MAX), Json::Float(1.5), Json::Bool(true), Json::Null]),
            ),
        ]);
        let text = value.to_text();
        assert_eq!(Json::parse(&text), Ok(value.clone()));
        assert_eq!(Json::parse(&text).map(|read| read.to_text()), Ok(text.clone()));
        assert!(text.starts_with("{\n  \"name\": \"A \\\"quoted\\\"\\n\u{30a2}\\u0001\",\n  \"empty\": [],\n  \"none\": {},\n"), "{text}");
    }

    /// Not from upstream: escapes, numbers and the errors of the reader.
    #[test]
    fn reader_decodes_escapes_and_reports_errors() {
        assert_eq!(Json::parse(r#""A\u00e9\ud83d\ude00\/""#), Ok(Json::string("A\u{e9}\u{1f600}/")));
        assert_eq!(Json::parse("[1, -2, 1.5e3, 2.0]"), Ok(Json::Array(vec![Json::Integer(1), Json::Integer(-2), Json::Float(1500.0), Json::Float(2.0)])));
        assert_eq!(Json::parse("{\"a\": \n tru}"), Err("invalid JSON at line 2: unexpected character".to_string()));
        assert_eq!(Json::parse("[1] 2"), Err("invalid JSON at line 1: text after the value".to_string()));
        assert_eq!(Json::parse(&"[".repeat(100)), Err("invalid JSON at line 1: nested too deeply".to_string()));
    }

    /// Not from upstream: a member with the default of its kind is not written, and an
    /// absent member reads as that default.
    #[test]
    fn default_members_are_left_out() {
        let empty: [String; 0] = [];
        let value = Json::object(
            Members::new()
                .text("name", "x")
                .text_or_empty("namespace", "")
                .flag("flags", false)
                .optional_text("this", &None)
                .list("items", &empty, |item| Json::string(item)),
        );
        assert_eq!(value, Json::Object(vec![("name".to_string(), Json::string("x"))]));
        let fields = Fields::of(&value, "a value").expect("an object");
        assert_eq!(fields.text("name"), Ok("x".to_string()));
        assert_eq!(fields.text_or_empty("namespace"), Ok(String::new()));
        assert_eq!(fields.flag("flags"), Ok(false));
        assert_eq!(fields.optional_text("this"), Ok(None));
        assert_eq!(fields.list("items", |item| text_of(item, "an item")), Ok(Vec::new()));
        assert_eq!(fields.text("missing"), Err("\"missing\" is missing in a value".to_string()));
        assert_eq!(fields.flag("name"), Err("\"name\" of a value is not a boolean".to_string()));
    }
}
