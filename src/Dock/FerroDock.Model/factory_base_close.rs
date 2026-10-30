// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use super::FactoryBase;
use crate::core::*;
use crate::DockCapabilityResolver;
use std::rc::Rc;

impl FactoryBase {
    /// Evaluates last-child and capability policy before the overridable close core.
    pub fn close_dockable(&self, dockable: Option<Rc<dyn IDockable>>) {
        let Some(dockable) = dockable else { return };
        if let Some(owner) = dockable.owner().and_then(|owner| owner.as_dock()) {
            if !owner.can_close_last_dockable()
                && owner
                    .visible_dockables()
                    .is_some_and(|items| items.count() <= 1)
            {
                return;
            }
        }
        if !DockCapabilityResolver::is_enabled(
            Some(dockable.clone()),
            DockCapability::Close,
            DockCapabilityResolver::resolve_operation_dock(Some(dockable.clone())),
        ) {
            return;
        }
        self.hooks().close_dockable_core(dockable);
    }

    /// Closes without capability/last-child eligibility checks, retaining cancellation and source lifecycle.
    pub fn close_dockable_core(&self, dockable: Rc<dyn IDockable>) {
        if self.runtime().on_dockable_closing(Some(dockable.clone())) {
            let generated = self
                .hooks()
                .handle_items_source_dockable_closing(dockable.clone());
            let hide = !generated
                && ((dockable.clone().as_tool().is_some() && self.runtime().hide_tools_on_close())
                    || (dockable.clone().as_document().is_some()
                        && self.runtime().hide_documents_on_close()));
            if hide {
                self.runtime().hide_dockable(Some(dockable.clone()));
            } else {
                self.runtime().remove_dockable(Some(dockable.clone()), true);
            }
            self.runtime().on_dockable_closed(Some(dockable));
        }
    }

    /// Removes a generated document/tool's current Context from its original source when available.
    /// Source recognition suppresses hide-on-close even when Context is null or removal returns false.
    pub fn handle_items_source_dockable_closing(&self, dockable: Rc<dyn IDockable>) -> bool {
        if let Some(document) = dockable.clone().as_document() {
            if let Some(source) = self.resolve_document_items_source_dock(dockable.clone()) {
                if let Some(item) = document.context() {
                    source.remove_item_from_source(Some(item));
                }
                return true;
            }
        }
        if let Some(tool) = dockable.clone().as_tool() {
            if let Some(source) = self.resolve_tool_items_source_dock(dockable) {
                if let Some(item) = tool.context() {
                    source.remove_item_from_source(Some(item));
                }
                return true;
            }
        }
        false
    }

    fn resolve_document_items_source_dock(
        &self,
        dockable: Rc<dyn IDockable>,
    ) -> Option<Rc<dyn IItemsSourceDock>> {
        if let Some(owner) = dockable
            .owner()
            .and_then(|owner| owner.as_items_source_dock())
        {
            if owner.is_document_from_items_source(dockable.clone()) {
                return Some(owner);
            }
        }
        if let Some(owner) = self
            .hooks()
            .get_tracked_items_source_owner(dockable.clone())
            .and_then(|owner| owner.as_items_source_dock())
        {
            if owner.is_document_from_items_source(dockable.clone()) {
                return Some(owner);
            }
        }
        let root = self
            .runtime()
            .find_root(Some(dockable.clone()), Some(Rc::new(|_| true)))?;
        for item in self.find_in_dock(Some(root), Some(Rc::new(|_| true))) {
            if let Some(source) = item.and_then(|item| item.as_items_source_dock()) {
                if source.is_document_from_items_source(dockable.clone()) {
                    return Some(source);
                }
            }
        }
        None
    }

    fn resolve_tool_items_source_dock(
        &self,
        dockable: Rc<dyn IDockable>,
    ) -> Option<Rc<dyn IToolItemsSourceDock>> {
        if let Some(owner) = dockable
            .owner()
            .and_then(|owner| owner.as_tool_items_source_dock())
        {
            if owner.is_tool_from_items_source(dockable.clone()) {
                return Some(owner);
            }
        }
        if let Some(owner) = self
            .hooks()
            .get_tracked_items_source_owner(dockable.clone())
            .and_then(|owner| owner.as_tool_items_source_dock())
        {
            if owner.is_tool_from_items_source(dockable.clone()) {
                return Some(owner);
            }
        }
        let root = self
            .runtime()
            .find_root(Some(dockable.clone()), Some(Rc::new(|_| true)))?;
        for item in self.find_in_dock(Some(root), Some(Rc::new(|_| true))) {
            if let Some(source) = item.and_then(|item| item.as_tool_items_source_dock()) {
                if source.is_tool_from_items_source(dockable.clone()) {
                    return Some(source);
                }
            }
        }
        None
    }
}
