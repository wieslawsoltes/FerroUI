//! Build-scoped Rust definitions and their validated physical source files.
use super::MarkupCatalog;
use ferroui_build_scan::{
    authored_symbols::{
        AuthoredAssembly, AuthoredDefinition, AuthoredMetadata, AuthoredSymbols, DeclarationKind,
        TypeNameEdit,
    },
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarkupAssemblySource {
    package: PathBuf,
    declaration: AuthoredAssembly,
}

impl MarkupAssemblySource {
    pub fn file(&self) -> Option<PathBuf> {
        resolve_rust_file(&self.package, &self.declaration.file)
    }

    pub fn metadata_in(&self, source: &str) -> Option<Vec<AuthoredMetadata>> {
        self.declaration.metadata_in(source)
    }
}

impl MarkupSource {
    /// Resolve the source each time so a moved file or a replacement symlink
    /// cannot supply the range for a different declaration.
    pub fn file(&self) -> Option<PathBuf> {
        resolve_rust_file(&self.package, &self.definition.file)
    }

    pub fn range_in(&self, text: &str) -> Option<Range<usize>> {
        self.definition.range_in(text)
    }

    pub fn property_name_range(&self, text: &str, new_name: &str) -> Option<Range<usize>> {
        self.definition.property_name_range(text, new_name)
    }

    pub fn type_name_edit(&self, text: &str, new_name: &str) -> Option<TypeNameEdit> {
        self.definition.type_name_edit(text, new_name)
    }

    pub fn type_file(&self) -> Option<PathBuf> {
        resolve_rust_file(
            &self.package,
            &self.definition.type_declaration.as_ref()?.file,
        )
    }

    pub fn type_range_in(&self, source: &str) -> Option<Range<usize>> {
        self.definition.type_declaration.as_ref()?.range_in(source)
    }

    pub fn package(&self) -> &Path {
        &self.package
    }

    pub fn rust_type(&self) -> &str {
        &self.definition.rust_type
    }

    pub fn is_class(&self) -> bool {
        self.definition.kind == DeclarationKind::Class
    }

    /// Compare registrations across build receipts independently of optional source hints.
    pub fn same_registration(&self, other: &Self) -> bool {
        let (left, right) = (&self.definition, &other.definition);
        self.package == other.package
            && left.rust_type == right.rust_type
            && left.type_name == right.type_name
            && left.file == right.file
            && left.modules == right.modules
            && left.kind == right.kind
            && left.owner == right.owner
            && left.member == right.member
            && left.registered_name == right.registered_name
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
    assemblies: BTreeMap<(String, String), Option<MarkupAssemblySource>>,
}

impl MarkupSources {
    pub(super) fn read(&mut self, directory: &Path, model: &AssemblyModel, remaining: &mut u64) {
        let Some((index, package)) = load(directory, model, remaining) else {
            return;
        };
        if let Some(declaration) = index
            .assembly_source
            .filter(|source| relative_rust_path(&source.file))
        {
            let source = MarkupAssemblySource {
                package: package.clone(),
                declaration,
            };
            let key = (model.name.clone(), model.crate_name.clone());
            match self.assemblies.entry(key) {
                Entry::Vacant(entry) => {
                    entry.insert(Some(source));
                }
                Entry::Occupied(mut entry) if entry.get().as_ref() != Some(&source) => {
                    entry.insert(None);
                }
                Entry::Occupied(_) => {}
            }
        }
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
                    if let Some(prior) = entry.get() {
                        if prior.same_registration(&source) {
                            match (
                                &prior.definition.type_declaration,
                                &source.definition.type_declaration,
                            ) {
                                (None, Some(_)) => {
                                    entry.insert(Some(Arc::new(source)));
                                }
                                (Some(a), Some(b)) if a != b => {
                                    entry.insert(None);
                                }
                                _ => {}
                            }
                        } else {
                            entry.insert(None);
                        }
                    }
                }
            }
        }
    }
}

impl MarkupCatalog {
    /// The physical assembly selected by the build, when its source is unambiguous.
    pub fn authored_assembly(&self, assembly: &AssemblyModel) -> Option<&MarkupAssemblySource> {
        self.sources
            .assemblies
            .get(&(assembly.name.clone(), assembly.crate_name.clone()))?
            .as_ref()
    }

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

fn resolve_rust_file(package: &Path, relative: &Path) -> Option<PathBuf> {
    if !relative_rust_path(relative) {
        return None;
    }
    let file = package.join(relative).canonicalize().ok()?;
    (file.starts_with(package)
        && file.extension().is_some_and(|extension| extension == "rs")
        && file.is_file())
    .then_some(file)
}
