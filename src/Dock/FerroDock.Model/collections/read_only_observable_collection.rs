// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
// Native adapter for the .NET ReadOnlyObservableCollection contract, absent
// from the pinned shared System crate. Storage/mutation remain in that crate.

use super::{IList, ListHandle};
use ferroui_system::collections::object_model::{CollectionChangedHandler, ObservableCollection};
use ferroui_system::collections::specialized::{
    CollectionChange, INotifyCollectionChanged, NotifyCollectionChangedEventArgs,
};
use ferroui_system::component_model::INotifyPropertyChanged;
use ferroui_system::events::{Event, HandlerList};
use std::any::Any;
use std::cell::Cell;
use std::rc::Rc;

struct Inner<T: Clone + PartialEq + 'static> {
    source: ObservableCollection<T>,
    typed: HandlerList<CollectionChangedHandler<T>>,
    changed: Event<CollectionChange>,
    properties: Event<str>,
    collection_token: Cell<Option<u64>>,
    property_token: Cell<Option<u64>>,
}
impl<T: Clone + PartialEq + 'static> Drop for Inner<T> {
    fn drop(&mut self) {
        if let Some(token) = self.collection_token.take() {
            self.source.remove_collection_changed(token);
        }
        if let Some(token) = self.property_token.take() {
            self.source.property_changed().remove(token);
        }
    }
}
/// A live read-only view over a shared FerroUI observable collection. It forwards
/// the original notifications without exposing the underlying mutable collection.
pub struct ReadOnlyObservableCollection<T: Clone + PartialEq + 'static> {
    inner: Rc<Inner<T>>,
}
impl<T: Clone + PartialEq + 'static> Clone for ReadOnlyObservableCollection<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}
impl<T: Clone + PartialEq + 'static> PartialEq for ReadOnlyObservableCollection<T> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}
impl<T: Clone + PartialEq + 'static> ReadOnlyObservableCollection<T> {
    /// Wraps, without copying, the collection and attaches one source collection
    /// handler and one source property handler, as the original wrapper does.
    pub fn new(source: ObservableCollection<T>) -> Self {
        let inner = Rc::new_cyclic(|weak: &std::rc::Weak<Inner<T>>| {
            let ids = Rc::new(Cell::new(0));
            let inner = Inner {
                source: source.clone(),
                typed: HandlerList::with_shared_ids(ids.clone()),
                changed: Event::with_shared_ids(ids),
                properties: Event::new(),
                collection_token: Cell::new(None),
                property_token: Cell::new(None),
            };
            let target = weak.clone();
            inner
                .collection_token
                .set(Some(source.add_collection_changed(Rc::new(move |args| {
                    if let Some(inner) = target.upgrade() {
                        Self::forward(&inner, args);
                    }
                }))));
            let target = weak.clone();
            inner
                .property_token
                .set(Some(source.property_changed().add(Rc::new(move |name| {
                    if let Some(inner) = target.upgrade() {
                        inner.properties.raise(name);
                    }
                }))));
            inner
        });
        Self { inner }
    }
    fn forward(inner: &Inner<T>, args: &NotifyCollectionChangedEventArgs<'_, T>) {
        let typed = inner.typed.snapshot();
        let untyped = inner.changed.snapshot();
        let change = CollectionChange {
            action: args.action,
            new_starting_index: args.new_starting_index,
            new_count: args.new_items.len(),
            old_starting_index: args.old_starting_index,
            old_count: args.old_items.len(),
        };
        let (mut a, mut b) = (0, 0);
        while a < typed.len() || b < untyped.len() {
            if b == untyped.len() || (a < typed.len() && typed[a].0 < untyped[b].0) {
                (typed[a].1)(args);
                a += 1;
            } else {
                (untyped[b].1)(&change);
                b += 1;
            }
        }
    }
    /// Gets whether two handles refer to the same read-only view.
    pub fn ptr_eq(&self, other: &Self) -> bool {
        self == other
    }
    /// Gets the stable identity of this shared read-only collection for native UI adapters.
    pub fn identity(&self) -> usize {
        Rc::as_ptr(&self.inner) as usize
    }
    /// Gets the current live count.
    pub fn count(&self) -> usize {
        self.inner.source.count()
    }
    /// Gets whether the current live collection is empty.
    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }
    /// Gets the current item at an index.
    pub fn get(&self, index: usize) -> T {
        self.inner.source.get(index)
    }
    /// Tests membership using the underlying collection's equality.
    pub fn contains(&self, item: &T) -> bool {
        self.inner.source.contains(item)
    }
    /// Finds the first matching live item.
    pub fn index_of(&self, item: &T) -> Option<usize> {
        self.inner.source.index_of(item)
    }
    /// Copies current values into an existing buffer without exposing mutation.
    pub fn copy_to(&self, destination: &mut [T], index: usize) {
        self.as_list_handle().copy_to(destination, index);
    }
    /// Gets a live enumerator. Shared observable mutation-version limitations apply.
    pub fn iter(&self) -> Box<dyn Iterator<Item = T>> {
        self.as_list_handle().iter()
    }
    /// Takes an explicit independent snapshot, corresponding to ToArray/ToList.
    pub fn to_vec(&self) -> Vec<T> {
        self.inner.source.to_vec()
    }
    /// Adds a typed change handler; notifications share the source arguments.
    pub fn add_collection_changed(&self, handler: Rc<CollectionChangedHandler<T>>) -> u64 {
        self.inner.typed.add(handler)
    }
    /// Removes a typed change handler by its token.
    pub fn remove_collection_changed(&self, token: u64) -> bool {
        self.inner.typed.remove(token)
    }
    /// Gets the explicit IList representation. Every mutator throws, even a
    /// removal of an absent item; the source mutable collection is not exposed.
    pub fn as_list_handle(&self) -> ListHandle<T> {
        ListHandle::from_list(Rc::new(ReadOnlyList(self.clone())))
    }
}
impl<T: Clone + PartialEq + 'static> INotifyCollectionChanged for ReadOnlyObservableCollection<T> {
    fn collection_changed(&self) -> &Event<CollectionChange> {
        &self.inner.changed
    }
}
impl<T: Clone + PartialEq + 'static> INotifyPropertyChanged for ReadOnlyObservableCollection<T> {
    fn property_changed(&self) -> &Event<str> {
        &self.inner.properties
    }
}
struct ReadOnlyList<T: Clone + PartialEq + 'static>(ReadOnlyObservableCollection<T>);
impl<T: Clone + PartialEq + 'static> IList<T> for ReadOnlyList<T> {
    fn count(&self) -> usize {
        self.0.count()
    }
    fn is_read_only(&self) -> bool {
        true
    }
    fn get(&self, index: usize) -> T {
        self.0.get(index)
    }
    fn set(&self, _index: usize, _item: T) {
        panic!("Collection is read-only.");
    }
    fn insert(&self, _index: usize, _item: T) {
        panic!("Collection is read-only.");
    }
    fn remove_at(&self, _index: usize) {
        panic!("Collection is read-only.");
    }
    fn clear(&self) {
        panic!("Collection is read-only.");
    }
    fn add(&self, _item: T) {
        panic!("Collection is read-only.");
    }
    fn remove(&self, _item: &T) -> bool {
        panic!("Collection is read-only.");
    }
    fn index_of(&self, item: &T) -> Option<usize> {
        self.0.index_of(item)
    }
    fn as_notify_collection_changed(&self) -> Option<&dyn INotifyCollectionChanged> {
        Some(&self.0)
    }
    fn reference_equals(&self, other: &dyn IList<T>) -> bool {
        (other as &dyn Any)
            .downcast_ref::<Self>()
            .is_some_and(|other| self.0.ptr_eq(&other.0))
    }
}
