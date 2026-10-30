//! Bridges live source read-only request collections to the native items contract.
use ferrodock_model::collections::ReadOnlyObservableCollection;
use ferrodock_model::services::{ConfirmationRequest, DialogRequest};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::{BoxedValue, PropertyValue};
use ferroui_controls::{
    IItemsList, ItemsChangedEventArgs, ItemsChangedHandler, ItemsSource, ItemsView, TypedItems,
};
use std::any::Any;
use std::rc::Rc;

struct RequestItems<T: PropertyValue>(ReadOnlyObservableCollection<T>);
impl<T: PropertyValue> IItemsList for RequestItems<T> {
    fn count(&self) -> usize {
        self.0.count()
    }
    fn get_at(&self, index: usize) -> Option<BoxedValue> {
        Some(Rc::new(self.0.get(index)))
    }
    fn is_notifying(&self) -> bool {
        true
    }
    fn add_collection_changed(&self, handler: Rc<ItemsChangedHandler>) -> Option<u64> {
        Some(self.0.add_collection_changed(Rc::new(move |e| {
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
        self.0.remove_collection_changed(token);
    }
    fn identity(&self) -> Option<usize> {
        Some(self.0.identity())
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
fn items<T: PropertyValue>(value: &ReadOnlyObservableCollection<T>) -> ItemsSource {
    ItemsSource::new(Rc::new(RequestItems(value.clone())))
}
pub(crate) fn register_types() {
    ValueTypes::register_nullable::<ReadOnlyObservableCollection<Rc<DialogRequest>>>();
    ValueTypes::register_nullable::<ReadOnlyObservableCollection<Rc<ConfirmationRequest>>>();
    ItemsSource::register_binding_conversion_with::<ReadOnlyObservableCollection<Rc<DialogRequest>>>(
        items,
    );
    ItemsSource::register_binding_conversion_with::<
        ReadOnlyObservableCollection<Rc<ConfirmationRequest>>,
    >(items);
    ValueTypes::register_conversion::<
        Option<ReadOnlyObservableCollection<Rc<DialogRequest>>>,
        Option<ItemsSource>,
    >(|v| Some(v.as_ref().map(items)));
    ValueTypes::register_conversion::<
        Option<ReadOnlyObservableCollection<Rc<ConfirmationRequest>>>,
        Option<ItemsSource>,
    >(|v| Some(v.as_ref().map(items)));
}
