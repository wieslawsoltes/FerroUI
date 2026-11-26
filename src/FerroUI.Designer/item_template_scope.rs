//! Retain an item template preview when an editor replaces its native frame.
use crate::{item_template_samples::ItemTemplatePreview, DefinitionScope, DesignerSession};
use ferroui_designer_core::{Document, NodeId};

pub struct ItemTemplateScopeSnapshot {
    document: Document,
    scope: DefinitionScope,
    selection: Vec<NodeId>,
}

impl DesignerSession {
    pub fn item_template_scope_snapshot(&self) -> Option<ItemTemplateScopeSnapshot> {
        let scope = self.definition_scope()?.clone();
        scope.item_template.as_ref()?;
        Some(ItemTemplateScopeSnapshot {
            document: self.document().snapshot_without_history(),
            scope,
            selection: self.selection().to_vec(),
        })
    }

    /// Reconcile the accepted source first, then translate to this freshly
    /// mounted session's IDs. A removed owner or reassigned template leaves the
    /// preview at the document instead of targeting a neighbor with an old ID.
    pub fn restore_item_template_scope(
        &mut self,
        accepted: &ItemTemplateScopeSnapshot,
    ) -> Result<bool, String> {
        let mut reconciled = accepted.document.clone();
        if !reconciled
            .update_source("Reconcile item template preview", self.document().source())
            .valid
        {
            return Ok(false);
        }
        let remap = |id| {
            let previous = reconciled.ast().node(id)?;
            let ast = self.document().ast();
            let current = ast.node(ast.element_at(previous.name_span.start)?)?;
            (current.name_span == previous.name_span && current.name == previous.name)
                .then_some(current.id)
        };
        let Some(mut item) = accepted.scope.item_template.clone() else {
            return Ok(false);
        };
        let Some(definition) = remap(accepted.scope.node) else {
            return Ok(false);
        };
        match &mut item {
            ItemTemplatePreview::Assigned { owner, property } => {
                let Some(current) = remap(*owner) else {
                    return Ok(false);
                };
                if !self.item_template_slot(current, *property).is_ok_and(|slot| self.local_template_definition(&slot) == Some(definition)) {
                    return Ok(false);
                }
                *owner = current;
            }
            ItemTemplatePreview::Included { key, property, .. } => {
                if !self.document().ast().node(definition).is_some_and(|node| {
                    property.accepts(node) && ferroui_designer_tools::keyed_template(node, *property) == *key
                }) {
                    return Ok(false);
                }
            }
        }
        let return_selection = accepted
            .scope
            .return_selection
            .iter()
            .copied()
            .filter_map(remap)
            .collect::<Vec<_>>();
        let selected = accepted
            .selection
            .iter()
            .copied()
            .filter_map(remap)
            .collect::<Vec<_>>();
        self.select(return_selection);
        self.enter_definition(definition, accepted.scope.options.clone(), Some(item))?;
        if !selected.is_empty() {
            self.select(selected);
        }
        Ok(true)
    }
}
