// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use super::FactoryBase;
use crate::collections::ListHandle;
use crate::core::*;
use std::rc::Rc;

pub(super) fn list_index(index: i32) -> usize {
    usize::try_from(index).expect("source index must be nonnegative")
}
pub(super) fn count_i32(count: usize) -> i32 {
    i32::try_from(count).expect("source IList Count fits Int32")
}
pub(super) fn index_i32(
    items: &ListHandle<Option<Rc<dyn IDockable>>>,
    item: &Option<Rc<dyn IDockable>>,
) -> i32 {
    items.index_of(item).map_or(-1, count_i32)
}

impl FactoryBase {
    /// Initializes an item, appends it to the current visible list, then raises added.
    pub fn add_dockable(&self, dock: Option<Rc<dyn IDock>>, dockable: Option<Rc<dyn IDockable>>) {
        self.runtime().init_dockable(
            dockable.clone(),
            dock.clone().map(|dock| dock as Rc<dyn IDockable>),
        );
        let dock = dock.expect("source add dock.VisibleDockables access");
        if dock.visible_dockables().is_none() {
            dock.set_visible_dockables(Some((self.create_dockable_list)(Vec::new())));
        }
        self.add_visible_dockable(Some(dock), dockable.clone());
        self.runtime().on_dockable_added(dockable);
    }

    /// Initializes and inserts an item when the index is nonnegative, then raises added.
    pub fn insert_dockable(
        &self,
        dock: Option<Rc<dyn IDock>>,
        dockable: Option<Rc<dyn IDockable>>,
        index: i32,
    ) {
        if index < 0 {
            return;
        }
        self.runtime().init_dockable(
            dockable.clone(),
            dock.clone().map(|dock| dock as Rc<dyn IDockable>),
        );
        let dock = dock.expect("source insert dock.VisibleDockables access");
        if dock.visible_dockables().is_none() {
            dock.set_visible_dockables(Some((self.create_dockable_list)(Vec::new())));
        }
        self.insert_visible_dockable(Some(dock), index, dockable.clone());
        self.runtime().on_dockable_added(dockable);
    }

    /// Appends to the current visible list and updates empty state and opened counts.
    pub fn add_visible_dockable(
        &self,
        dock: Option<Rc<dyn IDock>>,
        dockable: Option<Rc<dyn IDockable>>,
    ) {
        let dock = dock.expect("source add_visible_dockable dock.VisibleDockables access");
        if dock.visible_dockables().is_none() {
            dock.set_visible_dockables(Some((self.create_dockable_list)(Vec::new())));
        }
        dock.visible_dockables()
            .expect("source visible list exists")
            .add(dockable);
        self.update_is_empty(dock);
    }

    /// Inserts into the current visible list and updates empty state and opened counts.
    pub fn insert_visible_dockable(
        &self,
        dock: Option<Rc<dyn IDock>>,
        index: i32,
        dockable: Option<Rc<dyn IDockable>>,
    ) {
        let dock = dock.expect("source insert_visible_dockable dock.VisibleDockables access");
        if dock.visible_dockables().is_none() {
            dock.set_visible_dockables(Some((self.create_dockable_list)(Vec::new())));
        }
        dock.visible_dockables()
            .expect("source visible list exists")
            .insert(list_index(index), dockable);
        self.update_is_empty(dock);
    }

    /// Removes a matching visible item, when a list exists, then updates container state.
    pub fn remove_visible_dockable(
        &self,
        dock: Option<Rc<dyn IDock>>,
        dockable: Option<Rc<dyn IDockable>>,
    ) {
        let dock = dock.expect("source remove_visible_dockable dock.VisibleDockables access");
        if dock.visible_dockables().is_some() {
            dock.visible_dockables()
                .expect("source visible list exists")
                .remove(&dockable);
        }
        self.update_is_empty(dock);
    }

    /// Clears a nonempty visible list and updates state even when no list exists.
    pub fn remove_all_visible_dockables(&self, dock: Option<Rc<dyn IDock>>) {
        let dock = dock.expect("source remove_all_visible_dockables dock.VisibleDockables access");
        if dock.visible_dockables().is_some()
            && dock
                .visible_dockables()
                .expect("source visible list exists")
                .count()
                > 0
        {
            dock.visible_dockables()
                .expect("source visible list exists")
                .clear();
        }
        self.update_is_empty(dock);
    }

    /// Removes the current visible item at an index, then updates container state.
    pub fn remove_visible_dockable_at(&self, dock: Option<Rc<dyn IDock>>, index: i32) {
        let dock = dock.expect("source remove_visible_dockable_at dock.VisibleDockables access");
        if dock.visible_dockables().is_some() {
            dock.visible_dockables()
                .expect("source visible list exists")
                .remove_at(list_index(index));
        }
        self.update_is_empty(dock);
    }

    /// Chooses the preceding child (or first child), leaving a splitter replacement unselected.
    pub fn get_active_dockable_after_removal(
        &self,
        dock: Option<Rc<dyn IDock>>,
        _removed: Option<Rc<dyn IDockable>>,
        removed_index: i32,
    ) -> Option<Rc<dyn IDockable>> {
        let dock = dock.expect("source selection dock.VisibleDockables access");
        let children = dock.visible_dockables().filter(|items| items.count() > 0)?;
        let next = children.get(list_index(if removed_index > 0 {
            removed_index - 1
        } else {
            0
        }));
        next.filter(|item| item.clone().as_splitter().is_none())
    }

    /// Removes boundary, isolated and consecutive splitters using the source snapshot/event order.
    pub fn cleanup_orphaned_splitters(&self, dock: Option<Rc<dyn IDock>>) {
        let dock = dock.expect("source cleanup_orphaned_splitters dock.VisibleDockables access");
        if dock.visible_dockables().is_none()
            || dock
                .visible_dockables()
                .expect("source visible list exists")
                .count()
                == 0
        {
            return;
        }
        let mut needs_cleanup = true;
        while needs_cleanup {
            needs_cleanup = false;
            let mut to_remove = Vec::new();
            let mut dockables = dock
                .visible_dockables()
                .expect("source visible list exists")
                .to_vec();
            if !dockables.is_empty()
                && dockables
                    .iter()
                    .all(|item| item.clone().and_then(|item| item.as_splitter()).is_some())
            {
                to_remove.extend(dockables);
            } else {
                while dockables
                    .first()
                    .is_some_and(|item| item.clone().and_then(|item| item.as_splitter()).is_some())
                {
                    to_remove.push(dockables.remove(0));
                }
                while dockables
                    .last()
                    .is_some_and(|item| item.clone().and_then(|item| item.as_splitter()).is_some())
                {
                    to_remove.push(dockables.pop().expect("last item exists"));
                }
                for i in 0..dockables.len().saturating_sub(1) {
                    if dockables[i]
                        .clone()
                        .and_then(|item| item.as_splitter())
                        .is_some()
                    {
                        let mut j = i + 1;
                        while j < dockables.len()
                            && dockables[j]
                                .clone()
                                .and_then(|item| item.as_splitter())
                                .is_some()
                        {
                            to_remove.push(dockables[j].clone());
                            j += 1;
                        }
                    }
                }
            }
            for splitter in to_remove {
                if dock
                    .visible_dockables()
                    .expect("source visible list exists")
                    .contains(&splitter)
                {
                    self.remove_visible_dockable(
                        Some(dock.clone()),
                        Some(splitter.clone().expect("source confirmed splitter")),
                    );
                    self.runtime().on_dockable_removed(splitter);
                    needs_cleanup = true;
                }
            }
        }
        if dock
            .active_dockable()
            .is_some_and(|item| item.as_splitter().is_some())
            || (dock.active_dockable().is_some()
                && !dock
                    .visible_dockables()
                    .expect("source visible list exists")
                    .contains(&Some(
                        (dock.active_dockable().expect("source active item exists")).clone(),
                    )))
        {
            let first = dock.visible_dockables().and_then(|items| {
                items
                    .iter()
                    .find(|item| item.clone().and_then(|item| item.as_splitter()).is_none())
            });
            dock.set_active_dockable(first.flatten());
        }
    }
}
