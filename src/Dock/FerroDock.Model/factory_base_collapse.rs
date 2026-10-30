// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use super::collection::{index_i32, list_index};
use super::empty::is_dockable_empty;
use super::tracking::same;
use super::FactoryBase;
use crate::collections::ListHandle;
use crate::controls::*;
use crate::core::*;
use std::rc::Rc;

impl FactoryBase {
    fn is_dock_pinned(
        items: Option<ListHandle<Option<Rc<dyn IDockable>>>>,
        dock: Rc<dyn IDock>,
    ) -> bool {
        if let Some(items) = items.filter(|items| items.count() != 0) {
            for item in items.iter() {
                if same(
                    &item
                        .as_ref()
                        .expect("source item.Owner dereference")
                        .owner(),
                    &Some(dock.clone() as Rc<dyn IDockable>),
                ) {
                    return true;
                }
            }
            return true; // Source retains aligned empty docks whenever the strip is nonempty.
        }
        false
    }

    fn cleanup_proportional_dock_tree(&self, dock: Rc<dyn IProportionalDock>) {
        if dock.visible_dockables().is_none() {
            return;
        }
        let Some(owner) = dock.owner().and_then(|owner| owner.as_dock()) else {
            return;
        };
        if dock
            .owner()
            .is_some_and(|owner| owner.as_root_dock().is_some())
            || !dock.is_collapsable()
        {
            return;
        }
        if dock
            .visible_dockables()
            .expect("source visible list exists")
            .count()
            == 1
        {
            let single = dock
                .visible_dockables()
                .expect("source visible list exists")
                .get(0);
            if owner.visible_dockables().is_some() {
                let index = index_i32(
                    &owner.visible_dockables().expect("source owner list exists"),
                    &Some(dock.clone() as Rc<dyn IDockable>),
                );
                if index >= 0 {
                    if let Some(single_dock) = single.clone().and_then(|item| item.as_dock()) {
                        if !dock.proportion().is_nan() {
                            single_dock.set_proportion(dock.proportion());
                            single_dock.set_collapsed_proportion(dock.proportion());
                        }
                    }
                    owner
                        .visible_dockables()
                        .expect("source owner list exists")
                        .remove_at(list_index(index));
                    owner
                        .visible_dockables()
                        .expect("source owner list exists")
                        .insert(list_index(index), single.clone());
                    single
                        .as_ref()
                        .expect("source singleDockable.Owner dereference")
                        .set_owner(Some(owner.clone()));
                    self.runtime().init_dockable(single, Some(owner.clone()));
                    if let Some(proportional) = (owner as Rc<dyn IDockable>).as_proportional_dock()
                    {
                        self.cleanup_proportional_dock_tree(proportional);
                    }
                }
            }
        }
    }

    /// Collapses an attached empty container while preserving pinned strips, window cancellation and proportional hierarchy rules.
    pub fn collapse_dock(&self, dock: Rc<dyn IDock>) {
        if !dock.is_collapsable() {
            return;
        }
        if let Some(split) = (dock.clone() as Rc<dyn IDockable>).as_split_view_dock() {
            if dock.visible_dockables().is_some()
                && dock
                    .visible_dockables()
                    .expect("source visible list exists")
                    .count()
                    != 0
            {
                return;
            }
            if !(is_dockable_empty(split.pane_dockable())
                && is_dockable_empty(split.content_dockable()))
            {
                return;
            }
        } else {
            if dock.visible_dockables().is_none() {
                return;
            }
            if dock
                .visible_dockables()
                .expect("source visible list exists")
                .count()
                != 0
            {
                return;
            }
        }
        let root = self
            .runtime()
            .find_root(Some(dock.clone()), Some(Rc::new(|_| true)));
        if let (Some(root), Some(tool)) = (root, (dock.clone() as Rc<dyn IDockable>).as_tool_dock())
        {
            let pinned = match tool.alignment() {
                Alignment::Left => root.left_pinned_dockables(),
                Alignment::Right => root.right_pinned_dockables(),
                Alignment::Top => root.top_pinned_dockables(),
                Alignment::Bottom => root.bottom_pinned_dockables(),
                Alignment::Unset => None,
            };
            if Self::is_dock_pinned(pinned, dock.clone()) {
                return;
            }
        }
        let owner = dock.owner().and_then(|owner| owner.as_dock());
        let proportional = dock.owner().and_then(|owner| owner.as_proportional_dock());
        let attached = owner
            .as_ref()
            .and_then(|owner| owner.visible_dockables())
            .is_some_and(|items| {
                items.contains(&Some((dock.clone() as Rc<dyn IDockable>).clone()))
            });
        if attached {
            if let Some(owner) = &owner {
                if owner.visible_dockables().is_some() {
                    let items = owner.visible_dockables().expect("source owner list exists");
                    let index = index_i32(&items, &Some(dock.clone() as Rc<dyn IDockable>));
                    if index >= 0 {
                        let previous = if index > 0 {
                            items
                                .get(list_index(index - 1))
                                .and_then(|item| item.as_proportional_dock_splitter())
                        } else {
                            None
                        };
                        let next = if list_index(index + 1) < items.count() {
                            items
                                .get(list_index(index + 1))
                                .and_then(|item| item.as_proportional_dock_splitter())
                        } else {
                            None
                        };
                        if let Some(splitter) = previous.or(next) {
                            self.runtime().remove_dockable(Some(splitter), true);
                        }
                    }
                }
            }
        }
        if let Some(root) = (dock.clone() as Rc<dyn IDockable>).as_root_dock() {
            if root.window().is_some() {
                self.runtime().remove_window(root.window());
                return;
            }
        }
        if attached {
            self.runtime().remove_dockable(Some(dock), true);
        }
        if let Some(owner) = proportional {
            if let Some(items) = owner.visible_dockables() {
                let mut count = 0;
                for item in items.iter() {
                    if item
                        .and_then(|item| item.as_proportional_dock_splitter())
                        .is_none()
                    {
                        count += 1;
                        if count > 1 {
                            break;
                        }
                    }
                }
                if count <= 1 {
                    self.cleanup_proportional_dock_tree(owner.clone());
                    self.cleanup_orphaned_splitters(Some(owner));
                }
            }
        }
    }
}
