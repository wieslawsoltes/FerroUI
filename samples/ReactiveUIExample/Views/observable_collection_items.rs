//! The items of an `ObservableCollectionExtended` as the items source of an items control.
//!
//! Not an upstream file: the DynamicData page is an addition of the port, and in .NET an
//! `ObservableCollection<T>` is an `IList` that an items control reads directly. The items controls
//! of FerroUI read an [`IItemsList`]; this list reads the collection that `sort_and_bind` fills and
//! forwards its change notifications.

use ferroui_base::{BoxedValue, PropertyValue};
use ferroui_controls::{box_item, IItemsList, ItemsChangedEventArgs, ItemsChangedHandler, ItemsSource, ItemsView, TypedItems};
use ferroui_dynamic_data::binding::ObservableCollectionExtended;
use ferroui_system::collections::specialized::NotifyCollectionChangedEventArgs;
use std::any::Any;
use std::rc::Rc;

/// The items of an [`ObservableCollectionExtended`], read and observed by an items control.
pub struct ObservableCollectionItems<T: PropertyValue> {
    collection: ObservableCollectionExtended<T>,
}

impl<T: PropertyValue> ObservableCollectionItems<T> {
    /// The items of `collection`.
    pub fn new(collection: ObservableCollectionExtended<T>) -> Self {
        Self { collection }
    }

    /// The items of `collection` as the value of an items source property.
    pub fn items_source(collection: ObservableCollectionExtended<T>) -> ItemsSource {
        ItemsSource::new(Rc::new(Self::new(collection)))
    }

    /// The collection.
    pub fn collection(&self) -> &ObservableCollectionExtended<T> {
        &self.collection
    }
}

impl<T: PropertyValue> IItemsList for ObservableCollectionItems<T> {
    fn count(&self) -> usize {
        self.collection.count()
    }

    fn get_at(&self, index: usize) -> Option<BoxedValue> {
        box_item(&self.collection.get(index))
    }

    fn is_notifying(&self) -> bool {
        true
    }

    fn add_collection_changed(&self, handler: Rc<ItemsChangedHandler>) -> Option<u64> {
        Some(self.collection.add_collection_changed(Rc::new(move |e: &NotifyCollectionChangedEventArgs<'_, T>| {
            let new_items = TypedItems(e.new_items);
            let old_items = TypedItems(e.old_items);
            handler(&ItemsChangedEventArgs {
                action: e.action,
                new_items: ItemsView::from_typed(&new_items),
                old_items: ItemsView::from_typed(&old_items),
                new_starting_index: e.new_starting_index,
                old_starting_index: e.old_starting_index,
            });
        })))
    }

    fn remove_collection_changed(&self, token: u64) {
        let _ = self.collection.remove_collection_changed(token);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
