// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use super::FactoryBase;
use crate::core::*;
use std::rc::Rc;

impl FactoryBase {
    /// Hides an item in its root after capability-independent pin cleanup and visible removal.
    pub fn hide_dockable(&self, dockable: Option<Rc<dyn IDockable>>) {
        if self.is_dockable_pinned(dockable.clone(), None) {
            self.hooks().pin_dockable_core(dockable.clone());
        }
        let Some(root) = self
            .runtime()
            .find_root(dockable.clone(), Some(Rc::new(|_| true)))
        else {
            return;
        };
        if root.hidden_dockables().is_none() {
            root.set_hidden_dockables(Some((self.create_dockable_list)(Vec::new())));
        }
        let dockable = dockable.expect("source hide dockable.Owner access");
        if let Some(owner) = dockable.owner().and_then(|owner| owner.as_dock()) {
            if owner.visible_dockables().is_some() {
                self.remove_visible_dockable(Some(owner.clone()), Some(dockable.clone()));
                self.runtime().on_dockable_removed(Some(dockable.clone()));
                dockable.set_original_owner(Some(owner.clone()));
                self.cleanup_orphaned_splitters(Some(owner));
            }
        }
        dockable.set_owner(Some(root.clone()));
        root.hidden_dockables()
            .expect("source hide root.HiddenDockables.Add access")
            .add(Some(dockable.clone()));
        self.hooks()
            .update_docking_window_state_recursive(Some(dockable.clone()));
        self.runtime().on_dockable_hidden(Some(dockable));
    }

    /// Hides the first matching visible query result across registered dock controls.
    pub fn hide_dockable_by_id(&self, id: String) {
        let dockable = self
            .find(Some(Rc::new(move |item| {
                item.expect("source predicate item.Id dereference").id() == id
            })))
            .iter()
            .next();
        if let Some(Some(dockable)) = dockable {
            self.runtime().hide_dockable(Some(dockable));
        }
    }

    /// Restores a hidden item to its original dock or clears ownership when none is available.
    pub fn restore_dockable(&self, dockable: Option<Rc<dyn IDockable>>) {
        let Some(root) = self
            .runtime()
            .find_root(dockable.clone(), Some(Rc::new(|_| true)))
        else {
            return;
        };
        if root.hidden_dockables().is_none()
            || !root
                .hidden_dockables()
                .expect("source hidden list exists")
                .contains(&dockable)
        {
            return;
        }
        root.hidden_dockables()
            .expect("source hidden list exists")
            .remove(&dockable);
        self.runtime().on_dockable_restored(dockable.clone());
        let dockable = dockable.expect("source restore dockable.OriginalOwner access");
        if let Some(owner) = dockable.original_owner().and_then(|owner| owner.as_dock()) {
            self.add_visible_dockable(Some(owner.clone()), Some(dockable.clone()));
            self.runtime().on_dockable_added(Some(dockable.clone()));
            dockable.set_owner(Some(owner));
            dockable.set_original_owner(None);
        } else {
            dockable.set_owner(None);
        }
        self.hooks()
            .update_docking_window_state_recursive(Some(dockable));
    }

    /// Restores the first hidden matching ID in the visible registered-root query order.
    pub fn restore_dockable_by_id(&self, id: String) -> Option<Rc<dyn IDockable>> {
        for root in self
            .find(Some(Rc::new(|item| {
                item.and_then(|item| item.as_root_dock()).is_some()
            })))
            .iter()
            .filter_map(|item| item.and_then(|item| item.as_root_dock()))
        {
            if root.hidden_dockables().is_none() {
                continue;
            }
            let item = root
                .hidden_dockables()
                .expect("source hidden list exists")
                .iter()
                .find(|item| item.as_ref().expect("source item.Id dereference").id() == id);
            if let Some(Some(item)) = item {
                self.runtime().restore_dockable(Some(item.clone()));
                return Some(item);
            }
        }
        None
    }
}
