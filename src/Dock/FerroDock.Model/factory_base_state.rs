// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use super::FactoryBase;
use crate::controls::*;
use crate::core::*;
use std::rc::Rc;

impl FactoryBase {
    /// Resolves the current state and updates the dockable followed by its mixin flags.
    pub fn update_docking_window_state(&self, dockable: Option<Rc<dyn IDockable>>) {
        let resolved = self.hooks().resolve_docking_window_state(dockable.clone());
        {
            let _scope = self.enter_state_sync_scope();
            dockable
                .as_ref()
                .expect("source dockable.DockingState assignment")
                .set_docking_state(resolved);
        }
        self.set_docking_window_state_flags(
            dockable.expect("source state assignment succeeded"),
            resolved,
        );
    }

    /// Updates visible descendants and split-view extras in source order.
    pub fn update_docking_window_state_recursive(&self, dockable: Option<Rc<dyn IDockable>>) {
        self.hooks().update_docking_window_state(dockable.clone());
        let Some(dock) = dockable.clone().and_then(|item| item.as_dock()) else {
            return;
        };
        if dock.visible_dockables().is_some() {
            for child in dock
                .visible_dockables()
                .expect("source visible list exists")
                .iter()
            {
                self.hooks().update_docking_window_state_recursive(child);
            }
        }
        let Some(split) = dockable.and_then(|item| item.as_split_view_dock()) else {
            return;
        };
        let pane = split.pane_dockable();
        if let Some(pane) = &pane {
            if !dock
                .visible_dockables()
                .is_some_and(|items| items.contains(&Some((pane).clone())))
            {
                self.hooks()
                    .update_docking_window_state_recursive(Some(pane.clone()));
            }
        }
        if let Some(content) = split.content_dockable() {
            if !pane.as_ref().is_some_and(|pane| Rc::ptr_eq(pane, &content))
                && !dock
                    .visible_dockables()
                    .is_some_and(|items| items.contains(&Some((content).clone())))
            {
                self.hooks()
                    .update_docking_window_state_recursive(Some(content));
            }
        }
    }

    /// Resolves pinned/document/docked location plus floating and hidden modifiers.
    pub fn resolve_docking_window_state(
        &self,
        dockable: Option<Rc<dyn IDockable>>,
    ) -> DockingWindowState {
        let root = self
            .runtime()
            .find_root(dockable.clone(), Some(Rc::new(|_| true)));
        let mut state =
            if root.is_some_and(|root| self.is_dockable_pinned(dockable.clone(), Some(root))) {
                DockingWindowState::PINNED
            } else if in_document_area(dockable.clone()) {
                DockingWindowState::DOCUMENT
            } else {
                DockingWindowState::DOCKED
            };
        if in_floating_area(dockable.clone()) {
            state |= DockingWindowState::FLOATING;
        }
        if self.is_dockable_hidden(dockable) {
            state |= DockingWindowState::HIDDEN;
        }
        state
    }

    pub(super) fn is_dockable_hidden(&self, dockable: Option<Rc<dyn IDockable>>) -> bool {
        let Some(root) = self
            .runtime()
            .find_root(dockable.clone(), Some(Rc::new(|_| true)))
        else {
            return false;
        };
        if root.hidden_dockables().is_none() {
            return false;
        }
        if root
            .hidden_dockables()
            .expect("source hidden list exists")
            .contains(&dockable)
        {
            return true;
        }
        let mut current = dockable
            .expect("source hidden dockable.Owner access")
            .owner();
        while let Some(item) = current {
            if Rc::ptr_eq(&item, &(root.clone() as Rc<dyn IDockable>)) {
                break;
            }
            if root
                .hidden_dockables()
                .expect("source hidden list exists")
                .contains(&Some((item).clone()))
            {
                return true;
            }
            current = item.owner();
        }
        false
    }
}

fn in_document_area_core(dockable: Option<Rc<dyn IDockable>>) -> bool {
    let mut current = dockable;
    while let Some(item) = current {
        if item.clone().as_document_dock().is_some() {
            return true;
        }
        current = item.owner();
    }
    false
}

fn in_document_area(dockable: Option<Rc<dyn IDockable>>) -> bool {
    in_document_area_core(dockable.clone())
        || dockable
            .expect("source document-area dockable.OriginalOwner access")
            .original_owner()
            .is_some_and(|owner| in_document_area_core(Some(owner)))
}

fn find_owner_root(dockable: Option<Rc<dyn IDockable>>) -> Option<Rc<dyn IRootDock>> {
    let mut current = dockable;
    while let Some(item) = current {
        if let Some(root) = item.clone().as_root_dock() {
            return Some(root);
        }
        current = item.owner();
    }
    None
}

fn in_floating_area_core(dockable: Option<Rc<dyn IDockable>>) -> bool {
    find_owner_root(dockable)
        .and_then(|root| root.window())
        .and_then(|window| window.owner())
        .is_some_and(|owner| owner.as_root_dock().is_some())
}

fn in_floating_area(dockable: Option<Rc<dyn IDockable>>) -> bool {
    in_floating_area_core(dockable.clone())
        || dockable
            .expect("source floating-area dockable.OriginalOwner access")
            .original_owner()
            .is_some_and(|owner| in_floating_area_core(Some(owner)))
}
