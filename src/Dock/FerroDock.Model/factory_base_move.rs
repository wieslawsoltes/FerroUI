// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use super::collection::{count_i32, index_i32, list_index};
use super::FactoryBase;
use crate::collections::ListHandle;
use crate::controls::IRootDock;
use crate::core::*;
use std::rc::Rc;

impl FactoryBase {
    /// Reorders one visible item within a dock, selects it and removes orphaned splitters.
    pub fn move_dockable(
        &self,
        dock: Option<Rc<dyn IDock>>,
        source: Option<Rc<dyn IDockable>>,
        target: Option<Rc<dyn IDockable>>,
    ) {
        let dock = dock.expect("source move_dockable dock.VisibleDockables access");
        if dock.visible_dockables().is_none() {
            return;
        }
        let source_index = index_i32(
            &dock
                .visible_dockables()
                .expect("source visible list exists"),
            &source,
        );
        let target_index = index_i32(
            &dock
                .visible_dockables()
                .expect("source visible list exists"),
            &target,
        );
        if source_index >= 0 && target_index >= 0 && source_index != target_index {
            self.remove_visible_dockable_at(Some(dock.clone()), source_index);
            self.runtime().on_dockable_removed(source.clone());
            self.runtime()
                .on_dockable_undocked(source.clone(), DockOperation::Fill);
            self.insert_visible_dockable(Some(dock.clone()), target_index, source.clone());
            self.runtime().on_dockable_added(source.clone());
            self.runtime().on_dockable_moved(source.clone());
            self.runtime()
                .on_dockable_docked(source.clone(), DockOperation::Fill);
            dock.set_active_dockable(source);
            self.cleanup_orphaned_splitters(Some(dock));
        }
    }

    /// Reorders two items only when both belong to the same pinned strip.
    pub fn move_pinned_dockable(
        &self,
        source: Option<Rc<dyn IDockable>>,
        target: Option<Rc<dyn IDockable>>,
    ) {
        let Some(root) = self
            .runtime()
            .find_root(source.clone(), Some(Rc::new(|_| true)))
        else {
            return;
        };
        let Some(items) = self
            .get_pinned_collection(source.clone(), root)
            .filter(|items| items.contains(&target))
        else {
            return;
        };
        let source_index = index_i32(&items, &source);
        let target_index = index_i32(&items, &target);
        if source_index >= 0 && target_index >= 0 && source_index != target_index {
            items.remove_at(list_index(source_index));
            items.insert(list_index(target_index), source.clone());
            self.runtime().on_dockable_moved(source);
        }
    }

    pub(super) fn get_pinned_collection(
        &self,
        dockable: Option<Rc<dyn IDockable>>,
        root: Rc<dyn IRootDock>,
    ) -> Option<ListHandle<Option<Rc<dyn IDockable>>>> {
        for get in [
            IRootDock::left_pinned_dockables,
            IRootDock::right_pinned_dockables,
            IRootDock::top_pinned_dockables,
            IRootDock::bottom_pinned_dockables,
        ] {
            if get(root.as_ref()).is_some_and(|items| items.contains(&dockable)) {
                return get(root.as_ref());
            }
        }
        None
    }

    /// Moves through the source overload, preserving insertion/removal event order and deferred floating-root collapse.
    pub fn move_dockable_between_docks(
        &self,
        source_dock: Option<Rc<dyn IDock>>,
        target_dock: Option<Rc<dyn IDock>>,
        source: Option<Rc<dyn IDockable>>,
        target: Option<Rc<dyn IDockable>>,
    ) {
        self.unpin_dockable(source.clone());
        let target_dock = target_dock.expect("source move target.VisibleDockables access");
        if target_dock.visible_dockables().is_none() {
            target_dock.set_visible_dockables(Some((self.create_dockable_list)(Vec::new())));
            if target_dock.visible_dockables().is_none() {
                return;
            }
        }
        let source_dock = source_dock.expect("source move source.VisibleDockables access");
        let same_owner = Rc::ptr_eq(&source_dock, &target_dock);
        let mut target_index = 0;
        if source_dock.visible_dockables().is_some()
            && target_dock.visible_dockables().is_some()
            && target_dock
                .visible_dockables()
                .expect("source target list exists")
                .count()
                > 0
        {
            if same_owner {
                let source_index = index_i32(
                    &source_dock
                        .visible_dockables()
                        .expect("source visible list exists"),
                    &source,
                );
                target_index = target.as_ref().map_or_else(
                    || {
                        count_i32(
                            target_dock
                                .visible_dockables()
                                .expect("source target list exists")
                                .count(),
                        ) - 1
                    },
                    |target| {
                        index_i32(
                            &target_dock
                                .visible_dockables()
                                .expect("source target list exists"),
                            &Some(target.clone()),
                        )
                    },
                );
                if source_index == target_index {
                    return;
                }
            } else if let Some(target) = target {
                target_index = index_i32(
                    &target_dock
                        .visible_dockables()
                        .expect("source target list exists"),
                    &Some(target),
                );
                target_index = if target_index >= 0 {
                    target_index + 1
                } else {
                    count_i32(
                        target_dock
                            .visible_dockables()
                            .expect("source target list exists")
                            .count(),
                    ) - 1
                };
            } else {
                target_index = count_i32(
                    target_dock
                        .visible_dockables()
                        .expect("source target list exists")
                        .count(),
                ) - 1;
            }
        }
        if source_dock.visible_dockables().is_none() || target_dock.visible_dockables().is_none() {
            return;
        }
        if same_owner {
            let source_index = index_i32(
                &source_dock
                    .visible_dockables()
                    .expect("source visible list exists"),
                &source,
            );
            if source_index < target_index {
                self.insert_visible_dockable(
                    Some(target_dock.clone()),
                    target_index + 1,
                    source.clone(),
                );
                self.runtime().on_dockable_added(source.clone());
                self.remove_visible_dockable_at(Some(target_dock.clone()), source_index);
                self.runtime().on_dockable_removed(source.clone());
                self.runtime()
                    .on_dockable_undocked(source.clone(), DockOperation::Fill);
                self.runtime().on_dockable_moved(source.clone());
                self.runtime()
                    .on_dockable_docked(source, DockOperation::Fill);
            } else {
                let remove_index = source_index + 1;
                if count_i32(
                    target_dock
                        .visible_dockables()
                        .expect("source target list exists")
                        .count(),
                ) + 1
                    > remove_index
                {
                    self.insert_visible_dockable(
                        Some(target_dock.clone()),
                        target_index,
                        source.clone(),
                    );
                    self.runtime().on_dockable_added(source.clone());
                    self.remove_visible_dockable_at(Some(target_dock.clone()), remove_index);
                    self.runtime().on_dockable_removed(source.clone());
                    self.runtime()
                        .on_dockable_undocked(source.clone(), DockOperation::Fill);
                    self.runtime().on_dockable_moved(source.clone());
                    self.runtime()
                        .on_dockable_docked(source, DockOperation::Fill);
                }
            }
            self.cleanup_orphaned_splitters(Some(target_dock));
        } else {
            let defer_collapse = (source_dock.clone() as Rc<dyn IDockable>)
                .as_root_dock()
                .is_some_and(|root| root.window().is_some());
            self.runtime()
                .remove_dockable(source.clone(), !defer_collapse);
            self.runtime()
                .on_dockable_undocked(source.clone(), DockOperation::Fill);
            self.insert_visible_dockable(Some(target_dock.clone()), target_index, source.clone());
            self.runtime()
                .init_dockable(source.clone(), Some(target_dock.clone()));
            target_dock.set_active_dockable(source.clone());
            self.runtime().on_dockable_added(source.clone());
            self.runtime().on_dockable_moved(source.clone());
            self.runtime()
                .on_dockable_docked(source, DockOperation::Fill);
            self.cleanup_orphaned_splitters(Some(target_dock));
            if defer_collapse {
                self.runtime().collapse_dock(source_dock);
            }
        }
    }

    /// Swaps visible entries within a dock using the original asymmetric added/removed events.
    pub fn swap_dockable(
        &self,
        dock: Option<Rc<dyn IDock>>,
        source: Option<Rc<dyn IDockable>>,
        target: Option<Rc<dyn IDockable>>,
    ) {
        let dock = dock.expect("source swap_dockable dock.VisibleDockables access");
        if dock.visible_dockables().is_none() {
            return;
        }
        let source_index = index_i32(
            &dock
                .visible_dockables()
                .expect("source visible list exists"),
            &source,
        );
        let target_index = index_i32(
            &dock
                .visible_dockables()
                .expect("source visible list exists"),
            &target,
        );
        if source_index >= 0 && target_index >= 0 && source_index != target_index {
            let original_source = dock
                .visible_dockables()
                .expect("source visible list exists")
                .get(list_index(source_index));
            let original_target = dock
                .visible_dockables()
                .expect("source visible list exists")
                .get(list_index(target_index));
            dock.visible_dockables()
                .expect("source visible list exists")
                .set(list_index(target_index), original_source.clone());
            self.runtime().on_dockable_removed(original_target.clone());
            self.runtime().on_dockable_added(original_source.clone());
            dock.visible_dockables()
                .expect("source visible list exists")
                .set(list_index(source_index), original_target.clone());
            self.runtime().on_dockable_added(original_target.clone());
            self.runtime().on_dockable_swapped(original_source);
            self.runtime().on_dockable_swapped(original_target.clone());
            dock.set_active_dockable(original_target);
            self.cleanup_orphaned_splitters(Some(dock));
        }
    }

    /// Swaps entries between owners, reinitializes both and updates selection before cleanup.
    pub fn swap_dockable_between_docks(
        &self,
        source_dock: Option<Rc<dyn IDock>>,
        target_dock: Option<Rc<dyn IDock>>,
        source: Option<Rc<dyn IDockable>>,
        target: Option<Rc<dyn IDockable>>,
    ) {
        let source_dock = source_dock.expect("source swap source.VisibleDockables access");
        if source_dock.visible_dockables().is_none() {
            return;
        }
        let target_dock = target_dock.expect("source swap target.VisibleDockables access");
        if target_dock.visible_dockables().is_none() {
            return;
        }
        let source_index = index_i32(
            &source_dock
                .visible_dockables()
                .expect("source visible list exists"),
            &source,
        );
        let target_index = index_i32(
            &target_dock
                .visible_dockables()
                .expect("source target list exists"),
            &target,
        );
        if source_index >= 0 && target_index >= 0 {
            let original_source = source_dock
                .visible_dockables()
                .expect("source visible list exists")
                .get(list_index(source_index));
            let original_target = target_dock
                .visible_dockables()
                .expect("source target list exists")
                .get(list_index(target_index));
            source_dock
                .visible_dockables()
                .expect("source visible list exists")
                .set(list_index(source_index), original_target.clone());
            target_dock
                .visible_dockables()
                .expect("source target list exists")
                .set(list_index(target_index), original_source.clone());
            self.runtime()
                .init_dockable(original_source.clone(), Some(target_dock.clone()));
            self.runtime()
                .init_dockable(original_target.clone(), Some(source_dock.clone()));
            self.runtime().on_dockable_swapped(original_target.clone());
            self.runtime().on_dockable_swapped(original_source.clone());
            source_dock.set_active_dockable(original_target);
            target_dock.set_active_dockable(original_source);
            self.cleanup_orphaned_splitters(Some(source_dock));
            self.cleanup_orphaned_splitters(Some(target_dock));
        }
    }
}
