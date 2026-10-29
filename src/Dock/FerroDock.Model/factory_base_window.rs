// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use super::tracking::same;
use super::FactoryBase;
use crate::controls::IRootDock;
use crate::core::*;
use std::collections::HashMap;
use std::rc::Rc;

impl FactoryBase {
    /// Resolves the outer workspace root that owns floating window registrations.
    pub fn resolve_window_collection_root(
        &self,
        root: Option<Rc<dyn IRootDock>>,
    ) -> Option<Rc<dyn IRootDock>> {
        let mut current = root.expect("source collection root.Window access");
        let mut visited = HashMap::new();
        visited.insert(Rc::as_ptr(&current).cast::<()>() as usize, current.clone());
        while let Some(parent) = current
            .window()
            .and_then(|window| window.owner())
            .and_then(|owner| owner.as_root_dock())
        {
            if visited
                .insert(Rc::as_ptr(&parent).cast::<()>() as usize, parent.clone())
                .is_some()
            {
                break;
            }
            current = parent;
        }
        Some(current)
    }

    /// Registers the window, raises added and initializes it in source order.
    pub fn add_window(&self, root: Option<Rc<dyn IRootDock>>, window: Option<Rc<dyn IDockWindow>>) {
        let root = root.expect("source root.Windows access");
        if root.windows().is_none() {
            root.set_windows(Some((self.create_window_list)(Vec::new())));
        }
        root.windows()
            .expect("source windows list exists")
            .add(window.clone());
        self.runtime().on_window_added(window.clone());
        self.runtime().init_dock_window(window, Some(root));
    }

    /// Inserts a window at a nonnegative index, then raises added and initializes it.
    pub fn insert_window(
        &self,
        root: Option<Rc<dyn IRootDock>>,
        window: Option<Rc<dyn IDockWindow>>,
        index: i32,
    ) {
        if index < 0 {
            return;
        }
        let root = root.expect("source root.Windows access");
        if root.windows().is_none() {
            root.set_windows(Some((self.create_window_list)(Vec::new())));
        }
        root.windows()
            .expect("source windows list exists")
            .insert(index as usize, window.clone());
        self.runtime().on_window_added(window.clone());
        self.runtime().init_dock_window(window, Some(root));
    }

    /// Exits the host, honors cancellation/ownership changes, unregisters and clears model links.
    pub fn remove_window(&self, window: Option<Rc<dyn IDockWindow>>) {
        let window = window.expect("source window.Owner access");
        let Some(root) = window.owner().and_then(|owner| owner.as_root_dock()) else {
            return;
        };
        let layout = window.layout();
        window.exit();
        if !same(&window.owner(), &Some(root.clone() as Rc<dyn IDockable>))
            || window.host().is_some_and(|host| {
                host.as_host_window_close_state()
                    .is_some_and(|state| !state.is_closed())
            })
        {
            return;
        }
        if let Some(windows) = root.windows() {
            windows.remove(&Some(window.clone()));
        }
        self.runtime().on_window_removed(Some(window.clone()));
        if let Some(layout) = layout {
            if same(&layout.window(), &Some(window.clone())) {
                layout.set_window(None);
            }
        }
        window.set_parent_window(None);
        window.set_owner(None);
        window.set_factory(None);
        window.set_layout(None);
    }

    /// Requests descendant closure using snapshots so close callbacks may mutate live lists.
    pub fn close_window(&self, window: Option<Rc<dyn IDockWindow>>) {
        let window = window.expect("source window.Layout access");
        if let Some(layout) = window.layout() {
            self.close_window_dock(layout);
        }
    }

    fn close_window_dock(&self, dock: Rc<dyn IDock>) {
        if dock.visible_dockables().is_none() {
            return;
        }
        for item in dock
            .visible_dockables()
            .expect("source visible list exists")
            .to_vec()
        {
            if let Some(child) = item.clone().and_then(|item| item.as_dock()) {
                self.close_window_dock(child);
            }
            self.runtime().close_dockable(item);
        }
    }
}
