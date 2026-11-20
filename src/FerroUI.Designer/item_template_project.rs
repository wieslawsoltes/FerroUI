//! Navigate from a consuming control to a template in an accepted dictionary.
use crate::item_template_samples::{assigned_fixture, ItemTemplatePreview};
use crate::{DefinitionPreviewOptions, DesignerSession, PreviewStatus};
use ferroui_designer_core::{
    design_fixtures::{DesignFixture, DesignFixtures},
    Document, NodeId, SourceSpan,
};
use ferroui_designer_language::{LineIndex, Range};
use ferroui_designer_project::PreviewInclude;
use ferroui_designer_tools::{
    component_resource_location, keyed_item_template, ComponentResourceContext, ItemTemplateSlot,
    ItemTemplateValue,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    hash::{DefaultHasher, Hash, Hasher},
};

/// A checked source locator, independent of ephemeral node IDs in the consumer.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct IncludedItemTemplate {
    pub uri: String,
    pub range: Range,
    pub key: String,
    source_stamp: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct IncludedTemplatePreview {
    pub source: IncludedItemTemplate,
    pub sample: Option<DesignFixture>,
    pub consumer: String,
}

#[derive(Serialize)]
pub(crate) struct IncludedTemplateChoice {
    pub uri: String,
    pub key: String,
}

impl IncludedItemTemplate {
    pub fn node_in(&self, document: &Document, uri: &str) -> Result<(NodeId, SourceSpan), String> {
        let text = document.source();
        let source = text.strip_prefix('\u{feff}').unwrap_or(text);
        if ferroui_designer_project::canonical_file_uri(uri)
            .ok()
            .as_ref()
            != Some(&self.uri)
            || !document.is_valid()
            || self.source_stamp != source_stamp(source)
        {
            return Err(
                "The included template source changed. Reopen Item Templates from its consumer."
                    .into(),
            );
        }
        let span = LineIndex::new(source)
            .span(self.range)
            .map_err(|error| error.to_string())?;
        let bom = text.len() - source.len();
        let span = SourceSpan::new(span.start + bom, span.end + bom);
        let ast = document.ast();
        let node = ast
            .element_at(span.start)
            .and_then(|id| ast.node(id))
            .filter(|node| {
                node.name_span == span && keyed_item_template(node).as_ref() == Some(&self.key)
            })
            .ok_or("The template declaration changed.")?;
        Ok((node.id, span))
    }
}

impl DesignerSession {
    pub(crate) fn included_template(
        &self,
        owner: NodeId,
        key: &str,
        theme: Option<&ComponentResourceContext>,
    ) -> Result<Option<IncludedItemTemplate>, String> {
        let Some((source, node)) = self.included_template_node(owner, key, theme)? else {
            return Ok(None);
        };
        Ok(Some(IncludedItemTemplate {
            uri: source.uri.clone(),
            range: LineIndex::new(source.document.source())
                .range(node.name_span)
                .map_err(|error| error.to_string())?,
            key: key.to_owned(),
            source_stamp: source_stamp(source.document.source()),
        }))
    }

    pub(crate) fn included_template_node(
        &self,
        owner: NodeId,
        key: &str,
        theme: Option<&ComponentResourceContext>,
    ) -> Result<Option<(&PreviewInclude, &ferroui_designer_core::Node)>, String> {
        let document = self.document();
        if !matches!(self.preview_status(), PreviewStatus::Ready { revision } if *revision == document.revision())
        {
            return Ok(None);
        }
        let sources = &self.preview.resource_sources;
        let location =
            component_resource_location(document, self.document_uri(), owner, key, theme, sources)
                .map_err(|error| error.to_string())?;
        let Some(location) = location.filter(|location| location.uri.is_some()) else {
            return Ok(None);
        };
        let Some(source) = sources
            .0
            .documents()
            .find(|source| Some(source.uri.as_str()) == location.uri)
        else {
            return Ok(None);
        };
        Ok(source
            .document
            .ast()
            .node(location.node)
            .filter(|node| keyed_item_template(node).as_deref() == Some(key))
            .map(|node| (source, node)))
    }

    pub(crate) fn assigned_included_template(
        &self,
        slot: &ItemTemplateSlot,
    ) -> Result<Option<IncludedItemTemplate>, String> {
        let ItemTemplateValue::Resource { key, .. } = &slot.value else {
            return Ok(None);
        };
        self.included_template(
            slot.owner,
            key,
            self.component_resource_context(slot.owner).as_ref(),
        )
    }

    pub(crate) fn included_template_resources(
        &self,
        owner: NodeId,
    ) -> Result<Vec<IncludedTemplateChoice>, String> {
        let keys = self
            .preview
            .resource_sources
            .0
            .documents()
            .flat_map(|source| {
                source
                    .document
                    .ast()
                    .elements()
                    .filter_map(keyed_item_template)
            })
            .collect::<BTreeSet<_>>();
        keys.into_iter()
            .filter_map(|key| {
                self.included_template_node(owner, &key, None)
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
        let Some(source) = self.assigned_included_template(slot)? else {
            return Ok(None);
        };
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
        Ok(Some(IncludedTemplatePreview {
            source,
            sample,
            consumer: self.document_uri().to_owned(),
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
        };
        self.enter_definition(node, DefinitionPreviewOptions::default(), Some(item))
    }

    pub(crate) fn included_template_consumer(&self) -> Option<&str> {
        match self.definition_scope()?.item_template.as_ref()? {
            ItemTemplatePreview::Included { consumer, .. } => Some(consumer),
            ItemTemplatePreview::Assigned { .. } => None,
        }
    }
}

fn source_stamp(source: &str) -> String {
    let mut hash = DefaultHasher::new();
    source.hash(&mut hash);
    format!("{:016x}", hash.finish())
}
