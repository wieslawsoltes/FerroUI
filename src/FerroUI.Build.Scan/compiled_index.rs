//! Project tooling index of the generated Rust entry points of each XAML module.
use super::{JsonParser, JsonValue, field, json_optional, json_string};
use crate::compiled_names::CompiledGroupKind;
use std::path::PathBuf;

const VERSION: u32 = 1;

/// A compiler-produced companion to the package `.xamlmeta`. This index includes
/// internal modules and class members merged into another document, which are not
/// part of the loadable metadata exported to dependent packages.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledXamlIndex {
    pub version: u32,
    pub manifest: PathBuf,
    pub groups: Vec<CompiledXamlGroup>,
    /// File inputs actually read by the build, including unused candidates of class groups.
    pub file_inputs: Vec<CompiledXamlInput>,
}

impl CompiledXamlIndex {
    pub const FILE_NAME: &str = "xaml-symbols.json";

    pub fn new(manifest: PathBuf) -> Self {
        Self {
            version: VERSION,
            manifest,
            groups: Vec::new(),
            file_inputs: Vec::new(),
        }
    }

    pub fn is_supported(&self) -> bool {
        self.version == VERSION
    }

    pub fn parse(source: &str) -> Result<Self, String> {
        let json = JsonParser {
            text: source.as_bytes(),
            at: 0,
        }
        .document()?;
        let root = json.object("compiled XAML index")?;
        Ok(Self {
            version: field(root, "version")?.integer("version")?,
            manifest: field(root, "manifest")?.string("manifest")?.into(),
            groups: field(root, "groups")?
                .array("groups")?
                .iter()
                .map(CompiledXamlGroup::parse)
                .collect::<Result<_, _>>()?,
            file_inputs: match root.iter().find(|(key, _)| key == "file_inputs") {
                Some((_, value)) => value
                    .array("file_inputs")?
                    .iter()
                    .map(CompiledXamlInput::parse)
                    .collect::<Result<_, _>>()?,
                None => Vec::new(),
            },
        })
    }

    pub fn to_json(&self) -> String {
        let groups = self
            .groups
            .iter()
            .map(CompiledXamlGroup::to_json)
            .collect::<Vec<_>>()
            .join(",");
        let mut json = format!(
            "{{\"version\":{},\"manifest\":{},\"groups\":[{groups}]",
            self.version,
            json_string(&self.manifest.to_string_lossy())
        );
        if !self.file_inputs.is_empty() {
            let inputs = self
                .file_inputs
                .iter()
                .map(CompiledXamlInput::to_json)
                .collect::<Vec<_>>();
            json.push_str(&format!(",\"file_inputs\":[{}]", inputs.join(",")));
        }
        json.push('}');
        json
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct CompiledXamlInput {
    /// Authored physical path, relative to the Cargo manifest unless absolute.
    pub path: String,
    /// Rust's file name and one-based character coordinates at registration.
    pub rust_file: PathBuf,
    pub line: u32,
    pub column: u32,
}

impl CompiledXamlInput {
    fn parse(json: &JsonValue) -> Result<Self, String> {
        let input = json.object("a compiled XAML file input")?;
        Ok(Self {
            path: field(input, "path")?.string("path")?.into(),
            rust_file: field(input, "rust_file")?.string("rust_file")?.into(),
            line: field(input, "line")?.integer("line")?,
            column: field(input, "column")?.integer("column")?,
        })
    }

    fn to_json(&self) -> String {
        format!(
            "{{\"path\":{},\"rust_file\":{},\"line\":{},\"column\":{}}}",
            json_string(&self.path),
            json_string(&self.rust_file.to_string_lossy()),
            self.line,
            self.column,
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledXamlGroup {
    /// The Rust module at the package root, written to `OUT_DIR/xaml/<module>.rs`.
    pub module: String,
    pub kind: CompiledGroupKind,
    /// The directory scanned for an automatic group. An explicit group supplies
    /// its key and bytes from the build script and has no implicit physical path.
    pub discovery_root: Option<PathBuf>,
    /// File-backed registrations have stable logical names independent of their paths.
    pub file_backed: bool,
    /// Compiler order includes every input to name allocation, even if a member
    /// is merged into the class and has no callable Rust entry point.
    pub members: Vec<CompiledXamlMember>,
}

impl CompiledXamlGroup {
    fn parse(json: &JsonValue) -> Result<Self, String> {
        let group = json.object("a compiled XAML group")?;
        let kind = match field(group, "kind")?.string("kind")? {
            "documents" => CompiledGroupKind::Documents,
            "class" => CompiledGroupKind::Class,
            _ => return Err("Unknown compiled XAML group kind".into()),
        };
        Ok(Self {
            module: field(group, "module")?.string("module")?.into(),
            kind,
            discovery_root: field(group, "discovery_root")?
                .optional_string("discovery_root")?
                .map(PathBuf::from),
            file_backed: group
                .iter()
                .find(|(key, _)| key == "file_backed")
                .map(|(_, value)| value.boolean("file_backed"))
                .transpose()?
                .unwrap_or_default(),
            members: field(group, "members")?
                .array("members")?
                .iter()
                .map(CompiledXamlMember::parse)
                .collect::<Result<_, _>>()?,
        })
    }

    fn to_json(&self) -> String {
        let kind = match self.kind {
            CompiledGroupKind::Documents => "documents",
            CompiledGroupKind::Class => "class",
        };
        let discovery = self
            .discovery_root
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned());
        let members = self
            .members
            .iter()
            .map(CompiledXamlMember::to_json)
            .collect::<Vec<_>>()
            .join(",");
        let file_backed = if self.file_backed {
            ",\"file_backed\":true"
        } else {
            ""
        };
        format!(
            "{{\"module\":{},\"kind\":\"{kind}\",\"discovery_root\":{},\"members\":[{members}]{file_backed}}}",
            json_string(&self.module),
            json_optional(&discovery)
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledXamlMember {
    pub key: String,
    /// The emitted entry point, including the fixed `populate` of a class.
    pub symbol: Option<String>,
    /// The group's loader can resolve this document by its public address.
    pub public: bool,
}

impl CompiledXamlMember {
    fn parse(json: &JsonValue) -> Result<Self, String> {
        let member = json.object("a compiled XAML group member")?;
        Ok(Self {
            key: field(member, "key")?.string("key")?.into(),
            symbol: field(member, "symbol")?.optional_string("symbol")?,
            public: field(member, "public")?.boolean("public")?,
        })
    }

    fn to_json(&self) -> String {
        format!(
            "{{\"key\":{},\"symbol\":{},\"public\":{}}}",
            json_string(&self.key),
            json_optional(&self.symbol),
            self.public
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_original_symbol_index_format() {
        let input = r#"{"version":1,"manifest":"/my/tree/Cargo.toml","groups":[{"module":"views","kind":"class","discovery_root":"d\\é","members":[{"key":"A\"🙂.xaml","symbol":"populate","public":true},{"key":"B.xaml","symbol":null,"public":false}]}]}"#;
        let index = CompiledXamlIndex::parse(input).unwrap();
        assert!(index.is_supported());
        assert_eq!(index.to_json(), input);
        assert!(
            CompiledXamlIndex::parse(&"[".repeat(1000))
                .unwrap_err()
                .contains("deeply")
        );
    }
}
