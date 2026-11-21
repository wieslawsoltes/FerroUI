use super::RuntimePreview;
use ferroui_base::{FerroObject, Visual};
use ferroui_controls::primitives::TemplatedControl;
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
        if info.source_uri() != root.source_uri() || info.loader_uri() != root.loader_uri() {
            return None;
        }
        let projected = self.preview_source_document(document);
        self.authored_identity(node_at_source_info(
            projected,
            &info,
            object.get_type().name(),
        )?)
    }

    pub(crate) fn control_template_source(&self, owner: NodeId) -> Option<XamlSourceInfo> {
        let control = self.visual(owner)?.cast::<TemplatedControl>()?;
        XamlSourceInfo::for_control_template(&control.template()?)
    }

    pub(crate) fn local_control_template(
        &self,
        document: &Document,
        info: &XamlSourceInfo,
    ) -> Option<NodeId> {
        let root = self.root()?;
        let root = XamlSourceInfo::for_object(&root)?;
        if info.source_uri() != root.source_uri()
            || info.loader_uri() != root.loader_uri()
            || info.source_revision()? != root.source_revision()?
        {
            return None;
        }
        let projected = self.preview_source_document(document);
        self.authored_identity(node_at_source_info(projected, info, "ControlTemplate")?)
    }
}

pub(crate) fn node_at_source_info(
    document: &Document,
    info: &XamlSourceInfo,
    type_name: &str,
) -> Option<NodeId> {
    let source = document.valid_source();
    let index = LineIndex::new(source);
    let line = u32::try_from(info.line_number().checked_sub(1)?).ok()?;
    let column = usize::try_from(info.line_position().checked_sub(1)?).ok()?;
    let start = index.offset(Position { line, character: 0 }).ok()?;
    let row = source[start..].split('\n').next()?;
    let content = if line == 0 {
        row.trim_start_matches('\u{feff}')
    } else {
        row
    };
    let position = content
        .char_indices()
        .nth(column)
        .map(|(offset, _)| start + row.len() - content.len() + offset);
    let mut candidates = document.ast().elements().filter(|node| {
        (start..start + row.len()).contains(&node.span.start) && node.local_name() == type_name
    });
    let first = candidates.next()?;
    let Some(second) = candidates.next() else {
        return Some(first.id);
    };
    let position = position?;
    [first, second]
        .into_iter()
        .chain(candidates)
        .find(|node| node.span.start <= position && position <= node.name_span.end)
        .map(|node| node.id)
}
