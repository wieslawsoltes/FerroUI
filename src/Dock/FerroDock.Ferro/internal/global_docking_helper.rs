// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::{
    controls::IRootDock,
    core::{IDock, Orientation},
};
use std::rc::Rc;
pub(crate) struct GlobalDockingHelper;
impl GlobalDockingHelper {
    pub(crate) fn can_global_dock(target: Rc<dyn IRootDock>) -> (bool, bool) {
        let mut horizontal = 0_i32;
        let mut vertical = 0_i32;
        Self::count_proportional_dock_children(target, &mut horizontal, &mut vertical);
        (vertical >= 2, horizontal >= 2)
    }
    fn count_proportional_dock_children(
        dock: Rc<dyn IDock>,
        horizontal: &mut i32,
        vertical: &mut i32,
    ) {
        if *horizontal >= 2 && *vertical >= 2 {
            return;
        }
        if let Some(proportional) = dock.clone().as_proportional_dock() {
            if proportional.visible_dockables().is_some() {
                let mut count = 0_i32;
                for item in proportional
                    .visible_dockables()
                    .expect("source visible collection")
                    .iter()
                {
                    let wrapper = item
                        .clone()
                        .and_then(|item| item.as_proportional_dock())
                        .and_then(|p| p.visible_dockables())
                        .is_some_and(|items| items.count() == 1);
                    if !wrapper && item.and_then(|item| item.as_splitter()).is_none() {
                        count = count.wrapping_add(1);
                    }
                }
                if proportional.orientation() == Orientation::Horizontal {
                    *horizontal = horizontal.wrapping_add(count);
                } else {
                    *vertical = vertical.wrapping_add(count);
                }
                if *horizontal >= 2 && *vertical >= 2 {
                    return;
                }
                for item in proportional
                    .visible_dockables()
                    .expect("source collection re-read")
                    .iter()
                {
                    if let Some(child) = item.and_then(|item| item.as_proportional_dock()) {
                        Self::count_proportional_dock_children(child, horizontal, vertical);
                        if *horizontal >= 2 && *vertical >= 2 {
                            return;
                        }
                    }
                }
                return;
            }
        }
        if dock.visible_dockables().is_some() {
            for item in dock
                .visible_dockables()
                .expect("source visible collection")
                .iter()
            {
                if let Some(child) = item.and_then(|item| item.as_proportional_dock()) {
                    Self::count_proportional_dock_children(child, horizontal, vertical);
                    if *horizontal >= 2 && *vertical >= 2 {
                        return;
                    }
                }
            }
        }
    }
}
#[cfg(test)]
#[path = "global_docking_helper_tests.rs"]
mod tests;
