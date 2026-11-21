//! Authored module files belonging to one selected Cargo target.
mod paths;
mod project;
mod revision;

pub use project::RustModuleProject;

use crate::uri::canonical_path;
use crate::{PreviewSourceBuffer, file_path, file_uri};
use paths::{ModuleDirectory, ModulePath};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use syn::{Item, ItemMod};

const MAX_FILE_BYTES: usize = 1024 * 1024;
const MAX_MODULE_FILES: usize = 128;
const MAX_TOTAL_BYTES: usize = 8 * MAX_FILE_BYTES;

/// Captured text may include unsaved buffers. A module's ranges always refer to
/// its own file and may be used only while that text is still current.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RustModuleSource {
    pub uri: String,
    pub source: Arc<str>,
}

/// The explicit, unconditional source tree of a Cargo target. No Cargo command
/// or application code runs during discovery.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RustModuleSources {
    package_root: PathBuf,
    target: String,
    active: String,
    files: BTreeMap<String, RustModuleSource>,
    edges: BTreeMap<String, BTreeMap<Vec<String>, String>>,
    dependencies: BTreeSet<String>,
}

impl RustModuleSources {
    /// Capture one target's authored modules within its canonical package root.
    /// Returns no mapping when the active file has no unique path from the target.
    pub fn capture(
        package: &Path,
        target: &Path,
        active_uri: &str,
        active_source: &str,
        buffers: &[PreviewSourceBuffer],
    ) -> Option<Self> {
        let root = package.canonicalize().ok()?;
        let directory = ModuleDirectory::file(target, true)?;
        let target = contained_file(&root, target)?;
        let active = contained_file(&root, &file_path(active_uri).ok()?)?;
        let target_uri = file_uri(&target).ok()?;
        let active_uri = file_uri(&active).ok()?;
        let mut drafts = BTreeMap::new();
        for buffer in buffers {
            let Ok(path) = file_path(&buffer.uri) else {
                continue;
            };
            let Some(path) = contained_file(&root, &path) else {
                continue;
            };
            let text = buffer.source.as_deref();
            if drafts
                .insert(path, text)
                .is_some_and(|previous| previous != text)
            {
                return None;
            }
        }
        drafts.insert(active, Some(active_source));
        let mut builder = Builder {
            sources: Self {
                package_root: root.clone(),
                target: target_uri.clone(),
                active: active_uri.clone(),
                files: BTreeMap::new(),
                edges: BTreeMap::new(),
                dependencies: BTreeSet::new(),
            },
            root,
            drafts,
            remaining: MAX_TOTAL_BYTES,
        };
        builder.file(&target, &target_uri, directory)?;
        builder
            .sources
            .files
            .contains_key(&active_uri)
            .then_some(builder.sources)
    }

    pub fn target(&self) -> &RustModuleSource {
        &self.files[&self.target]
    }

    /// The canonical directory whose Cargo manifest defines the captured target.
    pub fn package_root(&self) -> &Path {
        &self.package_root
    }

    pub fn active(&self) -> &RustModuleSource {
        &self.files[&self.active]
    }

    pub fn source(&self, uri: &str) -> Option<&RustModuleSource> {
        self.files.get(uri)
    }

    /// Names of the module declaration and its inline ancestors in `owner`.
    pub fn module_file(&self, owner: &str, modules: &[String]) -> Option<&RustModuleSource> {
        self.source(self.edges.get(owner)?.get(modules)?)
    }

    /// Includes both filename conventions for observed declarations. A newly
    /// created alternate file can make a previously accepted module ambiguous.
    pub fn dependencies(&self) -> impl Iterator<Item = &str> {
        self.dependencies.iter().map(String::as_str)
    }
}

struct Builder<'a> {
    root: PathBuf,
    drafts: BTreeMap<PathBuf, Option<&'a str>>,
    remaining: usize,
    sources: RustModuleSources,
}

impl Builder<'_> {
    fn file(&mut self, path: &Path, uri: &str, directory: ModuleDirectory) -> Option<()> {
        if self.sources.files.len() >= MAX_MODULE_FILES || self.sources.files.contains_key(uri) {
            return None;
        }
        let limit = self.remaining.min(MAX_FILE_BYTES);
        let source = match self.drafts.get(path) {
            Some(draft) => Arc::<str>::from((*draft)?),
            None => Arc::from(read_file(path, limit)?),
        };
        if source.len() > limit {
            return None;
        }
        // `parse_str` uses offsets into the unmodified text including CRLF. A
        // parser that strips a BOM or a shebang would shift those offsets.
        let syntax = syn::parse_str::<syn::File>(&source).ok()?;
        if conditional(&syntax.attrs) {
            return None;
        }
        self.remaining -= source.len();
        self.sources.dependencies.insert(uri.into());
        self.sources.files.insert(
            uri.into(),
            RustModuleSource {
                uri: uri.into(),
                source,
            },
        );
        self.modules(uri, &directory, &syntax.items, &mut Vec::new())
    }

    fn modules(
        &mut self,
        owner: &str,
        directory: &ModuleDirectory,
        items: &[Item],
        names: &mut Vec<String>,
    ) -> Option<()> {
        use syn::ext::IdentExt;
        let mut counts = BTreeMap::new();
        for item in items {
            if let Item::Mod(item) = item {
                *counts.entry(item.ident.unraw().to_string()).or_insert(0) += 1;
            }
        }
        for item in items {
            let Item::Mod(item) = item else { continue };
            let name = item.ident.unraw().to_string();
            if counts[&name] != 1 || conditional(&item.attrs) {
                continue;
            }
            names.push(name);
            if let Some((_, items)) = &item.content {
                self.modules(owner, &directory.inline(item)?, items, names)?;
            } else if let Some((path, directory)) = self.outline(directory, item) {
                let uri = file_uri(&path).ok()?;
                self.file(&path, &uri, directory)?;
                self.sources
                    .edges
                    .entry(owner.into())
                    .or_default()
                    .insert(names.clone(), uri);
            }
            names.pop();
        }
        Some(())
    }

    fn outline(
        &mut self,
        directory: &ModuleDirectory,
        item: &ItemMod,
    ) -> Option<(PathBuf, ModuleDirectory)> {
        let (first, second) = directory.files(item)?;
        let first = self.available(first)?;
        let second = match second {
            Some(module) => self.available(module)?,
            None => None,
        };
        match (first, second) {
            (Some(module), None) | (None, Some(module)) => Some(module),
            _ => None,
        }
    }

    fn available(&mut self, module: ModulePath) -> Option<Option<(PathBuf, ModuleDirectory)>> {
        let path = contained_file(&self.root, &module.path)?;
        self.sources.dependencies.insert(file_uri(&path).ok()?);
        if module.path.starts_with(&self.root) {
            self.sources
                .dependencies
                .insert(file_uri(&module.path).ok()?);
        }
        let directory = ModuleDirectory::file(&module.path, module.own_directory)?;
        Some((self.drafts.contains_key(&path) || path.is_file()).then_some((path, directory)))
    }
}

fn conditional(attributes: &[syn::Attribute]) -> bool {
    attributes
        .iter()
        .any(|attribute| attribute.path().is_ident("cfg") || attribute.path().is_ident("cfg_attr"))
}

fn contained_file(root: &Path, path: &Path) -> Option<PathBuf> {
    let path = canonical_path(path).ok()?;
    (path.starts_with(root) && path.extension()?.eq_ignore_ascii_case("rs")).then_some(path)
}

fn read_file(path: &Path, limit: usize) -> Option<String> {
    if !path.is_file() {
        return None;
    }
    let mut source = String::new();
    File::open(path)
        .ok()?
        .take(limit as u64 + 1)
        .read_to_string(&mut source)
        .ok()?;
    Some(source)
}
