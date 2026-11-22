//! File-backed XAML addresses from the selected compiler output.
use crate::{
    canonical_file_uri, file_path, file_uri, relative_asset_path, resolve_uri, MarkupCatalog,
    PackagedAssets, ProjectError, Result,
};
use ferroui_build_scan::compiled_index::CompiledXamlIndex;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::Arc,
};

type Targets = BTreeMap<String, BTreeSet<usize>>;

#[derive(Clone)]
struct Base {
    group: usize,
    uri: String,
}

/// Logical resource addresses keep their compiler identity when a file moves.
/// Only the physical authoring paths participate in an editor relocation.
#[derive(Clone, Default)]
pub struct CompiledResources {
    package: PathBuf,
    sources: Vec<String>,
    by_input: BTreeMap<String, usize>,
    by_source: BTreeMap<String, usize>,
    bases: Vec<Vec<Base>>,
    groups: Vec<Targets>,
    published: Targets,
    complete: bool,
    packaged_assets: Arc<PackagedAssets>,
}

impl MarkupCatalog {
    pub fn compiled_resources(&self, package: &Path) -> Result<CompiledResources> {
        let manifest = package.join("Cargo.toml").canonicalize()?;
        let mut resources = self.compiled_xaml_output(&manifest)?
            .map(|output| CompiledResources::new(package, &output.index, self))
            .unwrap_or_default();
        resources.packaged_assets = self.packaged_assets().clone();
        Ok(resources)
    }
}

impl CompiledResources {
    pub fn packaged_assets(&self) -> &Arc<PackagedAssets> {
        &self.packaged_assets
    }

    pub fn has_preview_inputs(&self) -> bool {
        !self.is_empty() || !self.packaged_assets.is_empty()
    }

    fn new(package: &Path, index: &CompiledXamlIndex, catalog: &MarkupCatalog) -> Self {
        let Some(assembly) = &index.assembly else {
            return Self::default();
        };
        let published: BTreeSet<_> = catalog
            .models()
            .models()
            .iter()
            .filter(|model| model.name == *assembly)
            .flat_map(|model| &model.documents)
            .filter(|document| document.public)
            .filter_map(|document| {
                let module = document
                    .build_path
                    .as_ref()
                    .or(document.populate_path.as_ref())?
                    .rsplit("::")
                    .nth(1)?;
                Some((module, compiled_resource_key(&document.uri)?))
            })
            .collect();
        let mut resources = Self {
            package: package.to_path_buf(),
            complete: index.groups.iter().all(|group| {
                group.file_backed
                    && group
                        .members
                        .iter()
                        .all(|member| member.source_path.is_some())
            }),
            ..Self::default()
        };
        let root_uri = format!("ferres://{assembly}/");
        for group in index.groups.iter().filter(|group| group.file_backed) {
            let scope = resources.groups.len();
            let mut routes = Targets::new();
            for member in &group.members {
                let Some(path) = &member.source_path else {
                    continue;
                };
                let Some(source) = file_uri(package.join(path))
                    .ok()
                    .and_then(|uri| resources.checked_source(&uri))
                else {
                    continue;
                };
                let address = format!("{root_uri}{}", member.key.trim_start_matches('/'));
                let Ok(uri) = resolve_uri(&root_uri, &address) else {
                    continue;
                };
                let Some(key) = compiled_resource_key(&uri) else {
                    continue;
                };
                let source = resources.add_source(source);
                resources.bases[source].push(Base { group: scope, uri });
                routes.entry(key.clone()).or_default().insert(source);
                if published.contains(&(group.module.as_str(), key.clone())) {
                    resources.published.entry(key).or_default().insert(source);
                }
            }
            resources.groups.push(routes);
        }
        for input in &index.file_inputs {
            if let Some(source) = file_uri(package.join(&input.path))
                .ok()
                .and_then(|uri| resources.checked_source(&uri))
            {
                resources.add_source(source);
            }
        }
        resources.by_input = resources.by_source.clone();
        resources
    }

    fn add_source(&mut self, source: String) -> usize {
        *self.by_source.entry(source.clone()).or_insert_with(|| {
            let id = self.sources.len();
            self.sources.push(source);
            self.bases.push(Vec::new());
            id
        })
    }

    pub fn contains_source(&self, uri: &str) -> bool {
        self.by_source.contains_key(uri)
    }

    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }

    pub fn current_input_source(&self, input: &str) -> Option<&str> {
        let from = self.checked_source(&file_uri(self.package.join(input)).ok()?)?;
        self.by_input
            .get(&from)
            .map(|id| self.sources[*id].as_str())
    }

    pub fn has_compiled_base(&self, source: &str) -> bool {
        self.by_source
            .get(source)
            .is_some_and(|id| !self.bases[*id].is_empty())
    }

    pub fn covers_compiled_sources(&self) -> bool {
        self.complete
    }

    /// A detached application preview needs the original packaged base URI for
    /// relative assets. Resolve an alias only when the file identity is unique.
    pub fn preview_base<'a>(&'a self, owner: &str, fallback: &'a str) -> Result<&'a str> {
        let owner = canonical_file_uri(owner)?;
        let Some(bases) = self.by_source.get(&owner).map(|id| &self.bases[*id]) else {
            return Ok(fallback);
        };
        let Some(first) = bases.first() else {
            return Ok(fallback);
        };
        let key = compiled_resource_key(fallback);
        if let Some(base) = bases.iter().find(|base| compiled_resource_key(&base.uri) == key) {
            return Ok(&base.uri);
        }
        if bases.iter().all(|base| base.uri == first.uri) {
            return Ok(&first.uri);
        }
        Err(ProjectError(format!("The packaged XAML base for {owner} is ambiguous")))
    }

    /// Resolve against the author's actual group(s). A local member wins over the
    /// package's public addresses, including private dictionaries merged by a class.
    pub fn resolve(&self, owner: &str, reference: &str) -> Result<Option<&str>> {
        if self.is_empty() {
            return Ok(None);
        }
        let results = self.targets(owner, reference);
        let known = self.unique(
            results.iter().flatten().flat_map(|ids| ids.iter().copied()),
            reference,
        )?;
        if known.is_some() && results.iter().any(Option::is_none) {
            return Err(ambiguous(reference));
        }
        Ok(known)
    }

    pub fn references(&self, owner: &str, reference: &str, source: &str) -> bool {
        self.by_source.get(source).is_some_and(|id| {
            self.targets(owner, reference)
                .into_iter()
                .flatten()
                .any(|ids| ids.contains(id))
        })
    }

    fn targets(&self, owner: &str, reference: &str) -> Vec<Option<&BTreeSet<usize>>> {
        let lookup = |base: &str, group: Option<usize>| {
            let key = resolve_uri(base, reference)
                .ok()
                .and_then(|uri| compiled_resource_key(&uri))?;
            group
                .and_then(|id| self.groups[id].get(&key))
                .or_else(|| self.published.get(&key))
        };
        let Some(bases) = self
            .by_source
            .get(owner)
            .map(|id| &self.bases[*id])
            .filter(|bases| !bases.is_empty())
        else {
            return vec![lookup(owner, None)];
        };
        bases
            .iter()
            .map(|base| lookup(&base.uri, Some(base.group)))
            .collect()
    }

    fn unique(
        &self,
        mut ids: impl Iterator<Item = usize>,
        reference: &str,
    ) -> Result<Option<&str>> {
        let Some(first) = ids.next() else {
            return Ok(None);
        };
        if ids.any(|id| first != id) {
            return Err(ambiguous(reference));
        }
        Ok(Some(&self.sources[first]))
    }

    /// Retarget only registered paths in the owning package. The input is one
    /// simultaneous editor transaction, so swapping paths is also unambiguous.
    pub fn retarget(&mut self, moves: &BTreeMap<String, String>) {
        if self.sources.is_empty() || moves.is_empty() {
            return;
        }
        let moves: BTreeMap<_, _> = moves
            .iter()
            .filter_map(|(from, to)| Some((self.checked_source(from)?, self.checked_source(to)?)))
            .collect();
        let sources: Vec<_> = self
            .sources
            .iter()
            .map(|source| moves.get(source).cloned().unwrap_or_else(|| source.clone()))
            .collect();
        let by_source: BTreeMap<_, _> = sources
            .iter()
            .cloned()
            .enumerate()
            .map(|(id, uri)| (uri, id))
            .collect();
        if by_source.len() == sources.len() {
            self.sources = sources;
            self.by_source = by_source;
        }
    }

    fn checked_source(&self, uri: &str) -> Option<String> {
        let uri = canonical_file_uri(uri).ok()?;
        let file = file_path(&uri).ok()?;
        if !file.starts_with(&self.package)
            || !file
                .extension()
                .is_some_and(|value| value.eq_ignore_ascii_case("xaml"))
        {
            return None;
        }
        let nested = file
            .ancestors()
            .skip(1)
            .take_while(|path| *path != self.package)
            .any(|path| path.join("Cargo.toml").is_file());
        (!nested).then_some(uri)
    }
}

fn ambiguous(reference: &str) -> ProjectError {
    ProjectError(format!(
        "Compiled XAML groups resolve {reference} to different inputs"
    ))
}

/// Identity used by the compiled loader for a packaged XAML document.
pub fn compiled_resource_key(uri: &str) -> Option<String> {
    if !uri.starts_with("ferres://") {
        return None;
    }
    let normalized = resolve_uri(uri, uri).ok()?;
    let (assembly, path) = normalized.strip_prefix("ferres://")?.split_once('/')?;
    let path = relative_asset_path(path.split(['?', '#']).next()?)?;
    Some(
        assembly
            .chars()
            .chain(['/'])
            .chain(path.chars())
            .map(|character| {
                let mut upper = character.to_uppercase();
                match (upper.next(), upper.next()) {
                    (Some(single), None) => single,
                    _ => character,
                }
            })
            .collect(),
    )
}
