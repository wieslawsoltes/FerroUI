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
}

impl CompiledXamlIndex {
    pub const FILE_NAME: &str = "xaml-symbols.json";

    pub fn new(manifest: PathBuf) -> Self {
        Self {
            version: VERSION,
            manifest,
            groups: Vec::new(),
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
        })
    }

    pub fn to_json(&self) -> String {
        let groups = self
            .groups
            .iter()
            .map(CompiledXamlGroup::to_json)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"version\":{},\"manifest\":{},\"groups\":[{groups}]}}",
            self.version,
            json_string(&self.manifest.to_string_lossy())
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
        format!(
            "{{\"module\":{},\"kind\":\"{kind}\",\"discovery_root\":{},\"members\":[{members}]}}",
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
