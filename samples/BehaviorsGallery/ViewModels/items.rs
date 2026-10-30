//! Not from the reference: the collections of the view models of the gallery.
//!
//! The view models of the reference hold their items in `ObservableCollection<T>`.
//!
//! Deviation (DEVIATIONS.md, Behaviors gallery): a collection of view models is a notifying list
//! of untyped values ([`ItemList`], a `FerroList<BoxedValue>` whose items are the view models), the
//! list the items controls read and the item actions, the drop handlers and the drag behaviors of
//! the behavior crates change (they change lists of untyped values, not lists of a type of the
//! gallery). The helpers of this module read and write it with the type of its items.

use ferroui_base::collections::FerroList;
use ferroui_base::BoxedValue;
use ferroui_controls::ItemsSource;
use std::any::Any;
use std::rc::Rc;

/// A notifying list of view models (`ObservableCollection<T>`), as untyped values.
pub type ItemList = Rc<FerroList<BoxedValue>>;

/// A list of the items (`new ObservableCollection<T> { .. }`).
pub fn item_list<T: PartialEq + 'static>(items: impl IntoIterator<Item = Rc<T>>) -> ItemList {
    Rc::new(FerroList::from_items(
        items.into_iter().map(|item| item as BoxedValue),
    ))
}

/// A list of texts (`ObservableCollection<string>`), as untyped values.
pub fn text_list<'a>(items: impl IntoIterator<Item = &'a str>) -> ItemList {
    Rc::new(FerroList::from_items(
        items
            .into_iter()
            .map(|item| Rc::new(item.to_string()) as BoxedValue),
    ))
}

/// The value as the view model it holds.
pub fn as_item<T: 'static>(value: &BoxedValue) -> Option<Rc<T>> {
    let any: Rc<dyn Any> = value.clone();
    any.downcast::<T>().ok()
}

/// The view model as an untyped value.
pub fn boxed<T: PartialEq + 'static>(item: &Rc<T>) -> BoxedValue {
    item.clone() as BoxedValue
}

/// The items of `list` that are of type `T`.
pub fn items_of<T: 'static>(list: &FerroList<BoxedValue>) -> Vec<Rc<T>> {
    list.to_vec().iter().filter_map(as_item::<T>).collect()
}

/// The index of `item` in `list` (`IndexOf`), by identity.
pub fn index_of<T: 'static>(list: &FerroList<BoxedValue>, item: &Rc<T>) -> Option<usize> {
    list.to_vec()
        .iter()
        .position(|value| as_item::<T>(value).is_some_and(|value| Rc::ptr_eq(&value, item)))
}

/// The list as the items of an items control.
pub fn items_source(list: &Option<ItemList>) -> Option<ItemsSource> {
    list.as_ref().map(|list| ItemsSource::from(list.clone()))
}
