//! Authoring and source rebasing for a private copy of an accepted item template.
use crate::{
    component_session::property_schema, style_composition::supports_palette, DesignerSession,
};
use ferroui_designer_core::{Edit, NodeKind};
use ferroui_designer_language::relocate_xaml_fragment;
use ferroui_designer_tools::{
    IncludedResourceDocument, ItemTemplateChange, ItemTemplateMember, ItemTemplateSlot, ItemTemplateValue,
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
        if slot.property == ItemTemplateMember::Template {
            let source = if let Some(definition) = slot.value.definition() {
                document.ast().node(definition).map(|node| (document, node))
            } else if let ItemTemplateValue::Resource { key, .. } = &slot.value {
                self.included_template_node(slot.owner, key, slot.property, None)?.map(|(include, node)| (&include.document, node))
            } else { None };
            if !source.is_some_and(|(document, template)| self.control_template_matches(slot.owner, document, template)) {
                return Err("Choose a compatible control template before making an inline copy".into());
            }
        }
        let Some(includes) = self.accepted_preview_includes() else {
            if slot.can_copy_inline(document) {
                return slot
                    .plan(document, ItemTemplateChange::CopyInline)
                    .map_err(|error| error.to_string());
            }
            return Err("Wait for the current preview before copying a template.".into());
        };
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
        slot.plan_resource_copy(document, &sources, &owners, &|address, fragment| {
            relocate_xaml_fragment(fragment, property_schema(), address, self.document_uri())
                .map_err(|error| ToolError(error.to_string()))
        })
        .map_err(|error| error.to_string())
    }
}
