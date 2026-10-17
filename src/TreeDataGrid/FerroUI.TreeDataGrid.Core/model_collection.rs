//! The collections of models a source shows: the reference's `IEnumerable<TModel>`, which the
//! sources and the selection treat as an `IList` that may implement `INotifyCollectionChanged`.
//!
//! Not a file of the reference: in Rust the abilities a C# collection has through its runtime type
//! (indexing, change notification, mutation) are stated by a trait. Models are `Rc<M>`, the
//! reference types of the reference (`where TModel : class`), compared by identity.

use ferroui_base::collections::{FerroList, NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs};
use std::cell::RefCell;
use std::rc::Rc;

/// A change of a collection of models: the action, the models and the indexes.
pub type ModelsChangedArgs<'a, M> = NotifyCollectionChangedEventArgs<'a, Rc<M>>;

/// A handler of [`ModelCollection::add_collection_changed`].
pub type ModelsChangedHandler<M> = dyn for<'a> Fn(&ModelsChangedArgs<'a, M>);

/// The models of a source or of the children of a model.
pub trait ModelCollection<M: 'static> {
    /// The number of models.
    fn count(&self) -> usize;

    /// The model at `index`.
    fn get(&self, index: usize) -> Rc<M>;

    /// The index of `model`, compared by identity (the reference's `IList.IndexOf` of a reference
    /// type that does not override `Equals`).
    fn index_of(&self, model: &Rc<M>) -> Option<usize> {
        (0..self.count()).find(|&i| Rc::ptr_eq(&self.get(i), model))
    }

    /// Subscribes to the changes of the collection; `None` when it does not raise any (a
    /// collection that is not `INotifyCollectionChanged`).
    fn add_collection_changed(&self, _handler: Rc<ModelsChangedHandler<M>>) -> Option<u64> {
        None
    }

    /// Removes a subscription of [`add_collection_changed`](Self::add_collection_changed).
    fn remove_collection_changed(&self, _token: u64) {}

    /// Whether the collection raises change notifications.
    fn is_notifying(&self) -> bool {
        false
    }

    /// The mutable list behind the collection (`IList` that is not read only), for the moves of
    /// drag and drop.
    fn as_mutable(&self) -> Option<&dyn MutableModelCollection<M>> {
        None
    }

    /// The models, in order.
    fn to_vec(&self) -> Vec<Rc<M>> {
        (0..self.count()).map(|i| self.get(i)).collect()
    }
}

/// The mutations of a mutable collection of models (`IList<TModel>`).
pub trait MutableModelCollection<M: 'static> {
    fn insert(&self, index: usize, model: Rc<M>);
    fn remove_at(&self, index: usize);
    fn set(&self, index: usize, model: Rc<M>);
}

/// The shared form of a collection of models.
pub type Models<M> = Rc<dyn ModelCollection<M>>;

/// The identity of a collection, the key of the per-collection listener lists of
/// [`crate::utils::CollectionChangedEventManager`].
pub fn collection_identity<M: 'static>(collection: &Models<M>) -> usize {
    Rc::as_ptr(collection) as *const () as usize
}

// A notifying list: the reference's `ObservableCollection<T>` and `AvaloniaList<T>`.
impl<M: PartialEq + 'static> ModelCollection<M> for FerroList<Rc<M>> {
    fn count(&self) -> usize {
        FerroList::count(self)
    }

    fn get(&self, index: usize) -> Rc<M> {
        FerroList::get(self, index)
    }

    fn index_of(&self, model: &Rc<M>) -> Option<usize> {
        let mut found = None;
        let mut i = 0;
        self.for_each(|item| {
            if found.is_none() && Rc::ptr_eq(item, model) {
                found = Some(i);
            }
            i += 1;
        });
        found
    }

    fn add_collection_changed(&self, handler: Rc<ModelsChangedHandler<M>>) -> Option<u64> {
        Some(FerroList::add_collection_changed(self, handler))
    }

    fn remove_collection_changed(&self, token: u64) {
        FerroList::remove_collection_changed(self, token);
    }

    fn is_notifying(&self) -> bool {
        true
    }

    fn as_mutable(&self) -> Option<&dyn MutableModelCollection<M>> {
        Some(self)
    }

    fn to_vec(&self) -> Vec<Rc<M>> {
        FerroList::to_vec(self)
    }
}

impl<M: PartialEq + 'static> MutableModelCollection<M> for FerroList<Rc<M>> {
    fn insert(&self, index: usize, model: Rc<M>) {
        FerroList::insert(self, index, model);
    }

    fn remove_at(&self, index: usize) {
        FerroList::remove_at(self, index);
    }

    fn set(&self, index: usize, model: Rc<M>) {
        FerroList::set(self, index, model);
    }
}

// A list that does not notify: the reference's `List<T>` and arrays.
impl<M: 'static> ModelCollection<M> for RefCell<Vec<Rc<M>>> {
    fn count(&self) -> usize {
        self.borrow().len()
    }

    fn get(&self, index: usize) -> Rc<M> {
        self.borrow()[index].clone()
    }

    fn as_mutable(&self) -> Option<&dyn MutableModelCollection<M>> {
        Some(self)
    }

    fn to_vec(&self) -> Vec<Rc<M>> {
        self.borrow().clone()
    }
}

impl<M: 'static> MutableModelCollection<M> for RefCell<Vec<Rc<M>>> {
    fn insert(&self, index: usize, model: Rc<M>) {
        self.borrow_mut().insert(index, model);
    }

    fn remove_at(&self, index: usize) {
        self.borrow_mut().remove(index);
    }

    fn set(&self, index: usize, model: Rc<M>) {
        self.borrow_mut()[index] = model;
    }
}

// A fixed sequence of models (an array or a materialised enumerable).
impl<M: 'static> ModelCollection<M> for Vec<Rc<M>> {
    fn count(&self) -> usize {
        self.len()
    }

    fn get(&self, index: usize) -> Rc<M> {
        self[index].clone()
    }

    fn to_vec(&self) -> Vec<Rc<M>> {
        self.clone()
    }
}

/// A notifying list of models with the given items.
pub fn observable<M: 'static>(items: impl IntoIterator<Item = Rc<M>>) -> Rc<FerroList<Rc<M>>> {
    Rc::new(FerroList::from_items(items))
}

/// A list of models that does not notify (`List<T>`).
pub fn list<M: 'static>(items: impl IntoIterator<Item = Rc<M>>) -> Rc<RefCell<Vec<Rc<M>>>> {
    Rc::new(RefCell::new(items.into_iter().collect()))
}

/// The arguments of a reset notification.
pub fn reset_args<'a, M>() -> ModelsChangedArgs<'a, M> {
    NotifyCollectionChangedEventArgs {
        action: NotifyCollectionChangedAction::Reset,
        new_items: &[],
        old_items: &[],
        new_starting_index: -1,
        old_starting_index: -1,
    }
}
