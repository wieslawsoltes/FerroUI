// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Shared source IReadOnlyList references; reads retain the supplied list's behavior.
use ferroui_system::collections::specialized::INotifyCollectionChanged;
use ferroui_system::collections::{
    object_model::ObservableCollection, specialized::NotifyCollectionChangedEventArgs,
};
use std::{any::Any, rc::Rc};
/// Shared typed collection change callback from the original backend.
pub type ReadOnlyListChanged<T> = dyn Fn(&NotifyCollectionChangedEventArgs<'_, T>);
/// The source read-only indexed collection contract, absent from System 0.3.0.
pub trait IReadOnlyList<T: Clone + 'static>: Any {
    /// Gets Count.
    fn count(&self) -> usize;
    /// Gets the indexed element.
    fn get(&self, index: usize) -> T;
    /// Creates an independent source enumerator.
    fn get_enumerator(self: Rc<Self>) -> Box<dyn Iterator<Item = T>>;
    /// Discovers optional notifications on the original object.
    fn as_notify_collection_changed(&self) -> Option<&dyn INotifyCollectionChanged> {
        None
    }
    /// Subscribes to typed source changes when this object is notifying.
    fn add_collection_changed(&self, _handler: Rc<ReadOnlyListChanged<T>>) -> Option<u64> {
        None
    }
    /// Removes a typed source subscription.
    fn remove_collection_changed(&self, _token: u64) {}
    /// Identity of the underlying collection, also across adapter instances.
    fn identity(&self) -> usize;
}
/// An IReadOnlyList reference preserving identity and live/custom collection behavior.
pub struct ReadOnlyListHandle<T: Clone + 'static>(Rc<dyn IReadOnlyList<T>>);
impl<T: Clone + 'static> Clone for ReadOnlyListHandle<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<T: Clone + 'static> PartialEq for ReadOnlyListHandle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.identity() == other.identity()
    }
}
impl<T: Clone + 'static> ReadOnlyListHandle<T> {
    /// Wraps a custom source contract without copying its storage.
    pub fn from_read_only(list: Rc<dyn IReadOnlyList<T>>) -> Self {
        Self(list)
    }
    /// Views a shared native observable collection without copying its storage.
    pub fn from_observable(list: ObservableCollection<T>) -> Self
    where
        T: PartialEq,
    {
        Self(Rc::new(list))
    }
    /// Creates an immutable array-backed collection.
    pub fn from_items(items: Vec<T>) -> Self {
        Self(Rc::new(items))
    }
    /// Returns the original read-only contract.
    pub fn as_read_only(&self) -> Rc<dyn IReadOnlyList<T>> {
        self.0.clone()
    }
    /// Gets Count.
    pub fn count(&self) -> usize {
        self.0.count()
    }
    /// Gets an element.
    pub fn get(&self, index: usize) -> T {
        self.0.get(index)
    }
    /// Gets an independent original enumerator.
    pub fn iter(&self) -> Box<dyn Iterator<Item = T>> {
        self.0.clone().get_enumerator()
    }
    /// Gets the original optional notification contract.
    pub fn as_notify_collection_changed(&self) -> Option<&dyn INotifyCollectionChanged> {
        self.0.as_notify_collection_changed()
    }
    /// Subscribes directly to the original typed changes.
    pub fn add_collection_changed(&self, handler: Rc<ReadOnlyListChanged<T>>) -> Option<u64> {
        self.0.add_collection_changed(handler)
    }
    /// Removes the original typed subscription.
    pub fn remove_collection_changed(&self, token: u64) {
        self.0.remove_collection_changed(token)
    }
    /// Gets the original collection identity.
    pub fn identity(&self) -> usize {
        self.0.identity()
    }
}
impl<T: Clone + PartialEq + 'static> IReadOnlyList<T> for ObservableCollection<T> {
    fn count(&self) -> usize {
        self.count()
    }
    fn get(&self, index: usize) -> T {
        self.get(index)
    }
    fn get_enumerator(self: Rc<Self>) -> Box<dyn Iterator<Item = T>> {
        let mut index = 0;
        Box::new(std::iter::from_fn(move || {
            if index >= self.count() {
                return None;
            }
            let value = self.get(index);
            index += 1;
            Some(value)
        }))
    }
    fn as_notify_collection_changed(&self) -> Option<&dyn INotifyCollectionChanged> {
        Some(self)
    }
    fn add_collection_changed(&self, handler: Rc<ReadOnlyListChanged<T>>) -> Option<u64> {
        Some(self.add_collection_changed(handler))
    }
    fn remove_collection_changed(&self, token: u64) {
        self.remove_collection_changed(token);
    }
    fn identity(&self) -> usize {
        INotifyCollectionChanged::collection_changed(self) as *const _ as usize
    }
}
impl<T: Clone + 'static> IReadOnlyList<T> for Vec<T> {
    fn count(&self) -> usize {
        self.len()
    }
    fn get(&self, index: usize) -> T {
        self[index].clone()
    }
    fn get_enumerator(self: Rc<Self>) -> Box<dyn Iterator<Item = T>> {
        let mut index = 0;
        Box::new(std::iter::from_fn(move || {
            let value = self.as_slice().get(index)?.clone();
            index += 1;
            Some(value)
        }))
    }
    fn identity(&self) -> usize {
        self as *const Self as usize
    }
}
