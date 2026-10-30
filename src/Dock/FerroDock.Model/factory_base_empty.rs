// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use super::FactoryBase;
use crate::core::*;
use std::rc::Rc;

pub(super) fn is_dockable_empty(item: Option<Rc<dyn IDockable>>) -> bool {
    item.is_none_or(|item| {
        item.clone().as_splitter().is_some()
            || item
                .as_dock()
                .is_some_and(|dock| dock.is_empty() && dock.is_collapsable())
    })
}

impl FactoryBase {
    pub(super) fn update_is_empty(&self, dock: Rc<dyn IDock>) {
        let old = dock.is_empty();
        let visible = dock.visible_dockables();
        let new = if let Some(split) = (dock.clone() as Rc<dyn IDockable>).as_split_view_dock() {
            let split_visible = split.visible_dockables();
            !(split_visible.is_some_and(|items| {
                items.count() > 0 && items.iter().any(|item| !is_dockable_empty(item))
            })) && is_dockable_empty(split.pane_dockable())
                && is_dockable_empty(split.content_dockable())
        } else {
            visible.is_none_or(|items| items.count() == 0 || items.iter().all(is_dockable_empty))
        };
        if old != new {
            dock.set_is_empty(new);
            if let Some(parent) = dock.owner().and_then(|owner| owner.as_dock()) {
                self.update_is_empty(parent);
            }
        }
        Self::update_opened_dockables_count(dock);
    }

    fn update_opened_dockables_count(item: Rc<dyn IDockable>) {
        if item.clone().as_proportional_dock().is_some() || item.clone().as_root_dock().is_some() {
            let dock = item
                .clone()
                .as_dock()
                .expect("source container implements IDock");
            let count = dock.visible_dockables().map_or(0, |items| {
                items
                    .iter()
                    .try_fold(0_i32, |total, item| {
                        total.checked_add(
                            item.and_then(|item| item.as_dock())
                                .map_or(0, |dock| dock.opened_dockables_count()),
                        )
                    })
                    .expect("source Sum<Int32> overflow")
            });
            dock.set_opened_dockables_count(count);
        } else if let Some(dock) = item.clone().as_dock() {
            dock.set_opened_dockables_count(1);
        }
        if item.owner().is_some() {
            Self::update_opened_dockables_count(item.owner().expect("source owner exists"));
        }
    }
}
