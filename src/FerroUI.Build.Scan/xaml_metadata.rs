//! The `.xamlmeta` file of a crate with compiled markup, as far as the
//! compiler reads it today: the assembly and its compiled documents
//! (docs/porting/xaml.md, 9.5.1 `AssemblyModel` / `DocumentModel`, 9.6.3,
//! 9.7.3).
//!
//! Upstream's compiler reaches the compiled documents of a referenced
//! assembly through that assembly's metadata: the type
//! `CompiledFerroXaml.!FerroResources` (the port's name of it) with one static `Build:<path>`
//! method per document, and the `x:Class` types. A Rust crate cannot be
//! inspected that way, so a crate with compiled markup describes its
//! documents in this file, and the compiler of a dependent crate synthesises
//! the same type from it (`rust_emitter::CompiledMarkupTypeSystem` of the loader).
//!
//! # Transport
//!
//! The file reaches dependent crates through Cargo `links` metadata
//! (`DEP_<CRATE>_XAML_XAMLMETA`, 9.6.3 and 9.6.8): a crate whose build script
//! compiles its documents (`ferroui-build`) writes the file to `OUT_DIR`, and
//! a crate whose compiled markup is checked in keeps the file next to its
//! generated `compiled_xaml.rs` (drift-tested with it); the build script of
//! either prints its path for the build scripts of the dependents.
//!
//! # Format
//!
//! JSON, written by [`XamlMetadata::to_json`] (deterministic: the same model
//! gives the same text):
//!
//! ```text
//! {
//!   "name": "Tests",                                  assembly name
//!   "crate_name": "include_fixture_theme",            crate, as paths spell it
//!   "documents": [
//!     {
//!       "uri": "ferres://Tests/Style.xaml",
//!       "root_type": "FerroUI.Styling.Style",          full name of the type the document builds
//!       "class_rust_path": null,                       the x:Class type, when the document has one
//!       "build_path": "::include_fixture_theme::compiled_xaml::build_style_xaml",
//!       "populate_path": null,
//!       "public": true                                 x:ClassModifier
//!     }
//!   ],
//!   "dependencies": ["../Other/compiled_xaml.xamlmeta"],  relative to this file, or absolute
//!   "theme_units": [                                   only when a class has units (#568)
//!     {
//!       "class": "Fixture.Theme",                      the class whose documents the units are of
//!       "table_path": "::fixture::compiled_xaml::THEME_UNITS",
//!       "units": [
//!         {
//!           "name": "Controls/Button.xaml",            the document, or `ambient`
//!           "index": 1,                                canonical index; the ambient unit is 0
//!           "documents": ["Controls/Button.xaml"],
//!           "keys": ["AccentButtonStyle"],             text keys it defines
//!           "type_keys": ["Button"],                   type keys it defines (`{x:Type Button}`)
//!           "references": ["ButtonBackground"],        keys of the group it names
//!           "types": ["Border", "Button"]              types it constructs or selects
//!         }
//!       ]
//!     }
//!   ]
//! }
//! ```
//!
//! # Theme units
//!
//! The unit graph of the compiled theme of a class (#568, step 2 of
//! docs/porting/theme-trimming.md): the generated file applies every unit through the static
//! table `table_path`; the graph says what each unit defines and what it needs. The documents
//! of a unit are in the order of the group; the other lists are sorted and hold each name once.
//! Type names are local names, as the documents name them.
//! The member is left out when no class has units, and a reader that does not know it ignores
//! it.

use std::path::{Path, PathBuf};

/// A compiled document of a crate (`DocumentModel`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocumentModel {
    /// The URI the document is loaded by (`ferres://<assembly>/<path>`).
    pub uri: String,
    /// The full name (`Namespace.Name`) of the type of the root of the document.
    pub root_type: String,
    /// The public Rust path of the `x:Class` type of the document.
    pub class_rust_path: Option<String>,
    /// The absolute Rust path of the build function of a document without a class:
    /// `fn(Option<Rc<dyn IServiceProvider>>) -> Result<Ref<T>, XamlLoadError>`.
    pub build_path: Option<String>,
    /// The absolute Rust path of the populate function of the document.
    pub populate_path: Option<String>,
    /// Whether the document is public (`x:ClassModifier`): only a public document can be
    /// included from another crate.
    pub public: bool,
}

/// A unit of the compiled theme of a class (#568): one merged document, or the ambient content
/// that is always kept, and what it defines and needs.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ThemeUnitModel {
    /// The path of the document of the unit below the root URI of the assembly, or `ambient`.
    pub name: String,
    /// The canonical index of the unit: the order the units are applied in. The ambient unit
    /// is 0.
    pub index: u32,
    /// The documents whose entries the unit holds, by their path below the root URI.
    pub documents: Vec<String>,
    /// The text keys the unit defines.
    pub keys: Vec<String>,
    /// The type keys the unit defines (`{x:Type Button}`), by the local name of the type.
    pub type_keys: Vec<String>,
    /// The keys of the group the unit names (`{StaticResource}`, `{DynamicResource}`,
    /// `BasedOn`, `Theme`, and every other mention of a key).
    pub references: Vec<String>,
    /// The types the unit constructs or selects (elements, `TargetType`, selectors, `{x:Type}`),
    /// by their local names.
    pub types: Vec<String>,
}

/// The units of the compiled theme of a class (#568).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ThemeUnitsModel {
    /// The full name of the class whose documents the units are of.
    pub class: String,
    /// The absolute Rust path of the static table of every unit, in canonical order.
    pub table_path: String,
    /// The units, in canonical order.
    pub units: Vec<ThemeUnitModel>,
}

/// The compiled markup a crate exports to its dependents (`AssemblyModel`, the part of
/// it the compiler reads today).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct XamlMetadata {
    /// The assembly name of the crate.
    pub name: String,
    /// The name of the crate, as Rust paths spell it.
    pub crate_name: String,
    /// The compiled documents, in the order of the compilation.
    pub documents: Vec<DocumentModel>,
    /// The `.xamlmeta` files of the crates this crate includes documents of, relative to
    /// the directory of this file or absolute (the files of a build are in the `OUT_DIR`
    /// of each crate).
    pub dependencies: Vec<String>,
    /// The units of the compiled themes of the classes of the crate (#568), in the order of the
    /// compilation; empty for a crate without them.
    pub theme_units: Vec<ThemeUnitsModel>,
}

impl XamlMetadata {
    /// The model as the text of a `.xamlmeta` file.
    pub fn to_json(&self) -> String {
        let mut text = String::from("{\n");
        text.push_str(&format!("  \"name\": {},\n", json_string(&self.name)));
        text.push_str(&format!("  \"crate_name\": {},\n", json_string(&self.crate_name)));
        text.push_str("  \"documents\": [");
        for (index, document) in self.documents.iter().enumerate() {
            text.push_str(if index == 0 { "\n" } else { ",\n" });
            text.push_str("    {\n");
            text.push_str(&format!("      \"uri\": {},\n", json_string(&document.uri)));
            text.push_str(&format!("      \"root_type\": {},\n", json_string(&document.root_type)));
            text.push_str(&format!("      \"class_rust_path\": {},\n", json_optional(&document.class_rust_path)));
            text.push_str(&format!("      \"build_path\": {},\n", json_optional(&document.build_path)));
            text.push_str(&format!("      \"populate_path\": {},\n", json_optional(&document.populate_path)));
            text.push_str(&format!("      \"public\": {}\n", document.public));
            text.push_str("    }");
        }
        text.push_str(if self.documents.is_empty() { "],\n" } else { "\n  ],\n" });
        text.push_str("  \"dependencies\": [");
        let dependencies: Vec<String> = self.dependencies.iter().map(|path| json_string(path)).collect();
        text.push_str(&dependencies.join(", "));
        text.push(']');
        if !self.theme_units.is_empty() {
            text.push_str(",\n  \"theme_units\": [");
            for (index, units) in self.theme_units.iter().enumerate() {
                text.push_str(if index == 0 { "\n" } else { ",\n" });
                write_theme_units(&mut text, units);
            }
            text.push_str("\n  ]");
        }
        text.push_str("\n}\n");
        text
    }

    /// Reads the model from the text of a `.xamlmeta` file.
    pub fn parse(text: &str) -> Result<Self, String> {
        let value = JsonParser { text: text.as_bytes(), at: 0 }.document()?;
        let object = value.object("the file")?;
        let documents = field(object, "documents")?
            .array("documents")?
            .iter()
            .map(|document| {
                let document = document.object("a document")?;
                Ok(DocumentModel {
                    uri: field(document, "uri")?.string("uri")?.to_string(),
                    root_type: field(document, "root_type")?.string("root_type")?.to_string(),
                    class_rust_path: field(document, "class_rust_path")?.optional_string("class_rust_path")?,
                    build_path: field(document, "build_path")?.optional_string("build_path")?,
                    populate_path: field(document, "populate_path")?.optional_string("populate_path")?,
                    public: field(document, "public")?.boolean("public")?,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let dependencies = field(object, "dependencies")?
            .array("dependencies")?
            .iter()
            .map(|path| path.string("a dependency").map(str::to_string))
            .collect::<Result<Vec<_>, String>>()?;
        let theme_units = match object.iter().find(|(key, _)| key == "theme_units") {
            Some((_, value)) => value.array("theme_units")?.iter().map(read_theme_units).collect::<Result<Vec<_>, String>>()?,
            None => Vec::new(),
        };
        Ok(Self {
            name: field(object, "name")?.string("name")?.to_string(),
            crate_name: field(object, "crate_name")?.string("crate_name")?.to_string(),
            documents,
            dependencies,
            theme_units,
        })
    }

    /// Reads the `.xamlmeta` file `path` and, transitively, the files of its
    /// dependencies: every crate whose documents the compiler may reach through it
    /// (`DEP_*` metadata exists only for direct dependencies, 9.6.3). Each assembly is
    /// listed once, `path` first.
    pub fn read(path: impl AsRef<Path>) -> Result<Vec<XamlMetadata>, String> {
        let mut read: Vec<(PathBuf, XamlMetadata)> = Vec::new();
        let mut pending = vec![path.as_ref().to_path_buf()];
        while let Some(path) = pending.pop() {
            let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            let metadata = Self::parse(&text).map_err(|e| format!("{}: {e}", path.display()))?;
            if read.iter().any(|(_, known)| known.name == metadata.name) {
                continue;
            }
            let directory = path.parent().map(Path::to_path_buf).unwrap_or_default();
            for dependency in metadata.dependencies.iter().rev() {
                pending.push(directory.join(dependency));
            }
            read.push((path, metadata));
        }
        Ok(read.into_iter().map(|(_, metadata)| metadata).collect())
    }
}

/// Writes the units of a class as an item of `"theme_units"`: each unit on its lines, each
/// list on one line.
fn write_theme_units(text: &mut String, units: &ThemeUnitsModel) {
    let list = |items: &[String]| format!("[{}]", items.iter().map(|item| json_string(item)).collect::<Vec<_>>().join(", "));
    text.push_str("    {\n");
    text.push_str(&format!("      \"class\": {},\n", json_string(&units.class)));
    text.push_str(&format!("      \"table_path\": {},\n", json_string(&units.table_path)));
    text.push_str("      \"units\": [");
    for (index, unit) in units.units.iter().enumerate() {
        text.push_str(if index == 0 { "\n" } else { ",\n" });
        text.push_str("        {\n");
        text.push_str(&format!("          \"name\": {},\n", json_string(&unit.name)));
        text.push_str(&format!("          \"index\": {},\n", unit.index));
        text.push_str(&format!("          \"documents\": {},\n", list(&unit.documents)));
        text.push_str(&format!("          \"keys\": {},\n", list(&unit.keys)));
        text.push_str(&format!("          \"type_keys\": {},\n", list(&unit.type_keys)));
        text.push_str(&format!("          \"references\": {},\n", list(&unit.references)));
        text.push_str(&format!("          \"types\": {}\n", list(&unit.types)));
        text.push_str("        }");
    }
    text.push_str(if units.units.is_empty() { "]\n" } else { "\n      ]\n" });
    text.push_str("    }");
}

/// Reads an item of `"theme_units"`.
fn read_theme_units(value: &JsonValue) -> Result<ThemeUnitsModel, String> {
    let object = value.object("the units of a class")?;
    let strings = |object: &[(String, JsonValue)], name: &str| -> Result<Vec<String>, String> {
        field(object, name)?.array(name)?.iter().map(|item| item.string(name).map(str::to_string)).collect()
    };
    let units = field(object, "units")?
        .array("units")?
        .iter()
        .map(|unit| {
            let unit = unit.object("a unit")?;
            Ok(ThemeUnitModel {
                name: field(unit, "name")?.string("name")?.to_string(),
                index: field(unit, "index")?.integer("index")?,
                documents: strings(unit, "documents")?,
                keys: strings(unit, "keys")?,
                type_keys: strings(unit, "type_keys")?,
                references: strings(unit, "references")?,
                types: strings(unit, "types")?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(ThemeUnitsModel {
        class: field(object, "class")?.string("class")?.to_string(),
        table_path: field(object, "table_path")?.string("table_path")?.to_string(),
        units,
    })
}

fn json_optional(value: &Option<String>) -> String {
    value.as_deref().map_or_else(|| "null".to_string(), json_string)
}

/// `text` as a JSON string literal.
fn json_string(text: &str) -> String {
    let mut literal = String::with_capacity(text.len() + 2);
    literal.push('"');
    for character in text.chars() {
        match character {
            '"' => literal.push_str("\\\""),
            '\\' => literal.push_str("\\\\"),
            '\n' => literal.push_str("\\n"),
            '\r' => literal.push_str("\\r"),
            '\t' => literal.push_str("\\t"),
            c if (c as u32) < 0x20 => literal.push_str(&format!("\\u{:04x}", c as u32)),
            c => literal.push(c),
        }
    }
    literal.push('"');
    literal
}

/// A JSON value.
enum JsonValue {
    Null,
    Boolean(bool),
    /// A number, as its text.
    Number(String),
    String(String),
    Array(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>),
}

impl JsonValue {
    fn object(&self, what: &str) -> Result<&[(String, JsonValue)], String> {
        match self {
            JsonValue::Object(fields) => Ok(fields),
            _ => Err(format!("{what} is not an object")),
        }
    }

    fn array(&self, what: &str) -> Result<&[JsonValue], String> {
        match self {
            JsonValue::Array(items) => Ok(items),
            _ => Err(format!("{what} is not an array")),
        }
    }

    fn string(&self, what: &str) -> Result<&str, String> {
        match self {
            JsonValue::String(text) => Ok(text),
            _ => Err(format!("{what} is not a string")),
        }
    }

    fn optional_string(&self, what: &str) -> Result<Option<String>, String> {
        match self {
            JsonValue::Null => Ok(None),
            JsonValue::String(text) => Ok(Some(text.clone())),
            _ => Err(format!("{what} is neither a string nor null")),
        }
    }

    fn boolean(&self, what: &str) -> Result<bool, String> {
        match self {
            JsonValue::Boolean(value) => Ok(*value),
            _ => Err(format!("{what} is not a boolean")),
        }
    }

    fn integer(&self, what: &str) -> Result<u32, String> {
        match self {
            JsonValue::Number(text) => text.parse::<u32>().map_err(|_| format!("{what} is not an unsigned integer")),
            _ => Err(format!("{what} is not a number")),
        }
    }
}

fn field<'a>(object: &'a [(String, JsonValue)], name: &str) -> Result<&'a JsonValue, String> {
    object.iter().find(|(key, _)| key == name).map(|(_, value)| value).ok_or_else(|| format!("\"{name}\" is missing"))
}

/// A reader of the JSON the model is written in (RFC 8259; a number is kept as its text, and
/// the model has only the indices of theme units).
struct JsonParser<'a> {
    text: &'a [u8],
    at: usize,
}

impl JsonParser<'_> {
    fn document(mut self) -> Result<JsonValue, String> {
        let value = self.value()?;
        self.whitespace();
        if self.at != self.text.len() {
            return Err(self.error("text after the value"));
        }
        Ok(value)
    }

    fn error(&self, what: &str) -> String {
        let line = self.text[..self.at.min(self.text.len())].iter().filter(|b| **b == b'\n').count() + 1;
        format!("invalid JSON at line {line}: {what}")
    }

    fn whitespace(&mut self) {
        while self.at < self.text.len() && matches!(self.text[self.at], b' ' | b'\t' | b'\n' | b'\r') {
            self.at += 1;
        }
    }

    fn expect(&mut self, byte: u8) -> Result<(), String> {
        self.whitespace();
        if self.text.get(self.at) == Some(&byte) {
            self.at += 1;
            Ok(())
        } else {
            Err(self.error(&format!("expected '{}'", byte as char)))
        }
    }

    fn keyword(&mut self, keyword: &str, value: JsonValue) -> Result<JsonValue, String> {
        if self.text[self.at..].starts_with(keyword.as_bytes()) {
            self.at += keyword.len();
            Ok(value)
        } else {
            Err(self.error("unexpected character"))
        }
    }

    fn value(&mut self) -> Result<JsonValue, String> {
        self.whitespace();
        match self.text.get(self.at) {
            Some(b'{') => {
                self.at += 1;
                let mut fields = Vec::new();
                self.whitespace();
                if self.text.get(self.at) == Some(&b'}') {
                    self.at += 1;
                    return Ok(JsonValue::Object(fields));
                }
                loop {
                    self.whitespace();
                    let key = self.string()?;
                    self.expect(b':')?;
                    fields.push((key, self.value()?));
                    self.whitespace();
                    match self.text.get(self.at) {
                        Some(b',') => self.at += 1,
                        Some(b'}') => {
                            self.at += 1;
                            return Ok(JsonValue::Object(fields));
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
                    return Ok(JsonValue::Array(items));
                }
                loop {
                    items.push(self.value()?);
                    self.whitespace();
                    match self.text.get(self.at) {
                        Some(b',') => self.at += 1,
                        Some(b']') => {
                            self.at += 1;
                            return Ok(JsonValue::Array(items));
                        }
                        _ => return Err(self.error("expected ',' or ']'")),
                    }
                }
            }
            Some(b'"') => Ok(JsonValue::String(self.string()?)),
            Some(b't') => self.keyword("true", JsonValue::Boolean(true)),
            Some(b'f') => self.keyword("false", JsonValue::Boolean(false)),
            Some(b'n') => self.keyword("null", JsonValue::Null),
            Some(b'-' | b'0'..=b'9') => {
                let start = self.at;
                while self.at < self.text.len() && matches!(self.text[self.at], b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9') {
                    self.at += 1;
                }
                Ok(JsonValue::Number(String::from_utf8_lossy(&self.text[start..self.at]).into_owned()))
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

    /// Not from upstream: the model is read back as it was written.
    #[test]
    fn metadata_round_trips_through_its_text() {
        let metadata = XamlMetadata {
            name: "\u{30a2}\u{30bb}\u{30f3}\u{30d6}\u{30ea}".to_string(),
            crate_name: "fixture".to_string(),
            documents: vec![
                DocumentModel {
                    uri: "ferres://Tests/Folder/\"Quoted\".xaml".to_string(),
                    root_type: "FerroUI.Styling.Style".to_string(),
                    class_rust_path: None,
                    build_path: Some("::fixture::compiled_xaml::build_quoted_xaml".to_string()),
                    populate_path: None,
                    public: true,
                },
                DocumentModel {
                    uri: "ferres://Tests/Theme.xaml".to_string(),
                    root_type: "Fixture.Theme".to_string(),
                    class_rust_path: Some("::fixture::Theme".to_string()),
                    build_path: None,
                    populate_path: Some("::fixture::compiled_xaml::populate".to_string()),
                    public: false,
                },
            ],
            dependencies: vec!["../Other/compiled_xaml.xamlmeta".to_string()],
            theme_units: Vec::new(),
        };
        let text = metadata.to_json();
        assert!(!text.contains("theme_units"), "a crate without units writes no units");
        assert_eq!(XamlMetadata::parse(&text), Ok(metadata));
        let empty = XamlMetadata { name: "A".into(), crate_name: "a".into(), documents: Vec::new(), dependencies: Vec::new(), theme_units: Vec::new() };
        assert_eq!(XamlMetadata::parse(&empty.to_json()), Ok(empty));
    }

    /// Not from upstream: the units of a compiled theme (#568) are read back as they were
    /// written, and a file without them reads as one without units.
    #[test]
    fn theme_units_round_trip_through_the_text() {
        let unit = |name: &str, index: u32| ThemeUnitModel {
            name: name.to_string(),
            index,
            documents: vec![name.to_string()],
            keys: vec!["AccentButtonStyle".to_string(), "Key \"quoted\"".to_string()],
            type_keys: vec!["Button".to_string()],
            references: vec!["ButtonBackground".to_string()],
            types: vec!["Border".to_string(), "Button".to_string()],
        };
        let metadata = XamlMetadata {
            name: "Fixture".to_string(),
            crate_name: "fixture".to_string(),
            documents: Vec::new(),
            dependencies: Vec::new(),
            theme_units: vec![
                ThemeUnitsModel {
                    class: "Fixture.Theme".to_string(),
                    table_path: "::fixture::compiled_xaml::THEME_UNITS".to_string(),
                    units: vec![ThemeUnitModel { name: "ambient".to_string(), ..ThemeUnitModel::default() }, unit("Controls/Button.xaml", 1)],
                },
                ThemeUnitsModel { class: "Fixture.Other".to_string(), table_path: "::fixture::other::THEME_UNITS".to_string(), units: Vec::new() },
            ],
        };
        let text = metadata.to_json();
        assert!(text.contains("\n          \"index\": 1,\n"), "{text}");
        assert_eq!(XamlMetadata::parse(&text), Ok(metadata));
        assert_eq!(
            XamlMetadata::parse(r#"{"name": "A", "crate_name": "a", "documents": [], "dependencies": [], "theme_units": [{"class": "C", "table_path": "p", "units": [{"name": "u", "index": -1, "documents": [], "keys": [], "type_keys": [], "references": [], "types": []}]}]}"#),
            Err("index is not an unsigned integer".to_string())
        );
    }

    /// Not from upstream: escapes and errors of the reader.
    #[test]
    fn metadata_reader_decodes_escapes_and_reports_errors() {
        let text = r#"{"name": "Aé😀\/", "crate_name": "a", "documents": [], "dependencies": [], "extra": [1.5e3, -2]}"#;
        assert_eq!(XamlMetadata::parse(text).map(|m| m.name), Ok("A\u{e9}\u{1f600}/".to_string()));
        assert_eq!(
            XamlMetadata::parse(r#"{"name": "A", "documents": [], "dependencies": []}"#),
            Err("\"crate_name\" is missing".to_string())
        );
        assert_eq!(XamlMetadata::parse("{\"name\": \n tru}"), Err("invalid JSON at line 2: unexpected character".to_string()));
    }
}
