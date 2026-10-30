// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Read-only native view and original mutable IList access over the same source handle.
use ferrodock_model::collections::{IList, ListHandle};
use ferroui_base::{BoxedValue, PropertyValue};
use ferroui_controls::{
    IItemsList, ItemsChangedEventArgs, ItemsChangedHandler, ItemsSource, ItemsView, TypedItems,
};
use std::{any::Any, rc::Rc};
struct ModelItems<T: PropertyValue>(ListHandle<T>);
impl<T: PropertyValue> IItemsList for ModelItems<T> {
    fn count(&self) -> usize {
        self.0.count()
    }
    fn get_at(&self, index: usize) -> Option<BoxedValue> {
        ferroui_controls::box_item(&self.0.get(index))
    }
    fn index_of(&self, item: &Option<BoxedValue>) -> i32 {
        ferroui_base::metadata::from_markup_value::<T>(item)
            .and_then(|v| self.0.index_of(&v))
            .map_or(-1, |i| i as i32)
    }
    fn is_notifying(&self) -> bool {
        self.0.as_notify_collection_changed().is_some()
    }
    fn add_collection_changed(&self, handler: Rc<ItemsChangedHandler>) -> Option<u64> {
        self.0.add_collection_changed(Rc::new(move |e| {
            let new_items = TypedItems(e.new_items);
            let old_items = TypedItems(e.old_items);
            handler(&ItemsChangedEventArgs {
                action: e.action,
                new_items: ItemsView::from_typed(&new_items),
                old_items: ItemsView::from_typed(&old_items),
                new_starting_index: e.new_starting_index,
                old_starting_index: e.old_starting_index,
            });
        }))
    }
    fn remove_collection_changed(&self, token: u64) {
        self.0.remove_collection_changed(token);
    }
    fn identity(&self) -> Option<usize> {
        Some(self.0.identity())
    }
    fn as_any(&self) -> &dyn Any {
        &self.0
    }
}
/// No counts-only approximation is made: unsupported notifying implementations
/// must expose their existing typed System event through the IList query hooks.
pub(crate) fn items<T: PropertyValue>(value: &ListHandle<T>) -> Option<ItemsSource> {
    if value.as_notify_collection_changed().is_some() && !value.has_typed_collection_changes() {
        return None;
    }
    Some(ItemsSource::new(Rc::new(ModelItems(value.clone()))))
}
struct MutableModelItems<T: PropertyValue>(ListHandle<T>);
impl<T: PropertyValue> IList<Option<BoxedValue>> for MutableModelItems<T> {
    fn count(&self) -> usize {
        self.0.count()
    }
    fn get(&self, index: usize) -> Option<BoxedValue> {
        ferroui_controls::box_item(&self.0.get(index))
    }
    fn set(&self, index: usize, value: Option<BoxedValue>) {
        self.0.set(
            index,
            ferroui_base::metadata::from_markup_value::<T>(&value)
                .expect("source IList element type"),
        );
    }
    fn insert(&self, index: usize, value: Option<BoxedValue>) {
        self.0.insert(
            index,
            ferroui_base::metadata::from_markup_value::<T>(&value)
                .expect("source IList element type"),
        );
    }
    fn remove_at(&self, index: usize) {
        self.0.remove_at(index);
    }
    fn clear(&self) {
        self.0.clear();
    }
    fn index_of(&self, value: &Option<BoxedValue>) -> Option<usize> {
        ferroui_base::metadata::from_markup_value::<T>(value).and_then(|v| self.0.index_of(&v))
    }
    fn is_read_only(&self) -> bool {
        self.0.is_read_only()
    }
    fn version(&self) -> Option<u32> {
        self.0.as_list().version()
    }
    fn identity(&self) -> usize {
        self.0.identity()
    }
    fn reference_equals(&self, other: &dyn IList<Option<BoxedValue>>) -> bool {
        (other as &dyn Any)
            .downcast_ref::<Self>()
            .is_some_and(|o| self.0.ptr_eq(&o.0))
    }
    fn as_notify_collection_changed(
        &self,
    ) -> Option<&dyn ferroui_system::collections::specialized::INotifyCollectionChanged> {
        self.0.as_notify_collection_changed()
    }
    fn as_notify_property_changed(
        &self,
    ) -> Option<&dyn ferroui_system::component_model::INotifyPropertyChanged> {
        self.0.as_notify_property_changed()
    }
    fn has_typed_collection_changes(&self) -> bool {
        self.0.has_typed_collection_changes()
    }
    fn add_collection_changed(
        &self,
        handler: Rc<ferrodock_model::collections::ReadOnlyListChanged<Option<BoxedValue>>>,
    ) -> Option<u64> {
        self.0.add_collection_changed(Rc::new(move |e| {
            let new_items: Vec<_> = e.new_items.iter().map(ferroui_controls::box_item).collect();
            let old_items: Vec<_> = e.old_items.iter().map(ferroui_controls::box_item).collect();
            handler(
                &ferroui_system::collections::specialized::NotifyCollectionChangedEventArgs {
                    action: e.action,
                    new_items: &new_items,
                    old_items: &old_items,
                    new_starting_index: e.new_starting_index,
                    old_starting_index: e.old_starting_index,
                },
            );
        }))
    }
    fn remove_collection_changed(&self, token: u64) {
        self.0.remove_collection_changed(token);
    }
    fn get_enumerator(self: Rc<Self>) -> Box<dyn Iterator<Item = Option<BoxedValue>>> {
        Box::new(self.0.iter().map(|v| ferroui_controls::box_item(&v)))
    }
}
pub(crate) fn mutable<T: PropertyValue>(value: &ListHandle<T>) -> ListHandle<Option<BoxedValue>> {
    ListHandle::from_list(Rc::new(MutableModelItems(value.clone())))
}

#[cfg(test)]
#[path = "model_items_tests.rs"]
mod tests;
