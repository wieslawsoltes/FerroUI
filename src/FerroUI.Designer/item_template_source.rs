//! Resolve an inherited template from the native value in the accepted preview.
use crate::{runtime::node_at_source_info, DesignerSession};
use ferroui_designer_core::{Document, NodeId};
use ferroui_designer_project::PreviewInclude;
use ferroui_designer_tools::{
    ComponentResourceLocation, ItemTemplateMember, ItemTemplateSlot, ItemTemplateValue,
};

pub(crate) struct EffectiveControlTemplate<'a> {
    pub(crate) location: ComponentResourceLocation<'a>,
    pub(crate) include: Option<&'a PreviewInclude>,
}

impl DesignerSession {
    pub(crate) fn local_template_definition(&self, slot: &ItemTemplateSlot) -> Option<NodeId> {
        slot.value.definition().or_else(|| {
            let source = self.effective_control_template(slot)?;
            source.include.is_none().then_some(source.location.node)
        })
    }

    pub(crate) fn effective_control_template(
        &self,
        slot: &ItemTemplateSlot,
    ) -> Option<EffectiveControlTemplate<'_>> {
        if slot.property != ItemTemplateMember::Template
            || !matches!(slot.value, ItemTemplateValue::Unset)
        {
            return None;
        }
        let includes = self.accepted_preview_includes()?;
        let info = self.preview.control_template_source(slot.owner)?;
        let document = self.document();
        if let Some(node) = self
            .preview
            .local_control_template(document, &info)
            .filter(|node| self.compatible_template_at(slot.owner, document, *node))
        {
            return Some(EffectiveControlTemplate {
                location: ComponentResourceLocation {
                    uri: None,
                    address: self.document_uri(),
                    node,
                },
                include: None,
            });
        }

        let uri = info.loader_uri().or_else(|| info.source_uri().map(|source| source.as_str()))?;
        let (address, include) = includes.authored_revision(uri, info.source_revision()?)?;
        let node = node_at_source_info(&include.document, &info, "ControlTemplate")?;
        self.compatible_template_at(slot.owner, &include.document, node)
            .then_some(EffectiveControlTemplate {
                location: ComponentResourceLocation {
                    uri: Some(&include.uri),
                    address,
                    node,
                },
                include: Some(include),
            })
    }

    fn compatible_template_at(&self, owner: NodeId, document: &Document, node: NodeId) -> bool {
        document
            .ast()
            .node(node)
            .is_some_and(|node| self.control_template_matches(owner, document, node))
    }
}
