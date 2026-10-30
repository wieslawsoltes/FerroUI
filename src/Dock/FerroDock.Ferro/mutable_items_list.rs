//! Original mutable IList access at the native read-only ItemsSource boundary.
use ferrodock_model::collections::{IList, ListHandle};
use ferroui_base::collections::FerroList;
use ferroui_base::{BoxedValue, PropertyValue};
use ferroui_controls::IItemsList;
use std::rc::Rc;
/// Creates an original mutable list handle over the same shared native list.
/// Register a concrete shared ValueTypes cast from FerroList<T> to this handle
/// in the UI composition root when custom item types need untyped reorder access.
/// No items or notifications are copied, and no type registration occurs here.
pub fn from_ferro_list<T: PropertyValue>(list: FerroList<T>) -> ListHandle<Option<BoxedValue>> {
    ListHandle::from_list(Rc::new(NativeList(list)))
}
struct NativeList<T: PropertyValue>(FerroList<T>);
impl<T: PropertyValue> IList<Option<BoxedValue>> for NativeList<T> {
    fn count(&self) -> usize {
        self.0.count()
    }
    fn get(&self, index: usize) -> Option<BoxedValue> {
        IItemsList::get_at(&self.0, index)
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
        let index = IItemsList::index_of(&self.0, value);
        usize::try_from(index).ok()
    }
    fn reference_equals(&self, other: &dyn IList<Option<BoxedValue>>) -> bool {
        (other as &dyn std::any::Any)
            .downcast_ref::<Self>()
            .is_some_and(|other| self.0 == other.0)
    }
}
pub(crate) fn register_types() {
    use ferroui_base::data::core::ValueTypes;
    ValueTypes::register_cast::<FerroList<String>, ListHandle<Option<BoxedValue>>>(|list| {
        from_ferro_list(list.clone())
    });
    ValueTypes::register_cast::<FerroList<BoxedValue>, ListHandle<Option<BoxedValue>>>(|list| {
        from_ferro_list(list.clone())
    });
    ValueTypes::register_cast::<FerroList<Option<BoxedValue>>, ListHandle<Option<BoxedValue>>>(
        |list| from_ferro_list(list.clone()),
    );
}
