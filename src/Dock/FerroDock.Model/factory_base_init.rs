// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use super::FactoryBase;
use crate::collections::ListHandle;
use crate::controls::*;
use crate::core::*;
use ferroui_system::collections::generic::Dictionary;
use std::cell::RefCell;
use std::rc::Rc;

impl FactoryBase {
    /// Initializes a complete layout, establishes its initial selection and opens its windows.
    pub fn init_layout(&self, layout: Rc<dyn IDockable>) {
        if self.runtime().host_window_locator().is_none()
            && self.runtime().default_host_window_locator().is_some()
        {
            let locators = Rc::new(RefCell::new(Dictionary::new()));
            locators.borrow_mut().add(
                "IDockWindow".into(),
                self.runtime().default_host_window_locator(),
            );
            self.runtime().set_host_window_locator(Some(locators));
        }
        self.runtime().init_dockable(Some(layout.clone()), None);
        if let Some(dock) = layout.clone().as_dock() {
            if dock.default_dockable().is_some() {
                dock.set_active_dockable(dock.default_dockable());
            }
        }
        if let Some(root) = layout.as_root_dock() {
            if root.active_dockable().is_none() && root.visible_dockables().is_some() {
                if let Some(first) = Self::find_first_visible_dockable(root.clone()) {
                    root.set_active_dockable(Some(first));
                }
            }
            self.initialize_global_dock_tracking(root.clone());
            if root.show_windows().can_execute(None) {
                root.show_windows().execute(None);
            }
        }
    }

    /// Initializes context, ownership, factory links, child lists, state and the init event.
    pub fn init_dockable(
        &self,
        dockable: Option<Rc<dyn IDockable>>,
        owner: Option<Rc<dyn IDockable>>,
    ) {
        let dockable = dockable.expect("source InitDockable dereferences dockable.Context");
        if dockable.context().is_none() {
            if let Some(context) = self.runtime().get_context(dockable.id()) {
                dockable.set_context(Some(context));
            }
        }
        dockable.set_owner(owner);
        dockable.set_factory(Some(self.runtime()));
        if let Some(dock) = dockable.clone().as_dock() {
            let visible = self.ensure_observable_dockables(dock.visible_dockables());
            dock.set_visible_dockables(Some(visible.clone()));
            self.init_dockables(dockable.clone(), visible);
            self.update_is_empty(dock);
        }
        if let Some(split) = dockable.clone().as_split_view_dock() {
            self.init_split_view_dockables(split);
        }
        if let Some(root) = dockable.clone().as_root_dock() {
            if root.hidden_dockables().is_some() {
                self.init_dockables(
                    dockable.clone(),
                    root.hidden_dockables().expect("source hidden list exists"),
                );
            }
            for get in [
                IRootDock::left_pinned_dockables,
                IRootDock::right_pinned_dockables,
                IRootDock::top_pinned_dockables,
                IRootDock::bottom_pinned_dockables,
            ] {
                if get(root.as_ref()).is_some() {
                    self.init_dockables(
                        root.clone(),
                        get(root.as_ref()).expect("source pinned list exists"),
                    );
                }
            }
            if root.pinned_dock().is_some() {
                self.runtime().init_dockable(
                    Some(root.pinned_dock().expect("source pinned dock exists")),
                    Some(root.clone()),
                );
            }
            if root.windows().is_some() {
                for child in root.windows().expect("source window list exists").iter() {
                    self.runtime()
                        .init_dock_window(child, Some(dockable.clone()));
                }
            }
        }
        self.hooks()
            .update_docking_window_state(Some(dockable.clone()));
        self.runtime().on_dockable_init(Some(dockable));
    }

    fn find_first_visible_dockable(dock: Rc<dyn IDock>) -> Option<Rc<dyn IDockable>> {
        dock.visible_dockables()?;
        for item in dock
            .visible_dockables()
            .expect("source visible list exists")
            .iter()
        {
            if item.clone().and_then(|item| item.as_splitter()).is_none() {
                return item;
            }
        }
        for item in dock
            .visible_dockables()
            .expect("source visible list exists")
            .iter()
        {
            if let Some(child) = item.and_then(|item| item.as_dock()) {
                if let Some(result) = Self::find_first_visible_dockable(child) {
                    return Some(result);
                }
            }
        }
        None
    }

    fn init_dockables(
        &self,
        owner: Rc<dyn IDockable>,
        dockables: ListHandle<Option<Rc<dyn IDockable>>>,
    ) {
        for child in dockables.iter() {
            self.runtime().init_dockable(child, Some(owner.clone()));
        }
    }

    fn init_split_view_dockables(&self, split: Rc<dyn ISplitViewDock>) {
        let dock: Rc<dyn IDock> = split.clone();
        let pane = split.pane_dockable();
        if let Some(pane) = &pane {
            if !dock
                .visible_dockables()
                .is_some_and(|items| items.contains(&Some((pane).clone())))
            {
                self.runtime()
                    .init_dockable(Some(pane.clone()), Some(split.clone()));
            }
        }
        if let Some(content) = split.content_dockable() {
            if !pane.as_ref().is_some_and(|pane| Rc::ptr_eq(pane, &content))
                && !dock
                    .visible_dockables()
                    .is_some_and(|items| items.contains(&Some((content).clone())))
            {
                self.runtime().init_dockable(Some(content), Some(split));
            }
        }
    }

    /// Initializes selection, invokes the dockable hook, conditionally focuses and synchronizes siblings.
    pub fn init_active_dockable(
        &self,
        dockable: Option<Rc<dyn IDockable>>,
        owner: Option<Rc<dyn IDock>>,
    ) {
        self.runtime().on_active_dockable_changed(dockable.clone());
        if let Some(dockable) = &dockable {
            self.runtime().init_dockable(
                Some(dockable.clone()),
                owner.clone().map(|owner| owner as Rc<dyn IDockable>),
            );
            dockable.on_selected();
        }
        if let Some(dockable) = dockable {
            if self
                .hooks()
                .should_focus_active_dockable(Some(dockable.clone()), owner.clone())
            {
                self.runtime()
                    .set_focused_dockable(owner.clone(), Some(dockable));
            }
        }
        self.synchronize_docking_window_selection(owner);
    }

    /// Activates the containing root's window and raises the factory event.
    pub fn activate_window(&self, dockable: Option<Rc<dyn IDockable>>) {
        if let Some(root) = self.runtime().find_root(dockable, None) {
            if root.window().is_some() {
                root.window()
                    .expect("source root window exists")
                    .set_active();
                self.runtime().on_window_activated(root.window());
            }
        }
    }

    /// Selects a dockable in its owner and raises the factory activation event.
    pub fn set_active_dockable(&self, dockable: Option<Rc<dyn IDockable>>) {
        let dockable = dockable.expect("source SetActiveDockable dereferences dockable.Owner");
        if let Some(owner) = dockable.owner().and_then(|owner| owner.as_dock()) {
            owner.set_active_dockable(Some(dockable.clone()));
            self.runtime().on_dockable_activated(Some(dockable));
        }
    }

    /// Performs the source EnsureObservableList normalization for visible dockables.
    /// Notifying custom lists retain identity. A nonnotifying factory-created list
    /// is replaced by the shared FerroUI observable implementation before copying.
    pub fn ensure_observable_dockables(
        &self,
        items: Option<ListHandle<Option<Rc<dyn IDockable>>>>,
    ) -> ListHandle<Option<Rc<dyn IDockable>>> {
        if let Some(items) = &items {
            if items.as_notify_collection_changed().is_some() {
                return items.clone();
            }
        }
        let mut observable = (self.create_dockable_list)(Vec::new());
        if observable.as_notify_collection_changed().is_none() {
            observable = ListHandle::new();
        }
        if let Some(items) = items {
            for item in items.iter() {
                observable.add(item);
            }
        }
        observable
    }

    /// Initializes a window using the optional full-window factory or ID locator.
    pub fn init_dock_window(
        &self,
        window: Option<Rc<dyn IDockWindow>>,
        owner: Option<Rc<dyn IDockable>>,
    ) {
        let runtime = self.runtime();
        let host = if let Some(factory) = runtime.clone().as_dock_window_host_factory() {
            factory.create_host_window(window.clone())
        } else {
            runtime.get_host_window(window.as_ref().expect("source window.Id access").id())
        };
        // Source virtual overload dispatch remains on the enclosing factory.
        runtime.init_dock_window_with_host(window, owner, host);
    }

    /// Initializes the explicit host/model links, owner, factory and layout in source order.
    pub fn init_dock_window_with_host(
        &self,
        window: Option<Rc<dyn IDockWindow>>,
        owner: Option<Rc<dyn IDockable>>,
        host: Option<Rc<dyn IHostWindow>>,
    ) {
        let window = window.expect("source window.Host access");
        window.set_host(host);
        if window.host().is_some() {
            window
                .host()
                .expect("source host access requires a host")
                .set_window(Some(window.clone()));
        }
        window.set_owner(owner);
        window.set_factory(Some(self.runtime()));
        if window.layout().is_some() {
            let layout = window.layout();
            let owner = window
                .layout()
                .expect("source layout access requires a layout")
                .owner();
            self.runtime()
                .init_dockable(layout.map(|layout| layout as Rc<dyn IDockable>), owner);
        }
    }
}
