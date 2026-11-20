use super::RuntimePreview;
use ferroui_base::{FerroObject, Visual};
use ferroui_designer_core::{Document, NodeId};
use ferroui_designer_language::{LineIndex, Position};
use ferroui_markup_xaml::diagnostics::XamlSourceInfo;

impl RuntimePreview {
    pub(super) fn preview_source_document<'a>(&'a self, document: &'a Document) -> &'a Document {
        self.design_projection.as_ref().map_or_else(
            || {
                self.definition_projection
                    .as_ref()
                    .map_or(document, |projection| &projection.document)
            },
            |projection| &projection.document,
        )
    }

    /// Styles and themes are native objects without visual bounds. Locate
    /// their declarations through the same provenance and projection maps.
    pub(crate) fn authored_object(
        &self,
        document: &Document,
        object: &FerroObject,
    ) -> Option<NodeId> {
        if let Some(node) = object
            .to_ref()
            .cast::<Visual>()
            .and_then(|visual| self.source_id(&visual))
        {
            return Some(node);
        }
        let info = XamlSourceInfo::for_object(object)?;
        let root = self.root()?;
        let root = XamlSourceInfo::for_object(&root)?;
        if info.source_uri() != root.source_uri() {
            return None;
        }
        let projected = self.preview_source_document(document);
        let source = projected.valid_source();
        let index = LineIndex::new(source);
        let line = u32::try_from(info.line_number().checked_sub(1)?).ok()?;
        let column = usize::try_from(info.line_position().checked_sub(1)?).ok()?;
        let start = index.offset(Position { line, character: 0 }).ok()?;
        let rest = &source[start..];
        let row = rest.split('\n').next()?;
        let type_name = object.get_type().name();
        let candidates = projected
            .ast()
            .elements()
            .filter(|node| {
                (start..start + row.len()).contains(&node.span.start)
                    && node.local_name() == type_name
            })
            .collect::<Vec<_>>();
        if let Some(position) = row
            .char_indices()
            .nth(column)
            .map(|(offset, _)| start + offset)
        {
            if let Some(exact) = candidates
                .iter()
                .find(|node| node.span.start <= position && position <= node.name_span.end)
            {
                return self.authored_identity(exact.id);
            }
        }
        let [node] = candidates.as_slice() else {
            return None;
        };
        self.authored_identity(node.id)
    }
}
