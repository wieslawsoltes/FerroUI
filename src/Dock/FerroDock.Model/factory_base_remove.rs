// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use super::collection::index_i32;
use super::tracking::same;
use super::FactoryBase;
use crate::core::*;
use std::rc::Rc;

impl FactoryBase {
    /// Removes a hidden or visible item, cleans pin state, reconciles selection/focus and optionally collapses its dock.
    pub fn remove_dockable(&self, dockable: Option<Rc<dyn IDockable>>, collapse: bool) {
        if self.is_dockable_pinned(dockable.clone(), None) {
            self.hooks().pin_dockable_core(dockable.clone());
        }
        let dockable = dockable.expect("source removal dockable.Owner access");
        if let Some(hidden_root) = dockable.owner().and_then(|owner| owner.as_root_dock()) {
            if hidden_root
                .hidden_dockables()
                .is_some_and(|items| items.remove(&Some((dockable).clone())))
            {
                self.runtime().on_dockable_removed(Some(dockable.clone()));
                if same(&hidden_root.focused_dockable(), &Some(dockable)) {
                    self.runtime().set_focused_dockable(
                        Some(hidden_root.clone()),
                        hidden_root.active_dockable(),
                    );
                }
                return;
            }
        }
        let Some(dock) = dockable.owner().and_then(|owner| owner.as_dock()) else {
            return;
        };
        if dock.visible_dockables().is_none() {
            return;
        }
        let index = index_i32(
            &dock
                .visible_dockables()
                .expect("source visible list exists"),
            &Some(dockable.clone()),
        );
        if index < 0 {
            if self
                .runtime()
                .find_root(
                    Some(dockable.clone()),
                    Some(Rc::new(|root| root.is_focusable_root())),
                )
                .is_some_and(|root| same(&root.focused_dockable(), &Some(dockable)))
            {
                self.runtime()
                    .set_focused_dockable(Some(dock.clone()), dock.active_dockable());
            }
            return;
        }
        let was_active = same(&dock.active_dockable(), &Some(dockable.clone()));
        self.remove_visible_dockable(Some(dock.clone()), Some(dockable.clone()));
        self.runtime().on_dockable_removed(Some(dockable.clone()));
        if was_active {
            dock.set_active_dockable(self.hooks().get_active_dockable_after_removal(
                Some(dock.clone()),
                Some(dockable.clone()),
                index,
            ));
        }
        if self
            .runtime()
            .find_root(
                Some(dockable.clone()),
                Some(Rc::new(|root| root.is_focusable_root())),
            )
            .is_some_and(|root| same(&root.focused_dockable(), &Some(dockable)))
        {
            self.runtime()
                .set_focused_dockable(Some(dock.clone()), dock.active_dockable());
        }
        self.cleanup_orphaned_splitters(Some(dock.clone()));
        if collapse {
            self.runtime().collapse_dock(dock);
        }
    }
}
