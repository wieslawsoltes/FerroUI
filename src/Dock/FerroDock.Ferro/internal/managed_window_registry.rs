// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Original managed dock/layer registry; native weak-key pruning is not CLR ephemeron GC.
use crate::controls::{ManagedWindowDock, ManagedWindowLayer};
use ferrodock_model::core::{IDockable, IFactory};
use ferroui_base::Ref;
use std::{
    cell::RefCell,
    collections::HashMap,
    rc::{Rc, Weak},
};
type DockEntry = (Weak<dyn IFactory>, Rc<ManagedWindowDock>);
type LayerEntry = (Weak<dyn IFactory>, Ref<ManagedWindowLayer>);
ferroui_base::ferro_thread_local! {
    static DOCKS:RefCell<HashMap<usize,DockEntry>>=RefCell::new(HashMap::new());
    static LAYERS:RefCell<HashMap<usize,LayerEntry>>=RefCell::new(HashMap::new());
}
pub(crate) struct ManagedWindowRegistry;
impl ManagedWindowRegistry {
    pub(crate) fn get_or_create_dock(factory: &Rc<dyn IFactory>) -> Rc<ManagedWindowDock> {
        prune();
        let id = identity(factory);
        if let Some(dock) = DOCKS.with(|d| d.borrow().get(&id).map(|(_, dock)| dock.clone())) {
            return dock;
        }
        let dock = ManagedWindowDock::new();
        dock.set_id("ManagedWindowDock".into());
        dock.set_title("Managed Windows".into());
        dock.set_factory(Some(factory.clone()));
        DOCKS.with(|d| {
            assert!(
                d.borrow_mut()
                    .insert(id, (Rc::downgrade(factory), dock.clone()))
                    .is_none(),
                "source unique factory key"
            );
        });
        dock
    }
    pub(crate) fn try_get_layer(factory: &Rc<dyn IFactory>) -> Option<Ref<ManagedWindowLayer>> {
        prune();
        LAYERS.with(|l| {
            l.borrow()
                .get(&identity(factory))
                .map(|(_, layer)| layer.clone())
        })
    }
    pub(crate) fn register_layer(factory: &Rc<dyn IFactory>, layer: &Ref<ManagedWindowLayer>) {
        if Self::try_get_layer(factory).as_ref() == Some(layer) {
            layer.set_dock(Some(Self::get_or_create_dock(factory)));
            return;
        }
        if let Some(existing) = Self::try_get_layer(factory) {
            existing.set_dock(None);
            existing.set_is_visible(false);
            let removed = LAYERS.with(|l| l.borrow_mut().remove(&identity(factory)));
            drop(removed);
        }
        LAYERS.with(|l| {
            assert!(
                l.borrow_mut()
                    .insert(identity(factory), (Rc::downgrade(factory), layer.clone()))
                    .is_none(),
                "source unique factory key"
            );
        });
        layer.set_dock(Some(Self::get_or_create_dock(factory)));
    }
    pub(crate) fn unregister_layer(factory: &Rc<dyn IFactory>, layer: &Ref<ManagedWindowLayer>) {
        if Self::try_get_layer(factory).as_ref() == Some(layer) {
            let removed = LAYERS.with(|l| l.borrow_mut().remove(&identity(factory)));
            drop(removed);
            layer.set_dock(None);
            layer.set_is_visible(false);
        }
    }
}
fn identity(factory: &Rc<dyn IFactory>) -> usize {
    Rc::as_ptr(factory) as *const () as usize
}
fn prune() {
    let removed = DOCKS.with(|d| {
        let mut map = d.borrow_mut();
        let keys: Vec<_> = map
            .iter()
            .filter_map(|(id, (key, _))| key.upgrade().is_none().then_some(*id))
            .collect();
        keys.into_iter()
            .filter_map(|id| map.remove(&id))
            .collect::<Vec<_>>()
    });
    drop(removed);
    let removed = LAYERS.with(|d| {
        let mut map = d.borrow_mut();
        let keys: Vec<_> = map
            .iter()
            .filter_map(|(id, (key, _))| key.upgrade().is_none().then_some(*id))
            .collect();
        keys.into_iter()
            .filter_map(|id| map.remove(&id))
            .collect::<Vec<_>>()
    });
    drop(removed);
}
