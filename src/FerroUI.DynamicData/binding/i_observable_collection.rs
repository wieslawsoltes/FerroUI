//! `IObservableCollection<T>` and `INotifyCollectionChangedSuspender`
//! (`Binding/IObservableCollection.cs`, `Binding/INotifyCollectionChangedSuspender.cs`).

use crate::list::{ChangeSet, IList};
use ferroui_system::collections::specialized::INotifyCollectionChanged;
use ferroui_system::component_model::INotifyPropertyChanged;
use ferroui_system::IDisposable;
use std::rc::Rc;

/// A collection whose notifications can be held back (`INotifyCollectionChangedSuspender`).
pub trait INotifyCollectionChangedSuspender {
    /// Holds back the notifications of `Count` until the handle is disposed, which raises `Count`
    /// once if it changed (`SuspendCount`).
    fn suspend_count(&self) -> Rc<dyn IDisposable>;

    /// Holds back the notifications of changes and of `Count` until the handle is disposed, which
    /// raises `Count` and a reset (`SuspendNotifications`).
    fn suspend_notifications(&self) -> Rc<dyn IDisposable>;
}

/// The collection that the binding operators fill (`IObservableCollection<T>`): a list that
/// notifies its changes and can hold its notifications back.
///
/// Deviation (DEVIATIONS.md, DynamicData): the collection is a shared handle, as the object of
/// .NET is a reference, so its methods take `&self`; the members of `IList<T>` that the binding
/// uses are part of the trait.
pub trait IObservableCollection<T: Clone + PartialEq + 'static>: INotifyCollectionChanged + INotifyPropertyChanged + INotifyCollectionChangedSuspender {
    /// The number of items (`Count`).
    fn count(&self) -> usize;
    /// The item at `index` (the indexer's getter).
    fn get(&self, index: usize) -> T;
    /// Replaces the item at `index` (the indexer's setter).
    fn set(&self, index: usize, item: T);
    /// Inserts an item (`Insert`).
    fn insert(&self, index: usize, item: T);
    /// Removes the item at `index` (`RemoveAt`).
    fn remove_at(&self, index: usize);
    /// Removes every item (`Clear`).
    fn clear(&self);
    /// A copy of the items.
    fn to_vec(&self) -> Vec<T>;

    /// Adds an item at the end (`Add`).
    fn add(&self, item: T) {
        let index = self.count();
        self.insert(index, item);
    }

    /// The index of `item` (`IndexOf`).
    fn index_of(&self, item: &T) -> Option<usize> {
        (0..self.count()).find(|&index| &self.get(index) == item)
    }

    /// Whether the collection has `item` (`Contains`).
    fn contains(&self, item: &T) -> bool {
        self.index_of(item).is_some()
    }

    /// Removes `item`; false when it is not there (`Remove`).
    fn remove(&self, item: &T) -> bool {
        match self.index_of(item) {
            Some(index) => {
                self.remove_at(index);
                true
            }
            None => false,
        }
    }

    /// Clears the collection and adds the items (`Load`).
    fn load(&self, items: Vec<T>);

    /// Moves the item at `old_index` to `new_index` (`Move`).
    fn move_item(&self, old_index: usize, new_index: usize);

    /// Applies the changes of a list (`ListEx.Clone(IList<T>, IChangeSet<T>)`): the general form,
    /// which moves items with [`move_item`](Self::move_item) as upstream does for an
    /// `ObservableCollection<T>`.
    fn clone_changes(&self, changes: &ChangeSet<T>) {
        crate::list::clone_changes(&mut CollectionAsList::new(self), changes);
    }
}

/// An [`IObservableCollection`] seen as an [`IList`], for the functions of `ListEx`.
pub(crate) struct CollectionAsList<'a, T: Clone + PartialEq + 'static, C: IObservableCollection<T> + ?Sized> {
    collection: &'a C,
    item: std::marker::PhantomData<T>,
}

impl<'a, T: Clone + PartialEq + 'static, C: IObservableCollection<T> + ?Sized> CollectionAsList<'a, T, C> {
    pub(crate) fn new(collection: &'a C) -> Self {
        Self { collection, item: std::marker::PhantomData }
    }
}

impl<T: Clone + PartialEq + 'static, C: IObservableCollection<T> + ?Sized> IList<T> for CollectionAsList<'_, T, C> {
    fn count(&self) -> usize {
        self.collection.count()
    }

    fn get(&self, index: usize) -> T {
        self.collection.get(index)
    }

    fn set(&mut self, index: usize, item: T) {
        self.collection.set(index, item);
    }

    fn insert(&mut self, index: usize, item: T) {
        self.collection.insert(index, item);
    }

    fn remove_at(&mut self, index: usize) {
        self.collection.remove_at(index);
    }

    fn clear(&mut self) {
        self.collection.clear();
    }

    fn add(&mut self, item: T) {
        self.collection.add(item);
    }

    fn index_of(&self, item: &T) -> Option<usize> {
        self.collection.index_of(item)
    }

    fn remove(&mut self, item: &T) -> bool {
        self.collection.remove(item)
    }

    fn move_item(&mut self, original: usize, destination: usize) {
        self.collection.move_item(original, destination);
    }
}
