//! Navigate from a consuming control to a template in an accepted dictionary.
use crate::included_source::{IncludedSource, IncludedSourceIndex};
use crate::item_template_samples::{assigned_fixture, ItemTemplatePreview};
use crate::{DefinitionPreviewOptions, DesignerSession};
use ferroui_designer_core::{
    design_fixtures::{DesignFixture, DesignFixtures},
    Document, Node, NodeId, SourceSpan,
};
use ferroui_designer_project::PreviewInclude;
use ferroui_designer_tools::{
    component_resource_location, keyed_template, ComponentResourceContext, ItemTemplateMember,
    ItemTemplateSlot, ItemTemplateValue,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// A checked source locator, independent of ephemeral node IDs in the consumer.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct IncludedItemTemplate {
    #[serde(flatten)]
    pub location: IncludedSource,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub property: Option<ItemTemplateMember>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct IncludedTemplatePreview {
    pub source: IncludedItemTemplate,
    pub sample: Option<DesignFixture>,
    pub consumer: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_type: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct IncludedTemplateChoice {
    pub uri: String,
    pub key: String,
}

impl IncludedItemTemplate {
    fn locate(source: &PreviewInclude, node: &Node, property: ItemTemplateMember) -> Result<Self, String> {
        Ok(Self {
            location: IncludedSourceIndex::new(source).locate(node)?,
            key: keyed_template(node, property),
            property: (property == ItemTemplateMember::Template).then_some(property),
        })
    }

    pub fn node_in(&self, document: &Document, uri: &str) -> Result<(NodeId, SourceSpan), String> {
        let node = self.location.node_in(document, uri)?;
        let property = self.property.unwrap_or(ItemTemplateMember::ItemTemplate);
        if !property.accepts(node) || keyed_template(node, property) != self.key {
            return Err("The template declaration changed.".into());
        }
        Ok((node.id, node.name_span))
    }
}

impl DesignerSession {
    pub(crate) fn included_template_node(
        &self,
        owner: NodeId,
        key: &str,
        property: ItemTemplateMember,
        theme: Option<&ComponentResourceContext>,
    ) -> Result<Option<(&PreviewInclude, &ferroui_designer_core::Node)>, String> {
        let document = self.document();
        let Some(includes) = self.accepted_preview_includes() else {
            return Ok(None);
        };
        let sources = &self.preview.resource_sources;
        let location =
            component_resource_location(document, self.document_uri(), owner, key, theme, sources)
                .map_err(|error| error.to_string())?;
        let Some(location) = location.filter(|location| location.uri.is_some()) else {
            return Ok(None);
        };
        let Some(source) = includes
            .documents()
            .find(|source| Some(source.uri.as_str()) == location.uri)
        else {
            return Ok(None);
        };
        Ok(source
            .document
            .ast()
            .node(location.node)
            .filter(|node| keyed_template(node, property).as_deref() == Some(key)
                && (property != ItemTemplateMember::Template || self.control_template_matches(owner, &source.document, node)))
            .map(|node| (source, node)))
    }

    fn included_slot_definition(
        &self,
        slot: &ItemTemplateSlot,
    ) -> Result<Option<(&PreviewInclude, &Node)>, String> {
        if let ItemTemplateValue::Resource { key, .. } = &slot.value {
            return self.included_template_node(
                slot.owner, key, slot.property, self.component_resource_context(slot.owner).as_ref(),
            );
        }
        Ok(self.effective_control_template(slot).and_then(|template| {
            let include = template.include?;
            include.document.ast().node(template.location.node).map(|node| (include, node))
        }))
    }

    pub(crate) fn included_template_definition(&self, slot: &ItemTemplateSlot) -> Result<Option<IncludedItemTemplate>, String> {
        self.included_slot_definition(slot)?
            .map(|(include, node)| IncludedItemTemplate::locate(include, node, slot.property))
            .transpose()
    }

    pub(crate) fn included_template_resources(
        &self,
        owner: NodeId,
        property: ItemTemplateMember,
    ) -> Result<Vec<IncludedTemplateChoice>, String> {
        let Some(includes) = self.accepted_preview_includes() else {
            return Ok(Vec::new());
        };
        let keys = includes
            .documents()
            .flat_map(|source| {
                source
                    .document
                    .ast()
                    .elements()
                    .filter_map(move |node| keyed_template(node, property))
            })
            .collect::<BTreeSet<_>>();
        keys.into_iter()
            .filter_map(|key| {
                self.included_template_node(owner, &key, property, None)
                    .map(|found| {
                        found.map(|(source, _)| IncludedTemplateChoice {
                            uri: source.uri.clone(),
                            key,
                        })
                    })
                    .transpose()
            })
            .collect()
    }

    pub(crate) fn included_item_template_preview(
        &self,
        slot: &ItemTemplateSlot,
    ) -> Result<Option<IncludedTemplatePreview>, String> {
        let Some((include, node)) = self.included_slot_definition(slot)? else {
            return Ok(None);
        };
        let source = IncludedItemTemplate::locate(include, node, slot.property)?;
        let sample = if self.design_time_enabled() {
            assigned_fixture(
                self.document(),
                slot.owner,
                slot.property,
                self.design_fixture(),
            )?
        } else {
            None
        };
        let target_type = self.template_preview_options(slot.owner, slot.property, &include.document, node.id)?.target_type;
        Ok(Some(IncludedTemplatePreview {
            source,
            sample,
            consumer: self.document_uri().to_owned(),
            target_type,
        }))
    }

    pub(crate) fn enter_included_item_template(
        &mut self,
        target: IncludedTemplatePreview,
    ) -> Result<(), String> {
        let (node, _) = target
            .source
            .node_in(self.document(), self.document_uri())?;
        if let Some(sample) = &target.sample {
            let manifest = DesignFixtures {
                default: sample.name.clone(),
                fixtures: vec![sample.clone()],
            };
            DesignFixtures::parse(
                &serde_json::to_string(&manifest).map_err(|error| error.to_string())?,
            )?;
            if sample.data.len() != 1 || !sample.data.contains_key("Item") {
                return Err("The template sample must contain one Item.".into());
            }
        }
        let item = ItemTemplatePreview::Included {
            key: target.source.key,
            sample: target.sample,
            consumer: target.consumer,
            property: target.source.property.unwrap_or(ItemTemplateMember::ItemTemplate),
        };
        self.enter_definition(node, DefinitionPreviewOptions { target_type: target.target_type, ..DefinitionPreviewOptions::default() }, Some(item))
    }

    pub(crate) fn included_template_consumer(&self) -> Option<&str> {
        match self.definition_scope()?.item_template.as_ref()? {
            ItemTemplatePreview::Included { consumer, .. } => Some(consumer),
            ItemTemplatePreview::Assigned { .. } => None,
        }
    }
}
