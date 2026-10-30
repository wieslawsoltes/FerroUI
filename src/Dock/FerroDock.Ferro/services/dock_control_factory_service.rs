// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::i_dock_control_factory_service::IDockControlFactoryService;
use crate::{controls::DockControl, model_projection};
use ferrodock_controls_recycling::{
    ControlRecycling, ControlRecyclingDataTemplate, IControlRecycling,
};
use ferrodock_model::{
    collections::ListHandle,
    core::{IDock, IDockWindow, IDockable, IFactory},
};
use ferroui_base::{data::core::ValueTypes, BoxedValue, Ref};
use ferroui_system::collections::generic::Dictionary;
use std::{
    any::TypeId,
    cell::RefCell,
    collections::{hash_map::Entry, HashMap},
    rc::{Rc, Weak},
};
type RecyclingEntry = (Weak<dyn IFactory>, Rc<dyn IControlRecycling>);
// Access pruning cannot reproduce CLR ephemeron GC when a value retains its factory key.
ferroui_base::ferro_thread_local! { static CONTROL_RECYCLING:RefCell<HashMap<usize,RecyclingEntry>>=RefCell::new(HashMap::new()); }
// Add-only .NET HashSet traversal uses insertion slots. Shared Dictionary preserves that
// order and retains the original references; no mirror model tree or random HashMap order.
type Dockables = Dictionary<Option<Rc<dyn IDockable>>, ()>;
pub(crate) struct DockControlFactoryService;
impl IDockControlFactoryService for DockControlFactoryService {
    fn initialize_control_recycling(&self, control: &DockControl) {
        let Some(factory) = control.layout().and_then(|l| l.factory()) else {
            return;
        };
        let recycling = ControlRecyclingDataTemplate::get_control_recycling(control);
        if let Some(shared) = Self::get_shared(&factory) {
            if recycling.as_ref().is_some_and(|r| Rc::ptr_eq(r, &shared)) {
                return;
            }
            ControlRecyclingDataTemplate::set_control_recycling(control, Some(shared));
            return;
        }
        let recycling =
            recycling.unwrap_or_else(|| ControlRecycling::as_recycling(ControlRecycling::new()));
        ControlRecyclingDataTemplate::set_control_recycling(control, Some(recycling.clone()));
        CONTROL_RECYCLING.with(|store| {
            let mut store = store.borrow_mut();
            match store.entry(Self::identity(&factory)) {
                Entry::Vacant(entry) => {
                    entry.insert((Rc::downgrade(&factory), recycling));
                }
                Entry::Occupied(_) => panic!("source unique control-recycling factory key"),
            }
        });
    }
    fn cleanup_factory(&self, owner: &DockControl, layout: Rc<dyn IDock>) {
        let Some(factory) = layout.factory() else {
            return;
        };
        if !Self::has_other_dock_control_for_layout(&*factory, &layout) {
            Self::prune_factory_caches(&factory, layout);
        }
        Self::release_factory_defaults(factory, owner);
    }
}
impl DockControlFactoryService {
    pub(crate) fn new() -> Self {
        Self
    }
    fn identity(factory: &Rc<dyn IFactory>) -> usize {
        Rc::as_ptr(factory) as *const () as usize
    }
    fn get_shared(factory: &Rc<dyn IFactory>) -> Option<Rc<dyn IControlRecycling>> {
        let removed = CONTROL_RECYCLING.with(|store| {
            let mut store = store.borrow_mut();
            let expired: Vec<_> = store
                .iter()
                .filter_map(|(key, (factory, _))| factory.upgrade().is_none().then_some(*key))
                .collect();
            expired
                .into_iter()
                .filter_map(|key| store.remove(&key))
                .collect::<Vec<_>>()
        });
        drop(removed);
        CONTROL_RECYCLING.with(|store| {
            store
                .borrow()
                .get(&Self::identity(factory))
                .map(|(_, value)| value.clone())
        })
    }
    fn native_control(
        control: Rc<dyn ferrodock_model::core::IDockControl>,
    ) -> Option<Ref<DockControl>> {
        control
            .boxed_interface(TypeId::of::<Ref<DockControl>>())?
            .downcast_ref::<Ref<DockControl>>()
            .cloned()
    }
    fn has_other_dock_control_for_layout(factory: &dyn IFactory, layout: &Rc<dyn IDock>) -> bool {
        for control in factory.dock_controls().iter() {
            if let Some(control) = Self::native_control(control) {
                if control.layout().as_ref() != Some(layout) {
                    continue;
                }
                return true;
            }
        }
        false
    }
    fn prune_factory_caches(factory: &Rc<dyn IFactory>, layout: Rc<dyn IDock>) {
        let mut dockables = Dockables::new();
        Self::collect_all_dockables(Some(layout), &mut dockables);
        Self::prune_dictionary(factory.visible_dockable_controls(), &dockables);
        Self::prune_dictionary(factory.pinned_dockable_controls(), &dockables);
        Self::prune_dictionary(factory.tab_dockable_controls(), &dockables);
        Self::prune_dictionary(factory.visible_root_controls(), &dockables);
        Self::prune_dictionary(factory.pinned_root_controls(), &dockables);
        Self::prune_dictionary(factory.tab_root_controls(), &dockables);
        Self::prune_dictionary(factory.tool_controls(), &dockables);
        Self::prune_dictionary(factory.document_controls(), &dockables);
        Self::prune_control_recycling(factory, &dockables);
    }
    fn prune_dictionary<V>(
        dictionary: Rc<RefCell<Dictionary<Rc<dyn IDockable>, V>>>,
        dockables: &Dockables,
    ) {
        if dictionary.borrow().is_empty() {
            return;
        }
        let keys: Vec<_> = dictionary
            .borrow()
            .keys()
            .filter(|key| dockables.contains_key(&Some((*key).clone())))
            .cloned()
            .collect();
        for key in keys {
            let removed = dictionary.borrow_mut().remove(&key);
            drop(removed);
        }
    }
    fn prune_control_recycling(factory: &Rc<dyn IFactory>, dockables: &Dockables) {
        let Some(recycling) = Self::get_shared(factory) else {
            return;
        };
        let boxed: BoxedValue = Rc::new(recycling);
        let Some(recycling) =
            ValueTypes::as_object(&*boxed).and_then(|o| o.cast::<ControlRecycling>())
        else {
            return;
        };
        for dockable in dockables.keys() {
            let value = dockable.clone().map(model_projection::to_dockable_value);
            recycling.remove(value.as_ref());
        }
    }
    fn collect_all_dockables(item: Option<Rc<dyn IDockable>>, dockables: &mut Dockables) {
        if dockables.contains_key(&item) {
            return;
        }
        dockables.insert(item.clone(), ());
        let Some(item) = item else { return };
        if let Some(root) = item.clone().as_root_dock() {
            Self::add_dockables_all(root.hidden_dockables(), dockables);
            Self::add_dockables_all(root.left_pinned_dockables(), dockables);
            Self::add_dockables_all(root.right_pinned_dockables(), dockables);
            Self::add_dockables_all(root.top_pinned_dockables(), dockables);
            Self::add_dockables_all(root.bottom_pinned_dockables(), dockables);
            if let Some(pinned) = root.pinned_dock() {
                Self::collect_all_dockables(Some(pinned), dockables);
            }
            if root.windows().is_some() {
                for window in root.windows().expect("source windows re-read").iter() {
                    let window = window.expect("source window.Layout access");
                    if let Some(layout) = window.layout() {
                        Self::collect_all_dockables(Some(layout), dockables);
                    }
                }
            }
        }
        if let Some(dock) = item.clone().as_dock() {
            if dock.visible_dockables().is_some() {
                for child in dock
                    .visible_dockables()
                    .expect("source visible list re-read")
                    .iter()
                {
                    Self::collect_all_dockables(child, dockables);
                }
            }
        }
        if let Some(split) = item.clone().as_split_view_dock() {
            if let Some(pane) = split.pane_dockable() {
                Self::collect_all_dockables(Some(pane), dockables);
            }
            if let Some(content) = split.content_dockable() {
                if split
                    .pane_dockable()
                    .as_ref()
                    .is_none_or(|pane| !Rc::ptr_eq(&content, pane))
                {
                    Self::collect_all_dockables(Some(content), dockables);
                }
            }
        }
        // Source also permits custom models implementing both IDockable and IDockWindow.
        if let Some(window) = item
            .boxed_interface(TypeId::of::<Rc<dyn IDockWindow>>())
            .and_then(|v| v.downcast_ref::<Rc<dyn IDockWindow>>().cloned())
        {
            if let Some(layout) = window.layout() {
                Self::collect_all_dockables(Some(layout), dockables);
            }
        }
    }
    fn add_dockables_all(
        source: Option<ListHandle<Option<Rc<dyn IDockable>>>>,
        dockables: &mut Dockables,
    ) {
        if let Some(source) = source {
            for item in source.iter() {
                Self::collect_all_dockables(item, dockables);
            }
        }
    }
    fn release_factory_defaults(factory: Rc<dyn IFactory>, owner: &DockControl) {
        let remaining = factory
            .dock_controls()
            .iter()
            .filter_map(Self::native_control)
            .find(|control| !control.ptr_eq(&owner.to_ref()));
        if factory
            .default_context_locator()
            .as_ref()
            .is_some_and(|callback| owner.owns_context_callback(callback))
        {
            factory.set_default_context_locator(
                remaining
                    .as_ref()
                    .map(|control| control.default_context_callback()),
            );
        }
        if factory
            .default_host_window_locator()
            .as_ref()
            .is_some_and(|callback| owner.owns_host_callback(callback))
        {
            factory.set_default_host_window_locator(
                remaining
                    .as_ref()
                    .map(|control| control.default_host_window_callback()),
            );
        }
        let Some(locators) = factory.host_window_locator() else {
            return;
        };
        let callback = locators.borrow().get("IDockWindow").cloned().flatten();
        if callback
            .as_ref()
            .is_some_and(|callback| owner.owns_host_callback(callback))
        {
            if let Some(remaining) = remaining {
                let callback = remaining.default_host_window_callback();
                let old = locators
                    .borrow_mut()
                    .insert("IDockWindow".into(), Some(callback));
                drop(old);
            } else {
                let removed = locators.borrow_mut().remove("IDockWindow");
                drop(removed);
                if locators.borrow().is_empty()
                    && factory
                        .host_window_locator()
                        .is_some_and(|current| Rc::ptr_eq(&current, &locators))
                {
                    factory.set_host_window_locator(None);
                }
            }
        }
    }
}
