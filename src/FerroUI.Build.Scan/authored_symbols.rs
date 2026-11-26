//! Optional authoring index written beside a Cargo build's portable XAML model.
use crate::json::{text_of, Fields, Json, Members};
use std::path::{Path, PathBuf};

const VERSION: i64 = 1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthoredSymbols {
    pub manifest: PathBuf,
    pub assembly: String,
    pub crate_name: String,
    pub definitions: Vec<AuthoredDefinition>,
    pub assembly_source: Option<AuthoredAssembly>,
}

/// The selected, physical `MarkupAssembly` declaration in this Cargo build.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthoredAssembly {
    pub file: PathBuf,
    pub modules: Vec<String>,
    pub item: String,
    /// The package-relative crate root used by the scanner.
    pub crate_root: Option<PathBuf>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthoredMetadata {
    pub key: Option<AuthoredText>,
    pub value: Option<AuthoredText>,
    /// A known framework setting with no authorable declaration in this crate.
    pub framework_key: bool,
}

/// Source and stable identifier of a file in a captured Rust module tree.
#[derive(Clone, Copy)]
pub struct MetadataSource<'a> {
    pub id: &'a str,
    pub text: &'a str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthoredText {
    pub value: String,
    /// The literal or immutable constant reference at this tuple's use site.
    pub range: std::ops::Range<usize>,
    /// The source spelling of the literal that supplied this value.
    pub literal: std::ops::Range<usize>,
    /// The exact Rust literal when it was declared in a different module file.
    pub imported_literal: Option<String>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct AuthoredDefinition {
    pub rust_type: String,
    pub type_name: String,
    pub file: PathBuf,
    pub modules: Vec<String>,
    pub kind: DeclarationKind,
    /// The spelling of the owner used in the declaration macro.
    pub owner: String,
    /// A plain property key or a registered property accessor.
    pub member: Option<String>,
    /// Literal public name in a direct property registration, when available.
    pub registered_name: Option<String>,
    /// The declaration of the Rust item behind an implicitly named XAML type.
    pub type_declaration: Option<RustTypeDeclaration>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RustTypeDeclaration {
    pub file: PathBuf,
    pub modules: Vec<String>,
    pub name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TypeNameEdit {
    Alias(std::ops::Range<usize>),
    Symbol(std::ops::Range<usize>),
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum DeclarationKind {
    Class,
    Static,
    MarkupType,
    MarkupEnum,
    ClassInfo,
    Registered,
}

impl DeclarationKind {
    fn spelling(self) -> &'static str {
        match self {
            Self::Class => "class",
            Self::Static => "static",
            Self::MarkupType => "markup_type",
            Self::MarkupEnum => "markup_enum",
            Self::ClassInfo => "class_info",
            Self::Registered => "registered",
        }
    }

    fn parse(text: &str) -> Result<Self, String> {
        [
            Self::Class,
            Self::Static,
            Self::MarkupType,
            Self::MarkupEnum,
            Self::ClassInfo,
            Self::Registered,
        ]
        .into_iter()
        .find(|kind| kind.spelling() == text)
        .ok_or_else(|| format!("Unknown Rust XAML declaration kind {text:?}"))
    }
}

impl AuthoredSymbols {
    pub const FILE_NAME: &str = "xaml-rust-sources.json";

    pub fn parse(text: &str) -> Result<Self, String> {
        let value = Json::parse(text)?;
        let root = Fields::of(&value, "Rust XAML source index")?;
        if root.optional_integer("version")? != Some(VERSION) {
            return Err("Rebuild the XAML schema for Rust source navigation".into());
        }
        Ok(Self {
            manifest: root.text("manifest")?.into(),
            assembly: root.text("assembly")?,
            crate_name: root.text("crate_name")?,
            definitions: root.list("definitions", AuthoredDefinition::parse)?,
            assembly_source: root.optional("assembly_source", AuthoredAssembly::parse)?,
        })
    }

    pub fn to_json(&self) -> String {
        Json::object(
            Members::new()
                .always("version", Json::Integer(VERSION))
                .text("manifest", &self.manifest.to_string_lossy())
                .text("assembly", &self.assembly)
                .text("crate_name", &self.crate_name)
                .list(
                    "definitions",
                    &self.definitions,
                    AuthoredDefinition::to_json,
                )
                .optional(
                    "assembly_source",
                    self.assembly_source.as_ref().map(AuthoredAssembly::to_json),
                ),
        )
        .to_text()
    }

    /// Write only relocatable paths inside the package that actually supplied these types.
    pub fn write(&self, out: &Path) -> Result<(), String> {
        crate::export::write_if_changed(&out.join(Self::FILE_NAME), &self.to_json())?;
        Ok(())
    }
}

impl AuthoredAssembly {
    fn parse(value: &Json) -> Result<Self, String> {
        let entry = Fields::of(value, "Rust markup assembly source")?;
        Ok(Self {
            file: entry.text("file")?.into(),
            modules: entry.list("modules", |value| text_of(value, "inline module"))?,
            item: entry.text("item")?,
            crate_root: entry.optional_text("crate_root")?.map(PathBuf::from),
        })
    }

    fn to_json(&self) -> Json {
        Json::object(
            Members::new()
                .text("file", &self.file.to_string_lossy())
                .list("modules", &self.modules, |name| Json::string(name))
                .text("item", &self.item)
                .optional(
                    "crate_root",
                    self.crate_root.as_ref().map(|path| Json::string(&path.to_string_lossy())),
                ),
        )
    }
}

impl AuthoredDefinition {
    fn parse(value: &Json) -> Result<Self, String> {
        let entry = Fields::of(value, "Rust XAML definition")?;
        Ok(Self {
            rust_type: entry.text("rust_type")?,
            type_name: entry.text("type")?,
            file: entry.text("file")?.into(),
            modules: entry.list("modules", |value| text_of(value, "inline module"))?,
            kind: DeclarationKind::parse(&entry.text("kind")?)?,
            owner: entry.text("owner")?,
            member: entry.optional_text("member")?,
            registered_name: entry.optional_text("registered_name")?,
            type_declaration: entry.optional("type_declaration", RustTypeDeclaration::parse)?,
        })
    }

    fn to_json(&self) -> Json {
        Json::object(
            Members::new()
                .text("rust_type", &self.rust_type)
                .text("type", &self.type_name)
                .text("file", &self.file.to_string_lossy())
                .list("modules", &self.modules, |name| Json::string(name))
                .text("kind", self.kind.spelling())
                .text("owner", &self.owner)
                .optional_text("member", &self.member)
                .optional_text("registered_name", &self.registered_name)
                .optional(
                    "type_declaration",
                    self.type_declaration
                        .as_ref()
                        .map(RustTypeDeclaration::to_json),
                ),
        )
    }
}

impl RustTypeDeclaration {
    fn parse(value: &Json) -> Result<Self, String> {
        let entry = Fields::of(value, "authored Rust type")?;
        Ok(Self {
            file: entry.text("file")?.into(),
            modules: entry.list("modules", |value| text_of(value, "inline module"))?,
            name: entry.text("name")?,
        })
    }

    fn to_json(&self) -> Json {
        Json::object(
            Members::new()
                .text("file", &self.file.to_string_lossy())
                .list("modules", &self.modules, |name| Json::string(name))
                .text("name", &self.name),
        )
    }
}
