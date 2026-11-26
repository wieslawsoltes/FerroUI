//! Source identities for workspace-scoped resource lookup.
use super::*;

/// A host supplies the same accepted include edges used to render its preview.
pub trait ComponentResourceSources {
    fn include(
        &self,
        address: &str,
        document: &Document,
        node: &Node,
    ) -> Option<IncludedResourceDocument<'_>>;
}

pub struct IncludedResourceDocument<'a> {
    pub address: &'a str,
    pub uri: &'a str,
    pub document: &'a Document,
}

/// Node IDs are local to a source document. A missing URI denotes the active document.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ComponentResourceLocation<'a> {
    pub uri: Option<&'a str>,
    /// Logical loader address, including for a compiled alias of a physical file.
    pub address: &'a str,
    pub node: NodeId,
}

#[derive(Clone, Copy)]
pub(super) struct ResourceScope<'a> {
    pub document: &'a Document,
    pub address: &'a str,
    source_uri: Option<&'a str>,
    sources: Option<&'a dyn ComponentResourceSources>,
    mounted_theme: Option<ThemeDeclaration<'a>>,
    pub track_value: bool,
}

/// An included provider inherits the nearest authored theme from its mount point.
#[derive(Clone, Copy)]
pub(super) struct ThemeDeclaration<'a> {
    pub document: &'a Document,
    pub node: &'a Node,
}

impl<'a> ResourceScope<'a> {
    pub fn root(
        document: &'a Document,
        address: &'a str,
        sources: Option<&'a dyn ComponentResourceSources>,
    ) -> Self {
        Self { document, address, source_uri: None, sources, mounted_theme: None, track_value: false }
    }

    pub fn location(self, node: NodeId) -> ComponentResourceLocation<'a> {
        ComponentResourceLocation { uri: self.source_uri, address: self.address, node }
    }

    pub fn authored(source: &IncludedResourceDocument<'a>, sources: &'a dyn ComponentResourceSources) -> Self {
        Self { document: source.document, address: source.address, source_uri: Some(source.uri), sources: Some(sources), mounted_theme: None, track_value: false }
    }

    pub fn include(self, node: &Node) -> Option<Self> {
        let source = self.sources?.include(self.address, self.document, node)?;
        Some(Self {
            document: source.document,
            address: source.address,
            source_uri: Some(source.uri),
            mounted_theme: self.track_value.then(|| self.theme_at(node.id)).flatten(),
            ..self
        })
    }

    pub fn theme_at(self, at: NodeId) -> Option<ThemeDeclaration<'a>> {
        let document = self.document;
        let mut current = Some(at);
        while let Some(node) = current.and_then(|id| document.ast().node(id)) {
            current = node.parent;
            if current.and_then(|id| document.ast().node(id)).is_some_and(|parent| {
                references::dictionary_property(document, parent, "ThemeDictionaries")
            }) {
                return Some(ThemeDeclaration { document, node });
            }
        }
        self.mounted_theme
    }
}
