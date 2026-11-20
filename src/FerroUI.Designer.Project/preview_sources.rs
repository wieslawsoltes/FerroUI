//! Build an immutable, workspace-scoped view of literal XAML includes.
use crate::{file_path, file_uri, resolve_uri, CompiledResources};
use ferroui_designer_core::{Document, MarkupValue, Node, NodeKind};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

const MAX_DOCUMENT_BYTES: usize = 1024 * 1024;
const MAX_GRAPH_BYTES: usize = 8 * MAX_DOCUMENT_BYTES;
const MAX_INCLUDES: usize = 128;

#[derive(Clone, Deserialize)]
pub struct PreviewSourceBuffer {
    pub uri: String,
    pub source: Option<String>,
}

/// URI keys retain the loader's lexical identity. Watched paths use the editor's
/// canonical file identity as well so symlink aliases share one invalidation.
#[derive(Default)]
pub struct LocalPreviewSources {
    pub accepted: BTreeMap<String, String>,
    pub unavailable: BTreeMap<String, String>,
    pub dependencies: BTreeSet<String>,
    pub includes: PreviewIncludes,
    pub uses_drafts: bool,
}

/// Accepted include addresses and the authored documents that supplied them.
/// The loader address may differ from the physical file in a compiled group.
#[derive(Clone, Default)]
pub struct PreviewIncludes {
    documents: BTreeMap<String, PreviewInclude>,
}

#[derive(Clone)]
pub struct PreviewInclude {
    pub uri: String,
    pub document: Document,
}

impl PreviewIncludes {
    pub fn documents(&self) -> impl Iterator<Item = &PreviewInclude> {
        self.documents.values()
    }

    pub fn authored_sources(&self) -> impl Iterator<Item = (&str, &PreviewInclude)> {
        self.documents.iter().map(|(address, source)| (address.as_str(), source))
    }

    pub fn resolve(
        &self,
        address: &str,
        document: &Document,
        node: &Node,
    ) -> Option<(&str, &PreviewInclude)> {
        let reference = include_reference(document, node)?;
        let address = resolve_uri(address, &reference).ok()?;
        let (address, include) = self.documents.get_key_value(&address)?;
        Some((address.as_str(), include))
    }
}

pub fn local_preview_sources(
    root: Option<&Path>,
    source_uri: &str,
    source: &Document,
    buffers: &[PreviewSourceBuffer],
) -> LocalPreviewSources {
    local_preview_sources_with_resources(root, source_uri, source, buffers, None)
}

pub fn local_preview_sources_with_resources(
    root: Option<&Path>,
    source_uri: &str,
    source: &Document,
    buffers: &[PreviewSourceBuffer],
    resources: Option<&CompiledResources>,
) -> LocalPreviewSources {
    preview_sources(root, source_uri, source, buffers, resources, None)
}

/// Traverse the files backing an application loader's resource tree.
pub fn hosted_preview_sources(
    root: &Path,
    source_uri: &str,
    base_uri: &str,
    source: &Document,
    buffers: &[PreviewSourceBuffer],
    resources: Option<&CompiledResources>,
) -> LocalPreviewSources {
    preview_sources(Some(root), source_uri, source, buffers, resources, Some(base_uri))
}

fn preview_sources(
    root: Option<&Path>,
    source_uri: &str,
    source: &Document,
    buffers: &[PreviewSourceBuffer],
    resources: Option<&CompiledResources>,
    hosted_base: Option<&str>,
) -> LocalPreviewSources {
    let Ok(path) = file_path(source_uri) else {
        return LocalPreviewSources::default();
    };
    let Some(root) = root
        .or_else(|| path.parent())
        .and_then(|root| root.canonicalize().ok())
    else {
        return LocalPreviewSources::default();
    };
    let Ok(path) = canonical_path(&path) else {
        return LocalPreviewSources::default();
    };
    if !path.starts_with(&root) {
        return LocalPreviewSources::default();
    }
    let buffers = buffers
        .iter()
        .filter_map(|buffer| {
            let path = canonical_path(&file_path(&buffer.uri).ok()?).ok()?;
            path.starts_with(&root)
                .then_some((path, buffer.source.as_deref()))
        })
        .collect();
    let mut walker = Walker {
        root,
        buffers,
        result: LocalPreviewSources::default(),
        visited: BTreeMap::new(),
        active: BTreeSet::from([path.clone()]),
        remaining: MAX_GRAPH_BYTES,
        resources,
        hosted: hosted_base.is_some(),
    };
    walker.walk(hosted_base.unwrap_or(source_uri), &path, source);
    walker.result
}

struct Walker<'a> {
    root: PathBuf,
    buffers: BTreeMap<PathBuf, Option<&'a str>>,
    result: LocalPreviewSources,
    visited: BTreeMap<String, PathBuf>,
    active: BTreeSet<PathBuf>,
    remaining: usize,
    resources: Option<&'a CompiledResources>,
    hosted: bool,
}

impl Walker<'_> {
    fn walk(&mut self, loader_uri: &str, source_path: &Path, document: &Document) {
        let Ok(owner) = file_uri(source_path) else {
            return;
        };
        for node in document.ast().elements() {
            let Some(reference) = include_reference(document, node) else {
                continue;
            };
            let Ok(uri) = resolve_uri(loader_uri, &reference) else {
                continue;
            };
            let mapped = self
                .resources
                .map_or(Ok(None), |resources| resources.resolve(&owner, &reference));
            let path = match mapped {
                Ok(Some(source)) => file_path(source),
                Ok(None) if uri.starts_with("file:") => file_path(&uri),
                Ok(None) if self.hosted && uri.starts_with("ferres://") => {
                    let Some(path) = self.relative_hosted_source(&owner, &reference) else {
                        continue;
                    };
                    Ok(path)
                }
                Ok(None) => continue,
                Err(error) => Err(error),
            };
            let result = path
                .map_err(|error| error.to_string())
                .and_then(|path| self.load(&uri, &path));
            if let Err(message) = result {
                self.result.accepted.remove(&uri);
                self.result.includes.documents.remove(&uri);
                self.result.unavailable.insert(uri, message);
            }
        }
    }

    fn relative_hosted_source(&mut self, owner: &str, reference: &str) -> Option<PathBuf> {
        // A literal relative project include can also refer to an embedded asset.
        // Watch the physical candidate and leave unmapped assets to the app loader.
        if reference.starts_with('/') {
            return None;
        }
        let uri = resolve_uri(owner, reference).ok()?;
        let path = canonical_path(&file_path(&uri).ok()?).ok()?;
        if !path.starts_with(&self.root) || !is_xaml(&path) {
            return None;
        }
        self.result.dependencies.insert(uri);
        self.result.dependencies.insert(file_uri(&path).ok()?);
        (path.is_file() || self.buffers.contains_key(&path)).then_some(path)
    }

    fn load(&mut self, uri: &str, requested: &Path) -> Result<(), String> {
        let path = canonical_path(requested).map_err(|error| error.to_string())?;
        if !path.starts_with(&self.root) || !is_xaml(&path) {
            return Err(format!(
                "The include must be a XAML file inside the preview workspace: {uri}"
            ));
        }
        if self.active.contains(&path) {
            return Err(format!("Circular XAML include: {uri}"));
        }
        if let Some(previous) = self.visited.get(uri) {
            return if previous == &path {
                Ok(())
            } else {
                Err(format!(
                    "Compiled XAML groups resolve {uri} to different inputs"
                ))
            };
        }
        if self.visited.len() >= MAX_INCLUDES {
            return Err("The native preview supports up to 128 local XAML includes".into());
        }
        self.visited.insert(uri.into(), path.clone());
        if file_path(uri).is_ok_and(|alias| alias == requested) {
            self.result.dependencies.insert(uri.into());
        }
        let canonical = file_uri(&path).map_err(|error| error.to_string())?;
        self.result.dependencies.insert(canonical.clone());
        let max = self.remaining.min(MAX_DOCUMENT_BYTES);
        self.result.uses_drafts |= self.buffers.contains_key(&path);
        let source = match self.buffers.get(&path) {
            Some(Some(source)) => (*source).to_owned(),
            Some(None) => {
                return Err(format!(
                    "The open XAML draft is unavailable for preview: {uri}"
                ))
            }
            None => {
                if !path.is_file() {
                    return Err(format!("The included XAML file is unavailable: {uri}"));
                }
                let mut source = String::new();
                File::open(&path)
                    .and_then(|file| file.take((max + 1) as u64).read_to_string(&mut source))
                    .map_err(|error| format!("Could not read included XAML {uri}: {error}"))?;
                source
            }
        };
        if source.len() > max {
            return Err(format!(
                "The included XAML exceeds the preview source limit: {uri}"
            ));
        }
        self.remaining -= source.len();
        let source = source.strip_prefix('\u{feff}').unwrap_or(&source);
        let document = Document::new(source)
            .map_err(|error| format!("Repair included XAML {uri}: {error}"))?;
        self.active.insert(path.clone());
        self.walk(uri, &path, &document);
        self.active.remove(&path);
        if !self.result.unavailable.contains_key(uri) {
            self.result.accepted.insert(uri.into(), source.into());
            self.result.includes.documents.insert(uri.into(), PreviewInclude {
                uri: canonical,
                document,
            });
        }
        Ok(())
    }
}

fn is_xaml(path: &Path) -> bool {
    path.extension().is_some_and(|suffix| suffix.eq_ignore_ascii_case("xaml"))
}

fn canonical_path(path: &Path) -> crate::Result<PathBuf> {
    crate::uri::canonical_path(path)
}

fn include_reference(document: &Document, node: &Node) -> Option<String> {
    if !matches!(
        node.local_name(),
        "ResourceInclude" | "MergeResourceInclude" | "StyleInclude"
    ) || !matches!(
        node.namespace_uri.as_deref(),
        Some(
            "https://github.com/ferroui"
                | "using:FerroUI.Markup.Xaml.Styling"
                | "clr-namespace:FerroUI.Markup.Xaml.Styling;assembly=FerroUI.Markup.Xaml"
        )
    ) {
        return None;
    }
    let qualified = format!("{}.Source", node.local_name());
    let attribute = node.attributes.iter().find(|attribute| {
        attribute.name == "Source"
            || (attribute.name.rsplit(':').next() == Some(qualified.as_str())
                && attribute.namespace_uri == node.namespace_uri)
    });
    let value = if let Some(attribute) = attribute {
        attribute.markup().ok()?
    } else {
        let property = node
            .children
            .iter()
            .filter_map(|id| document.ast().node(*id))
            .find(|child| {
                child.kind == NodeKind::PropertyElement
                    && child.local_name() == qualified
                    && child.namespace_uri == node.namespace_uri
            })?;
        let mut text = String::new();
        for child in property
            .children
            .iter()
            .filter_map(|id| document.ast().node(*id))
        {
            match child.kind {
                NodeKind::Text | NodeKind::CData => {
                    text.push_str(child.text.as_deref().unwrap_or_default())
                }
                NodeKind::Comment => {}
                _ => return None,
            }
        }
        MarkupValue::parse(text.trim()).ok()?
    };
    match value {
        MarkupValue::Literal(value) if !value.trim().is_empty() => Some(value.trim().into()),
        _ => None,
    }
}
