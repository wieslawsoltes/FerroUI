use super::Event;
use crate::collections::{FerroList, NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs};
use crate::data::core::ValueType;
use crate::{BoxedValue, PropertyValue};
use std::cell::OnceCell;
use std::rc::Rc;

/// An untyped description of a change to a collection, as seen by bindings
/// with an indexer in their path.
///
/// Indexes are `-1` when not applicable to the action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CollectionChange {
    pub action: NotifyCollectionChangedAction,
    pub new_starting_index: i32,
    pub new_count: usize,
    pub old_starting_index: i32,
    pub old_count: usize,
}

/// Notifies clients of changes to a collection, such as when items are added
/// and removed or the whole collection is reset.
pub trait INotifyCollectionChanged {
    /// The event raised when the collection changes.
    fn collection_changed(&self) -> &Event<CollectionChange>;
}

/// Untyped indexed access to a list, for bindings with an indexer in their
/// path (`Items[2]`).
pub trait IBindableList {
    /// The number of items.
    fn count(&self) -> usize;

    /// The type of the items.
    fn item_type(&self) -> ValueType {
        ValueType::object()
    }

    /// The item at `index` as an untyped value, or `None` if out of range.
    fn get_item(&self, index: usize) -> Option<BoxedValue>;

    /// Replaces the item at `index`. Returns false if the index is out of
    /// range, the list is read-only or the value is of the wrong type.
    fn set_item(&self, index: usize, value: &BoxedValue) -> bool;

    /// The change notifications of the list, if it is a notifying list.
    fn as_notify_collection_changed(&self) -> Option<&dyn INotifyCollectionChanged>;
}

/// A notifying list that bindings can index into: a [`FerroList`] together
/// with the untyped change event that binding indexers subscribe to.
///
/// As a model object it has identity equality.
pub struct BindableList<T: PropertyValue> {
    items: FerroList<T>,
    changed: Event<CollectionChange>,
    forwarder: OnceCell<u64>,
}

impl<T: PropertyValue> BindableList<T> {
    /// Creates a list with the given items.
    pub fn new(items: impl IntoIterator<Item = T>) -> Rc<Self> {
        let this = Rc::new(Self { items: FerroList::from_items(items), changed: Event::new(), forwarder: OnceCell::new() });
        let weak = Rc::downgrade(&this);
        let token = this.items.add_collection_changed(Rc::new(move |e: &NotifyCollectionChangedEventArgs<'_, T>| {
            if let Some(this) = weak.upgrade() {
                this.changed.raise(&CollectionChange {
                    action: e.action,
                    new_starting_index: e.new_starting_index,
                    new_count: e.new_items.len(),
                    old_starting_index: e.old_starting_index,
                    old_count: e.old_items.len(),
                });
            }
        }));
        let _ = this.forwarder.set(token);
        super::ModelTypes::register_list::<Self>();
        this
    }

    /// The typed list.
    pub fn items(&self) -> &FerroList<T> {
        &self.items
    }
}

impl<T: PropertyValue> PartialEq for BindableList<T> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl<T: PropertyValue> INotifyCollectionChanged for BindableList<T> {
    fn collection_changed(&self) -> &Event<CollectionChange> {
        &self.changed
    }
}

impl<T: PropertyValue> IBindableList for BindableList<T> {
    fn count(&self) -> usize {
        self.items.count()
    }

    fn item_type(&self) -> ValueType {
        ValueType::of::<T>()
    }

    fn get_item(&self, index: usize) -> Option<BoxedValue> {
        self.items.try_get(index).map(|v| Rc::new(v) as BoxedValue)
    }

    fn set_item(&self, index: usize, value: &BoxedValue) -> bool {
        match value.downcast_ref::<T>() {
            Some(v) if index < self.items.count() => {
                self.items.set(index, v.clone());
                true
            }
            _ => false,
        }
    }

    fn as_notify_collection_changed(&self) -> Option<&dyn INotifyCollectionChanged> {
        Some(self)
    }
}
