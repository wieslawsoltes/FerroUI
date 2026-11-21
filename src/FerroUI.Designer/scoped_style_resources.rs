//! Resolve qualified style resources and inheritance in an accepted include graph.
use crate::runtime_include_sources::IncludeLookup;
use crate::scoped_style_fields::StyleField;
use ferroui_designer_core::{Document, NodeId};
use ferroui_designer_language::{Schema, FRAMEWORK_NAMESPACE};
use ferroui_designer_project::PreviewIncludes;
use ferroui_designer_tools::{
    component_resource_location, component_styles::authored_base_key, scoped_resource_resolution,
    ComponentCatalog, ComponentKind,
};
use serde_json::{json, Value};
use std::collections::BTreeSet;

pub(crate) struct StyleResources<'a> {
    pub document: &'a Document,
    pub address: &'a str,
    pub includes: &'a PreviewIncludes,
    pub schema: &'a Schema,
}

impl StyleResources<'_> {
    pub(crate) fn bases(&self, node: NodeId) -> Result<Vec<Value>, String> {
        Ok(self
            .keys(Some(ComponentKind::ControlTheme))?
            .into_iter()
            .filter_map(|key| {
                let ty = self.base_type(node, &key).ok()?;
                Some(json!({"node":key,"name":key,"type_name":ty}))
            })
            .collect())
    }

    pub(crate) fn base_type(&self, node: NodeId, key: &str) -> Result<String, String> {
        // Inheritance must be valid after an appearance change as well as in the current frame.
        let sources = IncludeLookup(self.includes);
        let mut visited = BTreeSet::from([(self.address, node)]);
        let mut at = (self.address, self.document, node);
        let mut target = self
            .schema
            .style_target(self.document, node)
            .ok_or("Resolve the control theme target type first")?;
        let mut key = key.to_owned();
        let mut first = None;
        loop {
            let location = component_resource_location(at.1, at.0, at.2, &key, None, &sources)
                .map_err(|error| error.to_string())?
                .ok_or("Choose a base theme that resolves unambiguously in this scope")?;
            if !visited.insert((location.address, location.node)) {
                return Err("BasedOn would create a control theme cycle".into());
            }
            let document = self
                .at(location.address)
                .ok_or("The base theme source is unavailable")?;
            let entry = document
                .ast()
                .node(location.node)
                .ok_or("The base theme declaration changed")?;
            if !self
                .schema
                .type_info(&entry.name, entry.namespace_uri.as_deref())
                .is_some_and(|ty| ty.name == "ControlTheme" && ty.namespace == FRAMEWORK_NAMESPACE)
            {
                return Err("Choose an authored ControlTheme as the base".into());
            }
            let base = self
                .schema
                .style_target(document, entry.id)
                .ok_or("Resolve the base control theme target type first")?;
            if !self.schema.type_assignable_to(target, base) {
                return Err("Choose a base theme for the same control type or an ancestor".into());
            }
            first.get_or_insert_with(|| base.name.clone());
            let Some(next) =
                authored_base_key(document, entry.id).map_err(|error| error.to_string())?
            else {
                break;
            };
            target = base;
            at = (location.address, document, entry.id);
            key = next;
        }
        first.ok_or_else(|| "Choose a compatible control theme".into())
    }

    pub(crate) fn choices(&self, node: NodeId, field: &StyleField) -> Result<Vec<Value>, String> {
        Ok(self
            .keys(None)?
            .into_iter()
            .filter_map(|key| {
                let type_name = self.compatible(node, &key, field).ok()?;
                Some(json!({"key":key,"type_name":type_name}))
            })
            .collect())
    }

    pub(crate) fn compatible(
        &self,
        node: NodeId,
        key: &str,
        field: &StyleField,
    ) -> Result<String, String> {
        let sources = IncludeLookup(self.includes);
        let lookup =
            scoped_resource_resolution(self.document, self.address, node, key, None, &sources)
                .map_err(|error| error.to_string())?;
        if lookup.external || lookup.may_be_missing || lookup.candidates.is_empty() {
            return Err("Choose a resource available in this style's theme scopes".into());
        }
        let mut types = BTreeSet::new();
        for location in lookup.candidates {
            let resource = self
                .at(location.address)
                .and_then(|document| document.ast().node(location.node))
                .ok_or("The authored resource source is unavailable")?;
            if !field.accepts_resource(self.schema, resource) {
                return Err("Choose a resource compatible with the setter in each theme".into());
            }
            types.insert(resource.local_name());
        }
        Ok(types.into_iter().collect::<Vec<_>>().join(" / "))
    }

    fn at(&self, address: &str) -> Option<&Document> {
        if address == self.address {
            return Some(self.document);
        }
        self.includes
            .authored_source(address)
            .map(|source| &source.document)
    }

    fn keys(&self, kind: Option<ComponentKind>) -> Result<BTreeSet<String>, String> {
        let mut keys = BTreeSet::new();
        for document in std::iter::once(self.document)
            .chain(self.includes.documents().map(|source| &source.document))
        {
            let entries =
                ComponentCatalog::declarations(document).map_err(|error| error.to_string())?;
            keys.extend(
                entries
                    .into_iter()
                    .filter(|entry| kind.is_none_or(|kind| entry.kind == kind))
                    .filter_map(|entry| entry.key),
            );
        }
        Ok(keys)
    }
}
