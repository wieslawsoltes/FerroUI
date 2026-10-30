// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use super::FactoryBase;
use crate::collections::ListHandle;
use crate::core::IDockable;
use crate::value_semantics::SourceItemKey;
use crate::BoxedValue;
use ferroui_system::collections::generic::Dictionary;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

type WeakEntry<T> = (Weak<dyn IDockable>, T);
type ContainerList = ListHandle<Option<Rc<dyn IDockable>>>;

/// Native state for the two source identity-keyed weak tables and item dictionary.
/// Values retain their source ownership: the reverse item dictionary owns containers,
/// and a live owner mapping retains its owner. Expired weak keys are pruned on access.
/// Full CLR ephemeron collection of owner→key cycles is still a runtime parity gap;
/// explicit Untrack releases those values, as in the source control lifecycle.
#[derive(Default)]
pub(super) struct ItemsSourceTracking {
    owners: RefCell<HashMap<usize, WeakEntry<Rc<dyn IDockable>>>>,
    items: RefCell<HashMap<usize, WeakEntry<SourceItemKey>>>,
    by_item: RefCell<Dictionary<SourceItemKey, ContainerList>>,
}

fn identity(dockable: &Rc<dyn IDockable>) -> usize {
    Rc::as_ptr(dockable) as *const () as usize
}

fn prune<T>(table: &RefCell<HashMap<usize, WeakEntry<T>>>) {
    let expired: Vec<usize> = table
        .borrow()
        .iter()
        .filter_map(|(id, (key, _))| (key.strong_count() == 0).then_some(*id))
        .collect();
    for id in expired {
        // Drop retained values outside the borrow, including user-defined destructors.
        let removed = table.borrow_mut().remove(&id);
        drop(removed);
    }
}

impl FactoryBase {
    /// Tracks a generated dockable using its current Context as the source item.
    pub fn track_items_source_dockable(
        &self,
        dockable: Rc<dyn IDockable>,
        owner: Rc<dyn IDockable>,
    ) {
        self.hooks().track_items_source_dockable_with_item(
            dockable.clone(),
            owner,
            dockable.context(),
        );
    }

    /// Replaces source ownership and item mappings, retaining duplicate-item container order.
    pub fn track_items_source_dockable_with_item(
        &self,
        dockable: Rc<dyn IDockable>,
        owner: Rc<dyn IDockable>,
        item: Option<BoxedValue>,
    ) {
        prune(&self.items_source.owners);
        let id = identity(&dockable);
        let removed = self.items_source.owners.borrow_mut().remove(&id);
        drop(removed);
        let replaced = self
            .items_source
            .owners
            .borrow_mut()
            .insert(id, (Rc::downgrade(&dockable), owner));
        drop(replaced);
        self.remove_tracked_items_source_item(dockable.clone());
        if let Some(item) = item {
            self.track_items_source_item(dockable, item);
        }
    }

    /// Removes the generated-container owner and every matching reverse item entry.
    pub fn untrack_items_source_dockable(&self, dockable: Rc<dyn IDockable>) {
        prune(&self.items_source.owners);
        let removed = self
            .items_source
            .owners
            .borrow_mut()
            .remove(&identity(&dockable));
        drop(removed);
        self.remove_tracked_items_source_item(dockable);
    }

    /// Gets the original source owner independently of the dockable's current layout owner.
    pub fn get_tracked_items_source_owner(
        &self,
        dockable: Rc<dyn IDockable>,
    ) -> Option<Rc<dyn IDockable>> {
        prune(&self.items_source.owners);
        self.items_source
            .owners
            .borrow()
            .get(&identity(&dockable))
            .filter(|(key, _)| Weak::ptr_eq(key, &Rc::downgrade(&dockable)))
            .map(|(_, owner)| owner.clone())
    }

    /// Finds the first still-tracked container for an equal source item, removing stale entries lazily.
    /// A null item returns None, preserving the source method's defensive null guard.
    pub fn get_container_from_item(&self, item: Option<BoxedValue>) -> Option<Rc<dyn IDockable>> {
        let item = item?;
        let key = SourceItemKey(item);
        let dockables = self.items_source.by_item.borrow().get(&key).cloned();
        if let Some(dockables) = dockables {
            while dockables.count() > 0 {
                let dockable = dockables
                    .get(0)
                    .expect("private source container lists only contain tracked objects");
                let tracked = self
                    .items_source
                    .owners
                    .borrow()
                    .get(&identity(&dockable))
                    .is_some_and(|(key, _)| Weak::ptr_eq(key, &Rc::downgrade(&dockable)));
                if tracked {
                    return Some(dockable);
                }
                dockables.remove_at(0);
            }
            let removed = self.items_source.by_item.borrow_mut().remove(&key);
            drop(removed);
        }
        None
    }

    fn track_items_source_item(&self, dockable: Rc<dyn IDockable>, item: BoxedValue) {
        prune(&self.items_source.items);
        let key = SourceItemKey(item);
        let replaced = self
            .items_source
            .items
            .borrow_mut()
            .insert(identity(&dockable), (Rc::downgrade(&dockable), key.clone()));
        drop(replaced);
        let dockables = self.items_source.by_item.borrow().get(&key).cloned();
        let dockables = dockables.unwrap_or_else(|| {
            let dockables = ListHandle::from_plain(Vec::new());
            self.items_source
                .by_item
                .borrow_mut()
                .insert(key, dockables.clone());
            dockables
        });
        dockables.add(Some(dockable));
    }

    fn remove_tracked_items_source_item(&self, dockable: Rc<dyn IDockable>) {
        prune(&self.items_source.items);
        let key = self
            .items_source
            .items
            .borrow()
            .get(&identity(&dockable))
            .filter(|(key, _)| Weak::ptr_eq(key, &Rc::downgrade(&dockable)))
            .map(|(_, key)| key.clone());
        let Some(key) = key else { return };
        let removed = self
            .items_source
            .items
            .borrow_mut()
            .remove(&identity(&dockable));
        drop(removed);
        let dockables = self.items_source.by_item.borrow().get(&key).cloned();
        if let Some(dockables) = dockables {
            for index in (0..dockables.count()).rev() {
                if dockables
                    .get(index)
                    .as_ref()
                    .is_some_and(|item| Rc::ptr_eq(item, &dockable))
                {
                    dockables.remove_at(index);
                }
            }
            if dockables.count() == 0 {
                let removed = self.items_source.by_item.borrow_mut().remove(&key);
                drop(removed);
            }
        }
    }
}
