//! Authoring and source rebasing for a private copy of an accepted item template.
use crate::{
    component_session::property_schema, style_composition::supports_palette, DesignerSession,
};
use ferroui_designer_core::{Edit, NodeKind};
use ferroui_designer_language::relocate_xaml_fragment;
use ferroui_designer_tools::{
    IncludedResourceDocument, ItemTemplateChange, ItemTemplateSlot, ItemTemplateValue,
    ResourceCopySources, ToolError,
};

impl DesignerSession {
    pub(crate) fn inline_template_copy_offer(
        &self,
        slot: &ItemTemplateSlot,
    ) -> Option<Result<Vec<Edit>, String>> {
        (slot.reason.is_none() && matches!(slot.value, ItemTemplateValue::Resource { .. }))
            .then(|| self.plan_inline_template_copy(slot))
    }

    pub(crate) fn plan_inline_template_copy(
        &self,
        slot: &ItemTemplateSlot,
    ) -> Result<Vec<Edit>, String> {
        let document = self.document();
        if slot.can_copy_inline(document) {
            return slot
                .plan(document, ItemTemplateChange::CopyInline)
                .map_err(|error| error.to_string());
        }
        let includes = self
            .accepted_preview_includes()
            .ok_or("Wait for the current preview before copying a template.")?;
        let documents = includes
            .authored_sources()
            .map(|(address, include)| IncludedResourceDocument {
                address,
                uri: &include.uri,
                document: &include.document,
            })
            .collect::<Vec<_>>();
        let sources = ResourceCopySources {
            address: self.document_uri(),
            graph: &self.preview.resource_sources,
            includes: &documents,
        };
        let mut owners = Vec::new();
        let mut current = Some(slot.owner);
        while let Some(node) = current.and_then(|node| document.ast().node(node)) {
            if node.kind == NodeKind::Element
                && supports_palette(document, node)
                && self.ensure_layer_manipulable(node.id).is_ok()
            {
                owners.push(node.id);
            }
            current = node.parent;
        }
        slot.plan_included_copy(document, &sources, &owners, &|address, fragment| {
            relocate_xaml_fragment(fragment, property_schema(), address, self.document_uri())
                .map_err(|error| ToolError(error.to_string()))
        })
        .map_err(|error| error.to_string())
    }
}
