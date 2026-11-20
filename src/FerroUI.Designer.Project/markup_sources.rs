//! Build-scoped Rust definitions and their validated physical source files.
use super::MarkupCatalog;
use ferroui_build_scan::{
    authored_symbols::{AuthoredDefinition, AuthoredSymbols, DeclarationKind},
    model::{AssemblyModel, TypeModel},
};
use std::{
    collections::{btree_map::Entry, BTreeMap},
    fs,
    io::Read,
    ops::Range,
    path::{Component, Path, PathBuf},
    sync::Arc,
};

pub(super) const INDEX_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarkupSource {
    package: PathBuf,
    definition: AuthoredDefinition,
}

impl MarkupSource {
    /// Resolve the source each time so a moved file or a replacement symlink
    /// cannot supply the range for a different declaration.
    pub fn file(&self) -> Option<PathBuf> {
        let file = self
            .package
            .join(&self.definition.file)
            .canonicalize()
            .ok()?;
        (file.starts_with(&self.package)
            && file.extension().is_some_and(|extension| extension == "rs")
            && file.is_file())
        .then_some(file)
    }

    pub fn range_in(&self, text: &str) -> Option<Range<usize>> {
        self.definition.range_in(text)
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct Symbol {
    assembly: String,
    crate_name: String,
    rust_type: String,
    type_name: String,
    member: Option<(bool, String)>,
}

#[derive(Default)]
pub(super) struct MarkupSources {
    definitions: BTreeMap<Symbol, Option<Arc<MarkupSource>>>,
}

impl MarkupSources {
    pub(super) fn read(&mut self, directory: &Path, model: &AssemblyModel, remaining: &mut u64) {
        let Some((index, package)) = load(directory, model, remaining) else {
            return;
        };
        for definition in index.definitions {
            if !relative_rust_path(&definition.file) {
                continue;
            }
            let key = Symbol {
                assembly: model.name.clone(),
                crate_name: model.crate_name.clone(),
                rust_type: definition.rust_type.clone(),
                type_name: definition.type_name.clone(),
                member: definition
                    .member
                    .as_ref()
                    .map(|name| (definition.kind == DeclarationKind::Registered, name.clone())),
            };
            let source = MarkupSource {
                package: package.clone(),
                definition,
            };
            match self.definitions.entry(key) {
                Entry::Vacant(entry) => {
                    entry.insert(Some(Arc::new(source)));
                }
                Entry::Occupied(mut entry) => {
                    if entry.get().as_deref() != Some(&source) {
                        entry.insert(None);
                    }
                }
            }
        }
    }
}

impl MarkupCatalog {
    /// A definition in this build's selected assembly, or no result on ambiguity.
    pub fn authored_definition(
        &self,
        assembly: &AssemblyModel,
        type_: &TypeModel,
        member: Option<(bool, &str)>,
    ) -> Option<Arc<MarkupSource>> {
        self.sources
            .definitions
            .get(&Symbol {
                assembly: assembly.name.clone(),
                crate_name: assembly.crate_name.clone(),
                rust_type: type_.rust_path.text.clone(),
                type_name: type_.name.clone(),
                member: member.map(|(registered, name)| (registered, name.to_owned())),
            })?
            .clone()
    }
}

fn load(
    directory: &Path,
    model: &AssemblyModel,
    remaining: &mut u64,
) -> Option<(AuthoredSymbols, PathBuf)> {
    let path = directory.join(AuthoredSymbols::FILE_NAME);
    let metadata = fs::symlink_metadata(&path).ok()?;
    if !metadata.is_file() || metadata.len() > *remaining {
        return None;
    }
    let mut text = String::new();
    fs::File::open(path)
        .ok()?
        .take(*remaining + 1)
        .read_to_string(&mut text)
        .ok()?;
    *remaining = remaining.checked_sub(text.len() as u64)?;
    let index = AuthoredSymbols::parse(&text).ok()?;
    if index.assembly != model.name
        || index.crate_name != model.crate_name
        || !index.manifest.is_absolute()
    {
        return None;
    }
    let manifest = index.manifest.canonicalize().ok()?;
    if manifest.file_name().is_none_or(|name| name != "Cargo.toml") || !manifest.is_file() {
        return None;
    }
    Some((index, manifest.parent()?.to_owned()))
}

fn relative_rust_path(path: &Path) -> bool {
    path.extension().is_some_and(|extension| extension == "rs")
        && path
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
}
