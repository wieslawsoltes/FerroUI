//! The untyped collection contract of the items controls.
//!
//! The managed framework types `ItemsSource`, selection sources and the
//! like as "any enumerable" and discovers at run time whether the object is
//! an indexable list and whether it notifies of changes. Here that contract
//! is explicit:
//!
//! - an item is an `Option<BoxedValue>` (an untyped, nullable value);
//! - [`IItemsList`] is the untyped, indexable, optionally notifying list;
//! - [`ItemsSource`] is the shared handle stored in properties. It compares
//!   by the identity of the collection, so assigning the same collection
//!   twice is not a change.
//!
//! [`IItemsList`] is implemented for the collection abstractions of the
//! port, so each of them converts into an [`ItemsSource`] without copying:
//!
//! | Collection | Notifying | Items |
//! |---|---|---|
//! | `Rc<FerroList<T>>` | yes | each `T` boxed on access (no allocation when `T` is `BoxedValue` or `Option<BoxedValue>`) |
//! | `Rc<BindableList<T>>` | yes | as its inner `FerroList<T>` |
//! | `Vec<Option<BoxedValue>>`, `Vec<BoxedValue>` | no | as stored |
//! | any iterator of property values ([`ItemsSource::from_values`]) | no | boxed once, up front |
//!
//! A source that only enumerates is copied into a list when it is assigned,
//! exactly as the managed implementation copies a non-list enumerable.
//!
//! # Change notifications
//!
//! A notifying list reports changes with [`ItemsChangedEventArgs`], the
//! untyped counterpart of the typed `NotifyCollectionChangedEventArgs` of
//! `FerroList<T>`. The managed framework hands the changed list itself to
//! the handlers, which mostly ask it for its count only. Likewise
//! `new_items` and `old_items` here are an [`ItemsView`]: a `Copy` view,
//! two or three words wide, whose `len()` never reads an item and whose
//! `get(i)`/`iter()`/`to_vec()` read exactly the items asked for. A view is
//! one of:
//!
//! - a borrowed slice of untyped items ([`ItemsView::from_slice`]);
//! - a window of a list, read through [`IItemsList::get_at`] on demand
//!   ([`ItemsView::window`], [`ItemsView::of_source`]): replacing the source
//!   of an items control notifies with windows over the old and the new
//!   source, so a data-virtualised source is never enumerated;
//! - the typed items of a `FerroList<T>` change ([`ItemsView::from_typed`]),
//!   boxed one by one when read: adapting a typed notification allocates
//!   nothing, whatever the number of items.
//!
//! Handlers on virtualizing paths use the counts only. A handler that keeps
//! items beyond the notification copies them with `to_vec()`.
//!
//! Controls are items too: a control item is boxed as exactly `Ref<Control>`
//! (see `Control::boxed`).
//!
//! Bindings deliver untyped values of the view model's property type. To
//! bind a view model property holding one of the collection types above to
//! an items source property, register the conversion once with
//! [`ItemsSource::register_binding_conversion`].

use ferroui_base::collections::{FerroList, NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::data::model::BindableList;
use ferroui_base::{AnyValue, BoxedValue, PropertyValue};
use std::any::Any;
use std::rc::Rc;

/// The items carried by a change notification of an untyped items list: a
/// cheap, copyable view that produces an item only when asked for it.
///
/// See the module documentation ("Change notifications").
#[derive(Clone, Copy)]
pub struct ItemsView<'a>(ItemsViewSource<'a>);

#[derive(Clone, Copy)]
enum ItemsViewSource<'a> {
    /// Items that already exist as untyped values.
    Slice(&'a [Option<BoxedValue>]),
    /// `count` items of `list` starting at `start`, read on demand.
    Window { list: &'a dyn IItemsList, start: usize, count: usize },
    /// The items of a typed collection, boxed on demand.
    Typed(&'a dyn TypedItemsSource),
}

/// The typed items of a change, viewed as untyped items by
/// [`ItemsView::from_typed`]. Each item is boxed when it is read (no
/// allocation when `T` is `BoxedValue` or `Option<BoxedValue>`).
pub struct TypedItems<'a, T>(pub &'a [T]);

trait TypedItemsSource {
    fn len(&self) -> usize;
    fn get(&self, index: usize) -> Option<BoxedValue>;
}

impl<T: PropertyValue> TypedItemsSource for TypedItems<'_, T> {
    #[inline]
    fn len(&self) -> usize {
        self.0.len()
    }

    #[inline]
    fn get(&self, index: usize) -> Option<BoxedValue> {
        box_item(&self.0[index])
    }
}

impl<'a> ItemsView<'a> {
    /// A view of no items.
    pub const EMPTY: ItemsView<'static> = ItemsView(ItemsViewSource::Slice(&[]));

    /// A view of existing untyped items.
    #[inline]
    pub const fn from_slice(items: &'a [Option<BoxedValue>]) -> Self {
        Self(ItemsViewSource::Slice(items))
    }

    /// A view of `count` items of `list` starting at `start`. Nothing is
    /// read from the list until an item is asked for.
    #[inline]
    pub fn window(list: &'a dyn IItemsList, start: usize, count: usize) -> Self {
        Self(ItemsViewSource::Window { list, start, count })
    }

    /// A view of all the items of `source`. Only the count is read.
    #[inline]
    pub fn of_source(source: &'a ItemsSource) -> Self {
        Self::window(&**source.list(), 0, source.count())
    }

    /// A view of `count` items of `source` starting at `start`.
    #[inline]
    pub fn of_source_range(source: &'a ItemsSource, start: usize, count: usize) -> Self {
        Self::window(&**source.list(), start, count)
    }

    /// A view of typed items, each boxed when it is read.
    #[inline]
    pub fn from_typed<T: PropertyValue>(items: &'a TypedItems<'a, T>) -> Self {
        Self(ItemsViewSource::Typed(items))
    }

    /// The number of items. Never reads an item.
    #[inline]
    pub fn len(&self) -> usize {
        match self.0 {
            ItemsViewSource::Slice(items) => items.len(),
            ItemsViewSource::Window { count, .. } => count,
            ItemsViewSource::Typed(items) => items.len(),
        }
    }

    /// Whether the view has no items. Never reads an item.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The item at `index`. Panics if out of range.
    #[inline]
    pub fn get(&self, index: usize) -> Option<BoxedValue> {
        match self.0 {
            ItemsViewSource::Slice(items) => items[index].clone(),
            ItemsViewSource::Window { list, start, count } => {
                assert!(index < count, "index out of range");
                list.get_at(start + index)
            }
            ItemsViewSource::Typed(items) => items.get(index),
        }
    }

    /// The items, each read when the iterator reaches it.
    #[inline]
    pub fn iter(&self) -> ItemsViewIter<'a> {
        ItemsViewIter { view: *self, index: 0, end: self.len() }
    }

    /// A copy of the items. Reads every item.
    pub fn to_vec(&self) -> Vec<Option<BoxedValue>> {
        match self.0 {
            ItemsViewSource::Slice(items) => items.to_vec(),
            _ => self.iter().collect(),
        }
    }
}

impl Default for ItemsView<'_> {
    fn default() -> Self {
        ItemsView::EMPTY
    }
}

impl std::fmt::Debug for ItemsView<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ItemsView(len = {})", self.len())
    }
}

impl<'a> From<&'a [Option<BoxedValue>]> for ItemsView<'a> {
    #[inline]
    fn from(items: &'a [Option<BoxedValue>]) -> Self {
        Self::from_slice(items)
    }
}

impl<'a> From<&'a Vec<Option<BoxedValue>>> for ItemsView<'a> {
    #[inline]
    fn from(items: &'a Vec<Option<BoxedValue>>) -> Self {
        Self::from_slice(items)
    }
}

impl<'a, const N: usize> From<&'a [Option<BoxedValue>; N]> for ItemsView<'a> {
    #[inline]
    fn from(items: &'a [Option<BoxedValue>; N]) -> Self {
        Self::from_slice(items)
    }
}

impl<'a> From<&'a ItemsSource> for ItemsView<'a> {
    #[inline]
    fn from(source: &'a ItemsSource) -> Self {
        Self::of_source(source)
    }
}

/// The iterator of an [`ItemsView`].
#[derive(Clone)]
pub struct ItemsViewIter<'a> {
    view: ItemsView<'a>,
    index: usize,
    end: usize,
}

impl Iterator for ItemsViewIter<'_> {
    type Item = Option<BoxedValue>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.end {
            self.index += 1;
            Some(self.view.get(self.index - 1))
        } else {
            None
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.end - self.index;
        (remaining, Some(remaining))
    }
}

impl DoubleEndedIterator for ItemsViewIter<'_> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.index < self.end {
            self.end -= 1;
            Some(self.view.get(self.end))
        } else {
            None
        }
    }
}

impl ExactSizeIterator for ItemsViewIter<'_> {}

impl<'a> IntoIterator for ItemsView<'a> {
    type Item = Option<BoxedValue>;
    type IntoIter = ItemsViewIter<'a>;

    #[inline]
    fn into_iter(self) -> ItemsViewIter<'a> {
        self.iter()
    }
}

impl<'a> IntoIterator for &ItemsView<'a> {
    type Item = Option<BoxedValue>;
    type IntoIter = ItemsViewIter<'a>;

    #[inline]
    fn into_iter(self) -> ItemsViewIter<'a> {
        self.iter()
    }
}

/// A change to an untyped items list.
#[derive(Clone, Copy, Debug)]
pub struct ItemsChangedEventArgs<'a> {
    /// The action that caused the notification.
    pub action: NotifyCollectionChangedAction,
    /// The items involved in the change (added, or the replacements).
    pub new_items: ItemsView<'a>,
    /// The items affected by a replace, remove or move.
    pub old_items: ItemsView<'a>,
    /// The index at which the change occurred, or `-1`.
    pub new_starting_index: i32,
    /// The index at which a move, remove or replace occurred, or `-1`.
    pub old_starting_index: i32,
}

impl ItemsChangedEventArgs<'static> {
    /// The arguments of a reset notification.
    pub const RESET: ItemsChangedEventArgs<'static> = ItemsChangedEventArgs {
        action: NotifyCollectionChangedAction::Reset,
        new_items: ItemsView::EMPTY,
        old_items: ItemsView::EMPTY,
        new_starting_index: -1,
        old_starting_index: -1,
    };
}

impl<'a> ItemsChangedEventArgs<'a> {
    /// The arguments of an add of `new_items` at `new_starting_index`.
    #[inline]
    pub fn add(new_items: impl Into<ItemsView<'a>>, new_starting_index: i32) -> Self {
        Self {
            action: NotifyCollectionChangedAction::Add,
            new_items: new_items.into(),
            old_items: ItemsView::EMPTY,
            new_starting_index,
            old_starting_index: -1,
        }
    }

    /// The arguments of a remove of `old_items` from `old_starting_index`.
    #[inline]
    pub fn remove(old_items: impl Into<ItemsView<'a>>, old_starting_index: i32) -> Self {
        Self {
            action: NotifyCollectionChangedAction::Remove,
            new_items: ItemsView::EMPTY,
            old_items: old_items.into(),
            new_starting_index: -1,
            old_starting_index,
        }
    }
}

/// The change of a list of untyped items, as it is: the items are viewed in
/// place.
impl<'a> From<&NotifyCollectionChangedEventArgs<'a, Option<BoxedValue>>> for ItemsChangedEventArgs<'a> {
    #[inline]
    fn from(e: &NotifyCollectionChangedEventArgs<'a, Option<BoxedValue>>) -> Self {
        Self {
            action: e.action,
            new_items: ItemsView::from_slice(e.new_items),
            old_items: ItemsView::from_slice(e.old_items),
            new_starting_index: e.new_starting_index,
            old_starting_index: e.old_starting_index,
        }
    }
}

/// The signature of a handler of changes to an untyped items list.
pub type ItemsChangedHandler = dyn for<'a> Fn(&ItemsChangedEventArgs<'a>);

/// Compares two items: both null, or both of the same type and equal.
#[inline]
pub fn items_equal(a: &Option<BoxedValue>, b: &Option<BoxedValue>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => Rc::ptr_eq(a, b) || (**a).any_value_eq(&**b),
        _ => false,
    }
}

/// Whether two untyped values are the same object: C# `ReferenceEquals` (and
/// `==` / `!=` on `object`) for values that travel in boxes.
///
/// The rule:
/// * both null, or the same box: the same reference;
/// * boxes that hold handles of the same object of the class hierarchy (a
///   control boxed twice, or boxed once as `Ref<Control>` and once as
///   `Ref<Border>`): the same reference;
/// * boxes that both hold a string: the same reference if the strings are
///   equal. A string is the immutable value-like type that the lists of the
///   managed original hand out as the same instance on every read, whereas a
///   typed list of strings here boxes each string anew on every read; without
///   this, an unchanged item read twice would look like another object;
/// * anything else in two boxes (two separately boxed equal numbers, two
///   equal structures) is two references, as two boxings are in the managed
///   original.
///
/// Use [`items_equal`] where the original calls `Equals`.
pub fn reference_equals(a: &Option<BoxedValue>, b: &Option<BoxedValue>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => boxed_reference_equals(a, b),
        _ => false,
    }
}

/// [`reference_equals`] for two values that are not null.
pub fn boxed_reference_equals(a: &BoxedValue, b: &BoxedValue) -> bool {
    // The addresses of the boxes; the comparison of the handles themselves
    // would also compare their metadata.
    if std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b)) {
        return true;
    }

    fn as_str(value: &dyn AnyValue) -> Option<&str> {
        if let Some(text) = value.downcast_ref::<String>() {
            return Some(text.as_str());
        }
        value.downcast_ref::<&'static str>().copied()
    }

    let (a, b): (&dyn AnyValue, &dyn AnyValue) = (&**a, &**b);
    if let (Some(a), Some(b)) = (as_str(a), as_str(b)) {
        return a == b;
    }

    match (ValueTypes::as_object(a), ValueTypes::as_object(b)) {
        (Some(a), Some(b)) => a.ptr_eq(&b),
        _ => false,
    }
}

/// Boxes a typed value as an item. A value that already is an untyped value
/// is passed through.
#[inline]
pub fn box_item<T: PropertyValue>(value: &T) -> Option<BoxedValue> {
    let any: &dyn Any = value;
    if let Some(boxed) = any.downcast_ref::<BoxedValue>() {
        return Some(boxed.clone());
    }
    if let Some(boxed) = any.downcast_ref::<Option<BoxedValue>>() {
        return boxed.clone();
    }
    Some(Rc::new(value.clone()))
}

/// The typed value held by an item: the item itself when `T` is the untyped
/// value type, otherwise the contained `T`. `None` for a null item or an
/// item of another type.
#[inline]
pub fn unbox_item<T: PropertyValue>(item: &Option<BoxedValue>) -> Option<T> {
    let boxed = item.as_ref()?;
    let any: &dyn Any = boxed;
    if let Some(value) = any.downcast_ref::<T>() {
        // `T` is `BoxedValue`.
        return Some(value.clone());
    }
    let value: &dyn AnyValue = &**boxed;
    value.downcast_ref::<T>().cloned()
}

/// Whether an item holds a value equal to `value`, without boxing `value`.
#[inline]
fn item_equals_typed<T: PropertyValue>(item: &Option<BoxedValue>, value: &T) -> bool {
    let any: &dyn Any = value;
    if let Some(boxed) = any.downcast_ref::<BoxedValue>() {
        return item.as_ref().is_some_and(|item| Rc::ptr_eq(item, boxed) || (**item).any_value_eq(&**boxed));
    }
    if let Some(boxed) = any.downcast_ref::<Option<BoxedValue>>() {
        return items_equal(item, boxed);
    }
    match item {
        Some(item) => {
            let item: &dyn AnyValue = &**item;
            item.downcast_ref::<T>().is_some_and(|item| item == value)
        }
        None => false,
    }
}

/// An untyped, indexable list of items that may notify of changes.
pub trait IItemsList {
    /// The number of items.
    fn count(&self) -> usize;

    /// The item at `index`. Panics if out of range.
    fn get_at(&self, index: usize) -> Option<BoxedValue>;

    /// The index of the first item equal to `item`, or `-1`.
    fn index_of(&self, item: &Option<BoxedValue>) -> i32 {
        for i in 0..self.count() {
            if items_equal(&self.get_at(i), item) {
                return i as i32;
            }
        }
        -1
    }

    /// Whether the list notifies of changes.
    fn is_notifying(&self) -> bool {
        false
    }

    /// Subscribes to changes. Returns the token for
    /// [`remove_collection_changed`](Self::remove_collection_changed), or
    /// `None` if the list does not notify.
    fn add_collection_changed(&self, _handler: Rc<ItemsChangedHandler>) -> Option<u64> {
        None
    }

    /// Unsubscribes a handler.
    fn remove_collection_changed(&self, _token: u64) {}

    /// The collection as [`Any`], to get back to the typed collection.
    fn as_any(&self) -> &dyn Any;
}

impl IItemsList for Vec<Option<BoxedValue>> {
    fn count(&self) -> usize {
        self.len()
    }

    fn get_at(&self, index: usize) -> Option<BoxedValue> {
        self[index].clone()
    }

    fn index_of(&self, item: &Option<BoxedValue>) -> i32 {
        self.iter().position(|i| items_equal(i, item)).map_or(-1, |i| i as i32)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl<T: PropertyValue> IItemsList for FerroList<T> {
    fn count(&self) -> usize {
        FerroList::count(self)
    }

    fn get_at(&self, index: usize) -> Option<BoxedValue> {
        let snapshot = self.snapshot();
        box_item(&snapshot[index])
    }

    fn index_of(&self, item: &Option<BoxedValue>) -> i32 {
        let snapshot = self.snapshot();
        snapshot.iter().position(|i| item_equals_typed(item, i)).map_or(-1, |i| i as i32)
    }

    fn is_notifying(&self) -> bool {
        true
    }

    fn add_collection_changed(&self, handler: Rc<ItemsChangedHandler>) -> Option<u64> {
        Some(FerroList::add_collection_changed(
            self,
            Rc::new(move |e: &NotifyCollectionChangedEventArgs<'_, T>| {
                // Nothing is boxed or copied here: the handler boxes the
                // items it reads.
                let new_items = TypedItems(e.new_items);
                let old_items = TypedItems(e.old_items);
                handler(&ItemsChangedEventArgs {
                    action: e.action,
                    new_items: ItemsView::from_typed(&new_items),
                    old_items: ItemsView::from_typed(&old_items),
                    new_starting_index: e.new_starting_index,
                    old_starting_index: e.old_starting_index,
                });
            }),
        ))
    }

    fn remove_collection_changed(&self, token: u64) {
        FerroList::remove_collection_changed(self, token);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl<T: PropertyValue> IItemsList for BindableList<T> {
    fn count(&self) -> usize {
        self.items().count()
    }

    fn get_at(&self, index: usize) -> Option<BoxedValue> {
        IItemsList::get_at(self.items(), index)
    }

    fn index_of(&self, item: &Option<BoxedValue>) -> i32 {
        IItemsList::index_of(self.items(), item)
    }

    fn is_notifying(&self) -> bool {
        true
    }

    fn add_collection_changed(&self, handler: Rc<ItemsChangedHandler>) -> Option<u64> {
        IItemsList::add_collection_changed(self.items(), handler)
    }

    fn remove_collection_changed(&self, token: u64) {
        IItemsList::remove_collection_changed(self.items(), token);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// A shared handle to an untyped items list: the value of an items source
/// property.
///
/// Equality is the identity of the underlying collection.
#[derive(Clone)]
pub struct ItemsSource(Rc<dyn IItemsList>);

impl ItemsSource {
    /// Wraps a list.
    pub fn new(list: Rc<dyn IItemsList>) -> Self {
        Self(list)
    }

    /// Creates a non-notifying source holding the given items.
    pub fn from_items(items: impl IntoIterator<Item = Option<BoxedValue>>) -> Self {
        Self(Rc::new(items.into_iter().collect::<Vec<_>>()))
    }

    /// Creates a non-notifying source by boxing each of the given values.
    pub fn from_values<T: PropertyValue>(values: impl IntoIterator<Item = T>) -> Self {
        Self(Rc::new(values.into_iter().map(|v| box_item(&v)).collect::<Vec<_>>()))
    }

    /// Creates a non-notifying source of strings.
    pub fn from_strs<'a>(values: impl IntoIterator<Item = &'a str>) -> Self {
        Self::from_values(values.into_iter().map(str::to_string))
    }

    /// The list.
    #[inline]
    pub fn list(&self) -> &Rc<dyn IItemsList> {
        &self.0
    }

    /// The number of items.
    #[inline]
    pub fn count(&self) -> usize {
        self.0.count()
    }

    /// The item at `index`. Panics if out of range.
    #[inline]
    pub fn get_at(&self, index: usize) -> Option<BoxedValue> {
        self.0.get_at(index)
    }

    /// The index of the first item equal to `item`, or `-1`.
    #[inline]
    pub fn index_of(&self, item: &Option<BoxedValue>) -> i32 {
        self.0.index_of(item)
    }

    /// Whether the list contains an item equal to `item`.
    #[inline]
    pub fn contains(&self, item: &Option<BoxedValue>) -> bool {
        self.0.index_of(item) >= 0
    }

    /// Whether the list notifies of changes.
    #[inline]
    pub fn is_notifying(&self) -> bool {
        self.0.is_notifying()
    }

    /// The typed collection behind the source, if it is a `T`.
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.0.as_any().downcast_ref::<T>()
    }

    /// The identity of the underlying collection.
    #[inline]
    pub fn identity(&self) -> usize {
        Rc::as_ptr(&self.0) as *const () as usize
    }

    /// Whether two handles refer to the same collection.
    #[inline]
    pub fn ptr_eq(&self, other: &ItemsSource) -> bool {
        self.identity() == other.identity()
    }

    /// The items, enumerated by index over the live list.
    pub fn iter(&self) -> impl Iterator<Item = Option<BoxedValue>> + '_ {
        let mut index = 0;
        std::iter::from_fn(move || {
            if index < self.0.count() {
                index += 1;
                Some(self.0.get_at(index - 1))
            } else {
                None
            }
        })
    }

    /// A copy of the items.
    pub fn to_vec(&self) -> Vec<Option<BoxedValue>> {
        self.iter().collect()
    }

    /// Lets bindings deliver a value of the collection type `S` to a
    /// property of type `Option<ItemsSource>` (or `ItemsSource`) on the
    /// current thread.
    pub fn register_binding_conversion<S: Clone + Into<ItemsSource> + 'static>() {
        ValueTypes::register_conversion::<S, ItemsSource>(|s| Some(s.clone().into()));
        ValueTypes::register_conversion::<S, Option<ItemsSource>>(|s| Some(Some(s.clone().into())));
    }
}

impl PartialEq for ItemsSource {
    fn eq(&self, other: &Self) -> bool {
        self.ptr_eq(other)
    }
}

impl std::fmt::Debug for ItemsSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ItemsSource(count = {})", self.count())
    }
}

impl From<Rc<dyn IItemsList>> for ItemsSource {
    fn from(value: Rc<dyn IItemsList>) -> Self {
        Self(value)
    }
}

impl<T: PropertyValue> From<Rc<FerroList<T>>> for ItemsSource {
    fn from(value: Rc<FerroList<T>>) -> Self {
        Self(value)
    }
}

impl<T: PropertyValue> From<Rc<BindableList<T>>> for ItemsSource {
    fn from(value: Rc<BindableList<T>>) -> Self {
        Self(value)
    }
}

impl From<Vec<Option<BoxedValue>>> for ItemsSource {
    fn from(value: Vec<Option<BoxedValue>>) -> Self {
        Self(Rc::new(value))
    }
}

impl From<Vec<BoxedValue>> for ItemsSource {
    fn from(value: Vec<BoxedValue>) -> Self {
        Self::from_items(value.into_iter().map(Some))
    }
}

impl From<Rc<Vec<Option<BoxedValue>>>> for ItemsSource {
    fn from(value: Rc<Vec<Option<BoxedValue>>>) -> Self {
        Self(value)
    }
}

#[cfg(test)]
mod reference_equals_tests {
    use super::{boxed_reference_equals, items_equal, reference_equals};
    use crate::{Border, Control};
    use ferroui_base::{BoxedValue, Ref, StyledElement};
    use std::rc::Rc;

    fn boxed<T: PartialEq + 'static>(value: T) -> Option<BoxedValue> {
        Some(Rc::new(value))
    }

    #[test]
    fn nulls_and_the_same_box_are_the_same_reference() {
        let value = boxed(42_i32);

        assert!(reference_equals(&None, &None));
        assert!(reference_equals(&value, &value.clone()));
        assert!(!reference_equals(&value, &None));
        assert!(!reference_equals(&None, &value));
    }

    #[test]
    fn equal_strings_in_different_boxes_are_the_same_reference() {
        assert!(reference_equals(&boxed("able".to_string()), &boxed("able".to_string())));
        assert!(reference_equals(&boxed("able".to_string()), &boxed("able")));
        assert!(reference_equals(&boxed(String::new()), &boxed(String::new())));
        assert!(!reference_equals(&boxed("able".to_string()), &boxed("abide".to_string())));
        assert!(!reference_equals(&boxed("able".to_string()), &boxed("Able".to_string())));
        assert!(!reference_equals(&boxed("1".to_string()), &boxed(1_i32)));
    }

    #[test]
    fn equal_values_in_different_boxes_are_different_references() {
        assert!(!reference_equals(&boxed(42_i32), &boxed(42_i32)));
        assert!(!reference_equals(&boxed(1.5_f64), &boxed(1.5_f64)));
        assert!(!reference_equals(&boxed(true), &boxed(true)));
        // They are equal values all the same.
        assert!(items_equal(&boxed(42_i32), &boxed(42_i32)));
    }

    #[test]
    fn the_same_control_in_different_boxes_is_the_same_reference() {
        let border = Border::new();
        let as_control: Ref<Control> = border.clone().upcast();
        let as_element: Ref<StyledElement> = border.clone().upcast();

        assert!(reference_equals(&Some(Control::boxed(border.clone())), &Some(Control::boxed(border.clone()))));
        // Whatever the type of the handle in the box is.
        assert!(reference_equals(&boxed(border.clone()), &boxed(as_control)));
        assert!(reference_equals(&boxed(as_element), &boxed(border.clone())));
        assert!(boxed_reference_equals(&Control::boxed(border.clone()), &Control::boxed(border)));
    }

    #[test]
    fn different_controls_are_different_references() {
        let (first, second) = (Border::new(), Border::new());

        assert!(!reference_equals(&Some(Control::boxed(first.clone())), &Some(Control::boxed(second))));
        assert!(!reference_equals(&Some(Control::boxed(first)), &boxed("able".to_string())));
    }
}
