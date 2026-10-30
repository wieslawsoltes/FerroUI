// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

//! Native projection of the original shared IReadOnlyList contract.
use ferrodock_model::collections::ReadOnlyListHandle;
use ferroui_base::{BoxedValue, PropertyValue};
use ferroui_controls::{
    IItemsList, ItemsChangedEventArgs, ItemsChangedHandler, ItemsSource, ItemsView, TypedItems,
};
use std::{any::Any, rc::Rc};
struct ReadOnlyItems<T: PropertyValue>(ReadOnlyListHandle<T>);
impl<T: PropertyValue> IItemsList for ReadOnlyItems<T> {
    fn count(&self) -> usize {
        self.0.count()
    }
    fn get_at(&self, index: usize) -> Option<BoxedValue> {
        ferroui_controls::box_item(&self.0.get(index))
    }
    fn index_of(&self, item: &Option<BoxedValue>) -> i32 {
        let Some(item) = ferroui_base::metadata::from_markup_value::<T>(item) else {
            return -1;
        };
        (0..self.0.count())
            .find(|&index| self.0.get(index) == item)
            .map_or(-1, |index| index as i32)
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
        self
    }
}
pub(crate) fn items<T: PropertyValue>(value: &ReadOnlyListHandle<T>) -> ItemsSource {
    ItemsSource::new(Rc::new(ReadOnlyItems(value.clone())))
}
