//! Project tooling index of the generated Rust entry points of each XAML module.
use crate::compiled_names::CompiledGroupKind;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const VERSION: u32 = 1;

/// A compiler-produced companion to the package `.xamlmeta`. This index includes
/// internal modules and class members merged into another document, which are not
/// part of the loadable metadata exported to dependent packages.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
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
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
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

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct CompiledXamlMember {
    pub key: String,
    /// The emitted entry point, including the fixed `populate` of a class.
    pub symbol: Option<String>,
    /// The group's loader can resolve this document by its public address.
    pub public: bool,
}
