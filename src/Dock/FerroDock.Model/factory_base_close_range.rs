// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use super::collection::{count_i32, index_i32, list_index};
use super::FactoryBase;
use crate::core::*;
use std::rc::Rc;

impl FactoryBase {
    fn close_dockables_range(
        &self,
        dock: Rc<dyn IDock>,
        start: i32,
        end: i32,
        excluding: Option<Rc<dyn IDockable>>,
    ) {
        if dock.visible_dockables().is_none() {
            return;
        }
        for i in (start..=end).rev() {
            if excluding.as_ref().is_none_or(|excluding| {
                !super::tracking::same(
                    &dock
                        .visible_dockables()
                        .expect("source visible list exists")
                        .get(list_index(i)),
                    &Some(excluding.clone()),
                )
            }) {
                self.runtime().close_dockable(
                    dock.visible_dockables()
                        .expect("source visible list exists")
                        .get(list_index(i)),
                );
            }
        }
    }

    /// Requests closure of all visible siblings other than the specified item, in reverse order.
    pub fn close_other_dockables(&self, dockable: Rc<dyn IDockable>) {
        let Some(dock) = dockable.owner().and_then(|owner| owner.as_dock()) else {
            return;
        };
        if dock.visible_dockables().is_none() {
            return;
        }
        let end = count_i32(
            dock.visible_dockables()
                .expect("source visible list exists")
                .count(),
        ) - 1;
        self.close_dockables_range(dock, 0, end, Some(dockable));
    }

    /// Requests closure of all visible siblings in reverse order.
    pub fn close_all_dockables(&self, dockable: Rc<dyn IDockable>) {
        let Some(dock) = dockable.owner().and_then(|owner| owner.as_dock()) else {
            return;
        };
        if dock.visible_dockables().is_none() {
            return;
        }
        let end = count_i32(
            dock.visible_dockables()
                .expect("source visible list exists")
                .count(),
        ) - 1;
        self.close_dockables_range(dock, 0, end, None);
    }

    /// Requests closure of visible siblings before the specified item's current index.
    pub fn close_left_dockables(&self, dockable: Rc<dyn IDockable>) {
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
        if index == -1 {
            return;
        }
        self.close_dockables_range(dock, 0, index - 1, None);
    }

    /// Requests closure of visible siblings after the specified item's current index.
    pub fn close_right_dockables(&self, dockable: Rc<dyn IDockable>) {
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
        if index == -1 {
            return;
        }
        let end = count_i32(
            dock.visible_dockables()
                .expect("source visible list exists")
                .count(),
        ) - 1;
        self.close_dockables_range(dock, index + 1, end, None);
    }
}
