// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use super::{DockableQuery, FactoryBase};
use crate::controls::IRootDock;
use crate::core::*;
use crate::BoxedValue;
use std::rc::Rc;

macro_rules! locator_property {
    ($name:ident, $setter:ident, $type:ty, $label:literal) => {
        impl FactoryBase {
            #[doc = concat!("Gets `", $label, "`.")]
            pub fn $name(&self) -> Option<$type> {
                self.$name.borrow().clone()
            }
            #[doc = concat!("Sets `", $label, "` without invoking the locator.")]
            pub fn $setter(&self, value: Option<$type>) {
                let previous = self.$name.replace(value);
                drop(previous);
            }
        }
    };
}
locator_property!(
    default_context_locator,
    set_default_context_locator,
    ContextFactory,
    "DefaultContextLocator"
);
locator_property!(
    default_host_window_locator,
    set_default_host_window_locator,
    HostWindowFactory,
    "DefaultHostWindowLocator"
);
locator_property!(
    context_locator,
    set_context_locator,
    ContextLocatorMap,
    "ContextLocator"
);
locator_property!(
    host_window_locator,
    set_host_window_locator,
    HostWindowLocatorMap,
    "HostWindowLocator"
);
locator_property!(
    dockable_locator,
    set_dockable_locator,
    DockableLocatorMap,
    "DockableLocator"
);

impl FactoryBase {
    /// Resolves an ID once and casts the located object to the requested dockable contract or concrete type.
    /// Interface hooks and exact concrete/custom reference casts retain original object identity.
    /// Concrete base-class casts for composed subclasses await native class metadata.
    pub fn get_dockable<T: IDockable + ?Sized + 'static>(&self, id: String) -> Option<Rc<T>> {
        if id.is_empty() {
            return None;
        }
        let locator = self
            .runtime()
            .dockable_locator()
            .and_then(|locators| locators.borrow().get(&id).cloned())?;
        let dockable = locator?()?;
        crate::native_reference::cast_dockable(dockable)
    }

    /// Gets whether tools are hidden instead of closed.
    pub fn hide_tools_on_close(&self) -> bool {
        self.hide_tools_on_close.get()
    }
    /// Sets whether tools are hidden instead of closed.
    pub fn set_hide_tools_on_close(&self, value: bool) {
        self.hide_tools_on_close.set(value);
    }
    /// Gets whether documents are hidden instead of closed.
    pub fn hide_documents_on_close(&self) -> bool {
        self.hide_documents_on_close.get()
    }
    /// Sets whether documents are hidden instead of closed.
    pub fn set_hide_documents_on_close(&self, value: bool) {
        self.hide_documents_on_close.set(value);
    }

    /// Resolves a context by ID, using the default only when the key is absent.
    pub fn get_context(&self, id: String) -> Option<BoxedValue> {
        if id.is_empty() {
            return None;
        }
        let runtime = self.runtime();
        let locator = runtime
            .context_locator()
            .and_then(|locators| locators.borrow().get(&id).cloned());
        if let Some(locator) = locator {
            return locator.and_then(|locator| locator());
        }
        runtime
            .default_context_locator()
            .and_then(|locator| locator())
    }

    /// Resolves a host by ID, using the default only when the key is absent.
    pub fn get_host_window(&self, id: String) -> Option<Rc<dyn IHostWindow>> {
        if id.is_empty() {
            return None;
        }
        let runtime = self.runtime();
        let locator = runtime
            .host_window_locator()
            .and_then(|locators| locators.borrow().get(&id).cloned());
        if let Some(locator) = locator {
            return locator.and_then(|locator| locator());
        }
        runtime
            .default_host_window_locator()
            .and_then(|locator| locator())
    }

    /// Finds a root using the source predicate order and virtual recursive dispatch.
    pub fn find_root(
        &self,
        dockable: Option<Rc<dyn IDockable>>,
        predicate: Option<RootPredicate>,
    ) -> Option<Rc<dyn IRootDock>> {
        if let Some(root) = dockable.clone().and_then(|item| item.as_root_dock()) {
            if predicate
                .as_ref()
                .is_none_or(|predicate| predicate(root.clone()))
            {
                return Some(root);
            }
        }
        let dockable = dockable.as_ref().expect("source dockable.Owner access");
        dockable.owner()?;
        if let Some(root) = dockable.owner().and_then(|owner| owner.as_root_dock()) {
            if predicate
                .as_ref()
                .is_none_or(|predicate| predicate(root.clone()))
            {
                return Some(root);
            }
        }
        self.runtime().find_root(dockable.owner(), predicate)
    }

    /// Searches visible children, split-view extras and floating window layouts in source order.
    pub fn find_dockable(
        &self,
        dock: Option<Rc<dyn IDock>>,
        predicate: Option<DockablePredicate>,
    ) -> Option<Rc<dyn IDockable>> {
        let predicate = predicate.expect("source predicate invocation");
        let item = dock.clone().map(|dock| dock as Rc<dyn IDockable>);
        if predicate(item.clone()) {
            return item;
        }
        let dock = dock.expect("source dock.VisibleDockables access");
        if dock.visible_dockables().is_some() {
            for dockable in dock
                .visible_dockables()
                .expect("source visible access requires a list")
                .iter()
            {
                if predicate(dockable.clone()) {
                    return dockable;
                }
                if let Some(child) = dockable.and_then(|item| item.as_dock()) {
                    if let Some(result) = self
                        .runtime()
                        .find_dockable(Some(child), Some(predicate.clone()))
                    {
                        return Some(result);
                    }
                }
            }
        }
        if let Some(split) = dock.clone().as_split_view_dock() {
            let pane = split.pane_dockable();
            if let Some(pane) = &pane {
                if !dock
                    .visible_dockables()
                    .is_some_and(|visible| visible.contains(&Some((pane).clone())))
                {
                    if predicate(Some(pane.clone())) {
                        return Some(pane.clone());
                    }
                    if let Some(child) = pane.clone().as_dock() {
                        if let Some(result) = self
                            .runtime()
                            .find_dockable(Some(child), Some(predicate.clone()))
                        {
                            return Some(result);
                        }
                    }
                }
            }
            if let Some(content) = split.content_dockable() {
                if pane.as_ref().is_none_or(|pane| !Rc::ptr_eq(pane, &content))
                    && !dock
                        .visible_dockables()
                        .is_some_and(|visible| visible.contains(&Some((content).clone())))
                {
                    if predicate(Some(content.clone())) {
                        return Some(content);
                    }
                    if let Some(child) = content.as_dock() {
                        if let Some(result) = self
                            .runtime()
                            .find_dockable(Some(child), Some(predicate.clone()))
                        {
                            return Some(result);
                        }
                    }
                }
            }
        }
        if let Some(root) = dock.as_root_dock() {
            if root.windows().is_some() {
                for window in root
                    .windows()
                    .expect("source windows access requires a list")
                    .iter()
                {
                    let window = window.expect("source window.Layout access");
                    if window.layout().is_none() {
                        continue;
                    }
                    if predicate(window.layout().map(|layout| layout as Rc<dyn IDockable>)) {
                        return window.layout().map(|layout| layout as Rc<dyn IDockable>);
                    }
                    if let Some(result) = self.runtime().find_dockable(
                        window.layout().map(|layout| layout as Rc<dyn IDock>),
                        Some(predicate.clone()),
                    ) {
                        return Some(result);
                    }
                }
            }
        }
        None
    }

    /// Returns a lazy, repeatable search over all registered dock controls.
    pub fn find(&self, predicate: Option<DockablePredicate>) -> DockableQuery {
        DockableQuery::all(self.runtime(), predicate)
    }
    /// Returns a lazy, repeatable search over the dock's visible tree.
    pub fn find_in_dock(
        &self,
        dock: Option<Rc<dyn IDock>>,
        predicate: Option<DockablePredicate>,
    ) -> DockableQuery {
        DockableQuery::in_dock(self.runtime(), dock, predicate)
    }
}
