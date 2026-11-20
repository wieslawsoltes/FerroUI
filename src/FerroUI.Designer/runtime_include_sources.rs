use ferroui_designer_core::{Document, Node};
use ferroui_designer_project::PreviewIncludes;
use ferroui_designer_tools::{ComponentResourceSources, IncludedResourceDocument};

#[derive(Default)]
pub(crate) struct ResourceSources(pub PreviewIncludes);

impl ComponentResourceSources for ResourceSources {
    fn include(
        &self,
        address: &str,
        document: &Document,
        node: &Node,
    ) -> Option<IncludedResourceDocument<'_>> {
        let (address, source) = self.0.resolve(address, document, node)?;
        Some(IncludedResourceDocument {
            address,
            uri: &source.uri,
            document: &source.document,
        })
    }
}
