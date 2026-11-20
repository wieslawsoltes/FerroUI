//! Build an immutable, workspace-scoped view of literal XAML includes.
use crate::{file_path, file_uri, resolve_uri};
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
}

pub fn local_preview_sources(
    root: Option<&Path>,
    source_uri: &str,
    source: &Document,
    buffers: &[PreviewSourceBuffer],
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
        visited: BTreeSet::new(),
        active: BTreeSet::from([path]),
        remaining: MAX_GRAPH_BYTES,
    };
    walker.walk(source_uri, source);
    walker.result
}

struct Walker<'a> {
    root: PathBuf,
    buffers: BTreeMap<PathBuf, Option<&'a str>>,
    result: LocalPreviewSources,
    visited: BTreeSet<String>,
    active: BTreeSet<PathBuf>,
    remaining: usize,
}

impl Walker<'_> {
    fn walk(&mut self, source_uri: &str, document: &Document) {
        for node in document.ast().elements() {
            if let Some(reference) = include_reference(document, node) {
                if let Ok(uri) = resolve_uri(source_uri, &reference) {
                    if uri.starts_with("file:") {
                        self.visit(&uri);
                    }
                }
            }
        }
    }

    fn visit(&mut self, uri: &str) {
        let result = self.load(uri);
        if let Err(message) = result {
            self.result.accepted.remove(uri);
            self.result.unavailable.insert(uri.into(), message);
        }
    }

    fn load(&mut self, uri: &str) -> Result<(), String> {
        let path = file_path(uri).map_err(|error| error.to_string())?;
        let path = canonical_path(&path).map_err(|error| error.to_string())?;
        if !path.starts_with(&self.root)
            || !path
                .extension()
                .is_some_and(|suffix| suffix.eq_ignore_ascii_case("xaml"))
        {
            return Err(format!(
                "The include must be a XAML file inside the preview workspace: {uri}"
            ));
        }
        if self.active.contains(&path) {
            return Err(format!("Circular XAML include: {uri}"));
        }
        if self.visited.len() >= MAX_INCLUDES && !self.visited.contains(uri) {
            return Err("The native preview supports up to 128 local XAML includes".into());
        }
        if !self.visited.insert(uri.into()) {
            return Ok(());
        }
        self.result.dependencies.insert(uri.into());
        if let Ok(canonical) = file_uri(&path) {
            self.result.dependencies.insert(canonical);
        }
        let max = self.remaining.min(MAX_DOCUMENT_BYTES);
        let source = match self.buffers.get(&path) {
            Some(Some(source)) => (*source).to_owned(),
            Some(None) => {
                return Err(format!(
                    "The open XAML draft exceeds the preview source limit: {uri}"
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
        self.walk(uri, &document);
        self.active.remove(&path);
        if !self.result.unavailable.contains_key(uri) {
            self.result.accepted.insert(uri.into(), source.into());
        }
        Ok(())
    }
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
