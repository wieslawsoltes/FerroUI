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
}

impl<'a> ResourceScope<'a> {
    pub fn root(
        document: &'a Document,
        address: &'a str,
        sources: Option<&'a dyn ComponentResourceSources>,
    ) -> Self {
        Self { document, address, source_uri: None, sources }
    }

    pub fn location(self, node: NodeId) -> ComponentResourceLocation<'a> {
        ComponentResourceLocation { uri: self.source_uri, address: self.address, node }
    }

    pub fn authored(source: &IncludedResourceDocument<'a>, sources: &'a dyn ComponentResourceSources) -> Self {
        Self { document: source.document, address: source.address, source_uri: Some(source.uri), sources: Some(sources) }
    }

    pub fn include(self, node: &Node) -> Option<Self> {
        let source = self.sources?.include(self.address, self.document, node)?;
        Some(Self {
            document: source.document,
            address: source.address,
            source_uri: Some(source.uri),
            ..self
        })
    }
}
