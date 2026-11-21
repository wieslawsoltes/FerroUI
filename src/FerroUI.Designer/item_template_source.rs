//! Resolve an inherited template from the native value in the accepted preview.
use crate::{runtime::node_at_source_info, DesignerSession};
use ferroui_designer_core::{Document, NodeId};
use ferroui_designer_project::{file_path, PreviewInclude};
use ferroui_designer_tools::{
    ComponentResourceLocation, ItemTemplateMember, ItemTemplateSlot, ItemTemplateValue,
};
use ferroui_reload_protocol::fingerprint;

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

        let uri = info.source_uri()?.as_str();
        let path = file_path(uri).ok()?;
        let revision = info.source_revision()?;
        let mut matches = includes.authored_sources().filter(|(address, include)| {
            if *address != uri && file_path(address).ok().as_ref() != Some(&path) {
                return false;
            }
            let source = include.document.valid_source();
            fingerprint(source.strip_prefix('\u{feff}').unwrap_or(source).as_bytes()) == revision
        });
        let (address, include) = matches.next()?;
        if matches.next().is_some() {
            return None;
        }
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
