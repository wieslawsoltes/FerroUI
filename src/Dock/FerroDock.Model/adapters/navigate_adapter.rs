// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use super::AdapterOwner;
use crate::controls::ISplitViewDock;
use crate::core::{IDock, IDockable, INavigateAdapter};
use crate::BoxedValue;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// Navigation history, child lookup, and window commands for a dock.
/// Standalone adapters retain their dock; embedded adapters use a weak backlink.
pub struct NavigateAdapter {
    back: RefCell<Vec<Rc<dyn IDockable>>>,
    forward: RefCell<Vec<Rc<dyn IDockable>>>,
    dock: AdapterOwner<dyn IDock>,
}

impl NavigateAdapter {
    /// Creates a standalone adapter retaining its dock, with empty history.
    pub fn new(dock: Rc<dyn IDock>) -> Self {
        Self {
            back: RefCell::new(Vec::new()),
            forward: RefCell::new(Vec::new()),
            dock: AdapterOwner::Strong(dock),
        }
    }

    /// Creates the adapter while its containing dock is being constructed.
    /// The owner must outlive operations through this weak construction path.
    pub fn from_weak(dock: Weak<dyn IDock>) -> Self {
        Self {
            back: RefCell::new(Vec::new()),
            forward: RefCell::new(Vec::new()),
            dock: AdapterOwner::Weak(dock),
        }
    }

    fn dock(&self) -> Rc<dyn IDock> {
        self.dock
            .get()
            .expect("the containing dock must be alive during navigation")
    }

    fn reset_active_dockable(&self) {
        let dock = self.dock();
        if let Some(root) = dock
            .active_dockable()
            .and_then(|active| active.as_root_dock())
        {
            let exit = root.exit_windows();
            if exit.can_execute(None) {
                exit.execute(None);
            }
        }
        dock.set_active_dockable(None);
    }

    fn push_back(&self, dockable: Rc<dyn IDockable>) {
        self.forward.borrow_mut().clear();
        self.back.borrow_mut().push(dockable);
    }

    fn navigate_to(&self, dockable: Rc<dyn IDockable>, snapshot: bool) {
        let dock = self.dock();
        if let Some(previous) = dock
            .active_dockable()
            .and_then(|active| active.as_root_dock())
        {
            let exit = previous.exit_windows();
            if exit.can_execute(None) {
                exit.execute(None);
            }
        }
        let active = dock.active_dockable();
        if active
            .as_ref()
            .is_none_or(|active| !Rc::ptr_eq(active, &dockable))
        {
            if snapshot {
                if let Some(active) = active {
                    self.push_back(active);
                }
            }
            if let Some(factory) = dock.factory() {
                factory.set_active_dockable(Some(dockable.clone()));
            } else {
                dock.set_active_dockable(Some(dockable.clone()));
            }
        }
        if let Some(next) = dockable.as_root_dock() {
            let show = next.show_windows();
            if show.can_execute(None) {
                show.execute(None);
            }
        }
    }

    fn navigate_to_use_visible(&self, id: &str, snapshot: bool) {
        let dock = self.dock();
        if let Some(visible) = dock.visible_dockables() {
            for child in visible.iter() {
                if child
                    .as_ref()
                    .expect("source predicate v.Id dereference")
                    .id()
                    == id
                {
                    self.navigate_to(child.expect("matched source item"), snapshot);
                    return;
                }
            }
        }
        let mut roots = dock
            .visible_dockables()
            .map(|visible| visible.to_vec())
            .unwrap_or_default();
        if let Some(split) = dock.as_split_view_dock() {
            Self::add_split_view_dockables(&mut roots, split.as_ref());
        }
        if let Some(found) = Self::find_flattened(roots, id) {
            self.navigate_to(found, snapshot);
        }
    }

    // EnumerableExtensions.Flatten emits each sibling's descendants first, then
    // all the original siblings. Snapshot each child sequence when reached,
    // keeping lookup lazy so later subtrees are untouched after a match.
    fn find_flattened(
        siblings: Vec<Option<Rc<dyn IDockable>>>,
        id: &str,
    ) -> Option<Rc<dyn IDockable>> {
        for sibling in &siblings {
            let children =
                if let Some(split) = sibling.clone().and_then(|item| item.as_split_view_dock()) {
                    let mut children = split
                        .visible_dockables()
                        .map(|visible| visible.to_vec())
                        .unwrap_or_default();
                    Self::add_split_view_dockables(&mut children, split.as_ref());
                    children
                } else {
                    sibling
                        .clone()
                        .and_then(|item| item.as_dock())
                        .and_then(|dock| dock.visible_dockables())
                        .map(|visible| visible.to_vec())
                        .unwrap_or_default()
                };
            if let Some(found) = Self::find_flattened(children, id) {
                return Some(found);
            }
        }
        siblings
            .into_iter()
            .find(|sibling| {
                sibling
                    .as_ref()
                    .expect("source predicate v.Id dereference")
                    .id()
                    == id
            })
            .flatten()
    }

    fn add_split_view_dockables(
        dockables: &mut Vec<Option<Rc<dyn IDockable>>>,
        split: &dyn ISplitViewDock,
    ) {
        let pane = split.pane_dockable();
        if let Some(pane) = &pane {
            if !dockables.contains(&Some(pane.clone())) {
                dockables.push(Some(pane.clone()));
            }
        }
        if let Some(content) = split.content_dockable() {
            if pane.as_ref().is_none_or(|pane| !Rc::ptr_eq(pane, &content))
                && !dockables.contains(&Some(content.clone()))
            {
                dockables.push(Some(content));
            }
        }
    }
}

impl NavigateAdapter {
    /// Returns whether the back stack has an entry.
    pub fn can_go_back(&self) -> bool {
        !self.back.borrow().is_empty()
    }
    /// Returns whether the forward stack has an entry.
    pub fn can_go_forward(&self) -> bool {
        !self.forward.borrow().is_empty()
    }

    /// Navigates to the last entry on the back stack.
    pub fn go_back(&self) {
        let previous = self.back.borrow_mut().pop();
        if let Some(previous) = previous {
            if let Some(active) = self.dock().active_dockable() {
                self.forward.borrow_mut().push(active);
            }
            self.navigate_to(previous, false);
        }
    }

    /// Navigates to the last entry on the forward stack.
    pub fn go_forward(&self) {
        let next = self.forward.borrow_mut().pop();
        if let Some(next) = next {
            if let Some(active) = self.dock().active_dockable() {
                self.back.borrow_mut().push(active);
            }
            self.navigate_to(next, false);
        }
    }

    /// Navigates to a boxed IDockable handle or string ID; None resets the dock.
    ///
    /// # Panics
    /// An incompatible parameter corresponds to the source ArgumentException.
    /// Navigates to an object or ID, or resets the active dockable.
    pub fn navigate(&self, root: Option<BoxedValue>, snapshot: bool) {
        match root {
            None => {
                self.reset_active_dockable();
                self.back.borrow_mut().clear();
                self.forward.borrow_mut().clear();
            }
            Some(root) => {
                if let Some(dockable) = root.downcast_ref::<Rc<dyn IDockable>>() {
                    self.navigate_to(dockable.clone(), snapshot);
                } else if let Some(id) = root.downcast_ref::<String>() {
                    self.navigate_to_use_visible(id, snapshot);
                } else {
                    panic!("Invalid root object type: {:?}", root.as_ref().type_id());
                }
            }
        }
    }

    /// Presents root windows and delegates to the active root.
    pub fn show_windows(&self) {
        let dock = self.dock();
        if let Some(windows) = dock.clone().as_root_dock().and_then(|root| root.windows()) {
            for window in windows.iter() {
                let window = window.expect("source window.IsModal access");
                window.present(window.is_modal());
            }
        }
        if let Some(root) = dock
            .active_dockable()
            .and_then(|active| active.as_root_dock())
        {
            let show = root.show_windows();
            if show.can_execute(None) {
                show.execute(None);
            }
        }
    }

    /// Saves and exits a snapshot of the root windows.
    pub fn exit_windows(&self) {
        let dock = self.dock();
        if let Some(windows) = dock.clone().as_root_dock().and_then(|root| root.windows()) {
            for window in windows.to_vec() {
                let window = window.expect("source window.Save access");
                window.save();
                window.exit();
            }
        }
        if let Some(root) = dock
            .active_dockable()
            .and_then(|active| active.as_root_dock())
        {
            let exit = root.exit_windows();
            if exit.can_execute(None) {
                exit.execute(None);
            }
        }
    }

    /// Closes through the root command or the active dockable factory.
    pub fn close(&self) {
        let dock = self.dock();
        if let Some(root) = dock.clone().as_root_dock() {
            let exit = root.exit_windows();
            if exit.can_execute(None) {
                exit.execute(None);
            }
            return;
        }
        if let Some(factory) = dock.factory() {
            if let Some(active) = dock.active_dockable() {
                factory.close_dockable(Some(active));
            }
        }
    }
}

impl INavigateAdapter for NavigateAdapter {
    fn can_go_back(&self) -> bool {
        NavigateAdapter::can_go_back(self)
    }
    fn can_go_forward(&self) -> bool {
        NavigateAdapter::can_go_forward(self)
    }
    fn go_back(&self) {
        NavigateAdapter::go_back(self)
    }
    fn go_forward(&self) {
        NavigateAdapter::go_forward(self)
    }
    fn navigate(&self, root: Option<BoxedValue>, snapshot: bool) {
        NavigateAdapter::navigate(self, root, snapshot)
    }
    fn show_windows(&self) {
        NavigateAdapter::show_windows(self)
    }
    fn exit_windows(&self) {
        NavigateAdapter::exit_windows(self)
    }
    fn close(&self) {
        NavigateAdapter::close(self)
    }
}
