use ferroui_designer_core::{Document, Node};
use ferroui_designer_project::PreviewIncludes;
use ferroui_designer_tools::{ComponentResourceSources, IncludedResourceDocument};

#[derive(Default)]
pub(crate) struct ResourceSources(pub PreviewIncludes);

pub(crate) struct IncludeLookup<'a>(pub &'a PreviewIncludes);

impl ComponentResourceSources for ResourceSources {
    fn include(
        &self,
        address: &str,
        document: &Document,
        node: &Node,
    ) -> Option<IncludedResourceDocument<'_>> {
        included_document(&self.0, address, document, node)
    }
}

impl ComponentResourceSources for IncludeLookup<'_> {
    fn include(
        &self,
        address: &str,
        document: &Document,
        node: &Node,
    ) -> Option<IncludedResourceDocument<'_>> {
        included_document(self.0, address, document, node)
    }
}

fn included_document<'a>(
    includes: &'a PreviewIncludes,
    address: &str,
    document: &Document,
    node: &Node,
) -> Option<IncludedResourceDocument<'a>> {
    let (address, source) = includes.resolve(address, document, node)?;
    Some(IncludedResourceDocument { address, uri: &source.uri, document: &source.document })
}
