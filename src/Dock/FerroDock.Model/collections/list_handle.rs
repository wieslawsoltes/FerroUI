// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use ferroui_system::collections::object_model::ObservableCollection;
use ferroui_system::collections::specialized::INotifyCollectionChanged;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::marker::PhantomData;
use std::rc::Rc;

/// Equality used by a plain list when its element representation has no PartialEq.
pub type ListComparer<T> = Rc<dyn Fn(&T, &T) -> bool>;

/// The mutable list contract used by source IList properties and factory methods.
/// Custom lists retain their own mutation, read-only, enumeration and notification behavior.
pub trait IList<T: Clone + 'static>: Any {
    /// Number of elements (`Count`).
    fn count(&self) -> usize;
    /// Whether the collection is read-only (`IsReadOnly`).
    fn is_read_only(&self) -> bool {
        false
    }
    /// Gets an element by index.
    fn get(&self, index: usize) -> T;
    /// Replaces an element by index.
    fn set(&self, index: usize, item: T);
    /// Inserts an element at the supplied index.
    fn insert(&self, index: usize, item: T);
    /// Removes an element at the supplied index.
    fn remove_at(&self, index: usize);
    /// Removes all elements.
    fn clear(&self);
    /// Finds the first matching element using this list's element comparer.
    fn index_of(&self, item: &T) -> Option<usize>;
    /// Appends an element.
    fn add(&self, item: T) {
        self.insert(self.count(), item);
    }
    /// Tests whether an element is present.
    fn contains(&self, item: &T) -> bool {
        self.index_of(item).is_some()
    }
    /// Removes the first matching element, returning whether it was present.
    fn remove(&self, item: &T) -> bool {
        if let Some(index) = self.index_of(item) {
            self.remove_at(index);
            true
        } else {
            false
        }
    }
    /// Copies the elements into an existing destination buffer.
    fn copy_to(&self, destination: &mut [T], index: usize) {
        let count = self.count();
        assert!(
            index <= destination.len() && count <= destination.len() - index,
            "destination does not have room for the list"
        );
        for offset in 0..count {
            destination[index + offset] = self.get(offset);
        }
    }
    /// Queries optional collection notifications without imposing them on plain lists.
    fn as_notify_collection_changed(&self) -> Option<&dyn INotifyCollectionChanged> {
        None
    }
    /// Queries original property notifications (Count and Item[]) when supported.
    fn as_notify_property_changed(
        &self,
    ) -> Option<&dyn ferroui_system::component_model::INotifyPropertyChanged> {
        None
    }
    /// Whether the original notifying collection exposes its typed change contents.
    /// Counts-only notifications cannot reconstruct removed or replaced source items.
    fn has_typed_collection_changes(&self) -> bool {
        false
    }
    /// Subscribes directly to original typed changes without maintaining a mirror list.
    fn add_collection_changed(&self, _handler: Rc<super::ReadOnlyListChanged<T>>) -> Option<u64> {
        None
    }
    /// Removes an original typed change subscription.
    fn remove_collection_changed(&self, _token: u64) {}
    /// Stable identity of the original collection, including shared backend adapters.
    fn identity(&self) -> usize {
        self as *const Self as *const () as usize
    }
    /// Returns the wrapped shared FerroUI collection, when this is its adapter.
    fn as_observable_collection(&self) -> Option<ObservableCollection<T>>
    where
        T: PartialEq,
    {
        None
    }
    /// Compares list object identity, not element values.
    fn reference_equals(&self, other: &dyn IList<T>) -> bool {
        std::ptr::eq(
            self as *const Self as *const (),
            other as *const dyn IList<T> as *const (),
        )
    }
    /// Mutation version for fail-fast enumeration, when supplied by the backend.
    fn version(&self) -> Option<u32> {
        None
    }
    /// Gets an independent enumerator. Custom lists may override the source enumeration contract.
    fn get_enumerator(self: Rc<Self>) -> Box<dyn Iterator<Item = T>> {
        let version = self.version();
        Box::new(ListEnumerator {
            list: self,
            index: 0,
            version,
            item: PhantomData,
        })
    }
}

/// A shared IList reference. Cloning keeps list identity and live mutations.
pub struct ListHandle<T: Clone + 'static> {
    inner: Rc<dyn IList<T>>,
}
impl<T: Clone + 'static> Clone for ListHandle<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}
impl<T: Clone + 'static> PartialEq for ListHandle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.inner.reference_equals(other.inner.as_ref())
    }
}
impl<T: Clone + 'static> ListHandle<T> {
    /// Creates a plain list using an explicit element comparer.
    pub fn from_plain_with_comparer(items: Vec<T>, comparer: ListComparer<T>) -> Self {
        Self::from_list(Rc::new(PlainList {
            items: RefCell::new(items),
            version: Cell::new(0),
            comparer,
        }))
    }
    /// Wraps a complete custom list without copying or substituting its behavior.
    pub fn from_list(list: Rc<dyn IList<T>>) -> Self {
        Self { inner: list }
    }
    /// Returns the original shared list contract.
    pub fn as_list(&self) -> Rc<dyn IList<T>> {
        self.inner.clone()
    }
    /// Compares list reference identity.
    pub fn ptr_eq(&self, other: &Self) -> bool {
        self == other
    }
    /// Number of elements.
    pub fn count(&self) -> usize {
        self.inner.count()
    }
    /// Whether there are no elements.
    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }
    /// Whether the backend reports a read-only collection.
    pub fn is_read_only(&self) -> bool {
        self.inner.is_read_only()
    }
    /// Gets an element.
    pub fn get(&self, index: usize) -> T {
        self.inner.get(index)
    }
    /// Replaces an element.
    pub fn set(&self, index: usize, item: T) {
        self.inner.set(index, item);
    }
    /// Appends an element.
    pub fn add(&self, item: T) {
        self.inner.add(item);
    }
    /// Inserts an element.
    pub fn insert(&self, index: usize, item: T) {
        self.inner.insert(index, item);
    }
    /// Removes an element by index.
    pub fn remove_at(&self, index: usize) {
        self.inner.remove_at(index);
    }
    /// Removes the first matching element.
    pub fn remove(&self, item: &T) -> bool {
        self.inner.remove(item)
    }
    /// Removes all elements.
    pub fn clear(&self) {
        self.inner.clear();
    }
    /// Finds an element using the backend's comparer.
    pub fn index_of(&self, item: &T) -> Option<usize> {
        self.inner.index_of(item)
    }
    /// Tests for an element using the backend's comparer.
    pub fn contains(&self, item: &T) -> bool {
        self.inner.contains(item)
    }
    /// Copies into an existing buffer.
    pub fn copy_to(&self, destination: &mut [T], index: usize) {
        self.inner.copy_to(destination, index);
    }
    /// Gets an independent, live enumerator.
    pub fn iter(&self) -> Box<dyn Iterator<Item = T>> {
        self.inner.clone().get_enumerator()
    }
    /// Copies elements into an independent vector, as source ToList does.
    pub fn to_vec(&self) -> Vec<T> {
        self.iter().collect()
    }
    /// Returns optional collection notifications from the original backend.
    pub fn as_notify_collection_changed(&self) -> Option<&dyn INotifyCollectionChanged> {
        self.inner.as_notify_collection_changed()
    }
    /// Original optional property notifications, without a relay event.
    pub fn as_notify_property_changed(
        &self,
    ) -> Option<&dyn ferroui_system::component_model::INotifyPropertyChanged> {
        self.inner.as_notify_property_changed()
    }
    /// Whether typed source change arguments are available.
    pub fn has_typed_collection_changes(&self) -> bool {
        self.inner.has_typed_collection_changes()
    }
    /// Subscribes directly to original typed collection changes.
    pub fn add_collection_changed(
        &self,
        handler: Rc<super::ReadOnlyListChanged<T>>,
    ) -> Option<u64> {
        self.inner.add_collection_changed(handler)
    }
    /// Removes a typed source subscription.
    pub fn remove_collection_changed(&self, token: u64) {
        self.inner.remove_collection_changed(token);
    }
    /// Stable identity of the original collection.
    pub fn identity(&self) -> usize {
        self.inner.identity()
    }
    /// Gets the wrapped FerroUI observable collection, when applicable.
    pub fn as_observable_collection(&self) -> Option<ObservableCollection<T>>
    where
        T: PartialEq,
    {
        self.inner.as_observable_collection()
    }
}
impl<T: Clone + PartialEq + 'static> ListHandle<T> {
    /// Creates the default observable list using FerroUI's existing collection.
    pub fn new() -> Self {
        Self::from_items(Vec::new())
    }
    /// Creates an observable list with these initial elements.
    pub fn from_items(items: Vec<T>) -> Self {
        Self::from_observable(ObservableCollection::from_items(items))
    }
    /// Wraps a shared FerroUI collection, retaining its identity and notifications.
    pub fn from_observable(collection: ObservableCollection<T>) -> Self {
        Self::from_list(Rc::new(ObservableList(collection)))
    }
    /// Creates an ordinary nonnotifying list with fail-fast enumeration.
    pub fn from_plain(items: Vec<T>) -> Self {
        Self::from_plain_with_comparer(items, Rc::new(|left, right| left == right))
    }
}
impl<T: Clone + PartialEq + 'static> Default for ListHandle<T> {
    fn default() -> Self {
        Self::new()
    }
}
impl<T: Clone + PartialEq + 'static> From<ObservableCollection<T>> for ListHandle<T> {
    fn from(value: ObservableCollection<T>) -> Self {
        Self::from_observable(value)
    }
}

struct PlainList<T> {
    items: RefCell<Vec<T>>,
    version: Cell<u32>,
    comparer: ListComparer<T>,
}
impl<T> PlainList<T> {
    fn changed(&self) {
        self.version.set(self.version.get().wrapping_add(1));
    }
}
impl<T: Clone + 'static> IList<T> for PlainList<T> {
    fn count(&self) -> usize {
        self.items.borrow().len()
    }
    fn get(&self, index: usize) -> T {
        self.items.borrow()[index].clone()
    }
    fn set(&self, index: usize, item: T) {
        let previous = std::mem::replace(&mut self.items.borrow_mut()[index], item);
        self.changed();
        drop(previous);
    }
    fn insert(&self, index: usize, item: T) {
        self.items.borrow_mut().insert(index, item);
        self.changed();
    }
    fn remove_at(&self, index: usize) {
        let previous = self.items.borrow_mut().remove(index);
        self.changed();
        drop(previous);
    }
    fn clear(&self) {
        let previous = self.items.replace(Vec::new());
        self.changed();
        drop(previous);
    }
    fn index_of(&self, item: &T) -> Option<usize> {
        let count = self.count();
        for index in 0..count {
            let candidate = self.items.borrow().get(index).cloned();
            if let Some(candidate) = candidate {
                if (self.comparer)(&candidate, item) {
                    return Some(index);
                }
            }
        }
        None
    }
    fn version(&self) -> Option<u32> {
        Some(self.version.get())
    }
}

struct ObservableList<T>(ObservableCollection<T>);
impl<T: Clone + PartialEq + 'static> IList<T> for ObservableList<T> {
    fn count(&self) -> usize {
        self.0.count()
    }
    fn get(&self, index: usize) -> T {
        self.0.get(index)
    }
    fn set(&self, index: usize, item: T) {
        self.0.set(index, item);
    }
    fn insert(&self, index: usize, item: T) {
        self.0.insert(index, item);
    }
    fn remove_at(&self, index: usize) {
        self.0.remove_at(index);
    }
    fn clear(&self) {
        self.0.clear();
    }
    fn index_of(&self, item: &T) -> Option<usize> {
        self.0.index_of(item)
    }
    fn as_notify_collection_changed(&self) -> Option<&dyn INotifyCollectionChanged> {
        Some(&self.0)
    }
    fn as_notify_property_changed(
        &self,
    ) -> Option<&dyn ferroui_system::component_model::INotifyPropertyChanged> {
        Some(&self.0)
    }
    fn has_typed_collection_changes(&self) -> bool {
        true
    }
    fn add_collection_changed(&self, handler: Rc<super::ReadOnlyListChanged<T>>) -> Option<u64> {
        Some(self.0.add_collection_changed(handler))
    }
    fn remove_collection_changed(&self, token: u64) {
        self.0.remove_collection_changed(token);
    }
    fn identity(&self) -> usize {
        self.0.collection_changed() as *const _ as usize
    }
    fn as_observable_collection(&self) -> Option<ObservableCollection<T>> {
        Some(self.0.clone())
    }
    fn reference_equals(&self, other: &dyn IList<T>) -> bool {
        (other as &dyn Any)
            .downcast_ref::<Self>()
            .is_some_and(|other| self.0.ptr_eq(&other.0))
    }
    // FerroUI 0.3.0 does not expose its mutation version. No extra notification
    // handler is attached: that would change its reentrancy subscriber count.
}

struct ListEnumerator<T: Clone + 'static, L: IList<T> + ?Sized> {
    list: Rc<L>,
    index: usize,
    version: Option<u32>,
    item: PhantomData<T>,
}
impl<T: Clone + 'static, L: IList<T> + ?Sized> Iterator for ListEnumerator<T, L> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        assert_eq!(
            self.list.version(),
            self.version,
            "Collection was modified; enumeration operation may not execute."
        );
        if self.index < self.list.count() {
            let value = self.list.get(self.index);
            self.index += 1;
            Some(value)
        } else {
            None
        }
    }
}
