//! Shared handles to observable lists, as used by the media object model.
//!
//! [`MediaCollection`] is a reference-counted handle to a
//! [`FerroList`](crate::collections::FerroList) that compares by identity, so
//! a list can be the value of a property (property values must be
//! `Clone + PartialEq`). The module also holds the list helpers the media
//! classes rely on: [`MediaCollection::for_each_item`] and
//! [`track_item_property_changed`].

use crate::collections::{FerroList, NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs};
use crate::reactive::{Disposable, IDisposable};
use crate::{FerroObject, ObjectType, Ref};
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

pub use crate::collections::ResetBehavior;

/// A shared, observable list. Cloning the handle shares the list; equality is
/// by identity.
pub struct MediaCollection<T: Clone + 'static> {
    list: Rc<FerroList<T>>,
}

impl<T: Clone + 'static> Clone for MediaCollection<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self { list: self.list.clone() }
    }
}

impl<T: Clone + 'static> PartialEq for MediaCollection<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.list, &other.list)
    }
}

impl<T: Clone + 'static> Default for MediaCollection<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone + fmt::Debug + 'static> fmt::Debug for MediaCollection<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.list.snapshot().iter()).finish()
    }
}

impl<T: Clone + 'static> FromIterator<T> for MediaCollection<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self::from_items(iter)
    }
}

impl<T: Clone + 'static> MediaCollection<T> {
    /// Creates an empty list.
    pub fn new() -> Self {
        Self { list: Rc::new(FerroList::new()) }
    }

    /// Creates a list holding `items`.
    pub fn from_items(items: impl IntoIterator<Item = T>) -> Self {
        Self { list: Rc::new(FerroList::from_items(items)) }
    }

    /// Wraps an existing list.
    pub fn from_list(list: Rc<FerroList<T>>) -> Self {
        Self { list }
    }

    /// The underlying list.
    #[inline]
    pub fn list(&self) -> &Rc<FerroList<T>> {
        &self.list
    }

    /// Whether both handles refer to the same list.
    #[inline]
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.list, &other.list)
    }

    /// The action notified when the list is cleared.
    #[inline]
    pub fn reset_behavior(&self) -> ResetBehavior {
        self.list.reset_behavior()
    }

    /// Sets the action notified when the list is cleared.
    #[inline]
    pub fn set_reset_behavior(&self, value: ResetBehavior) {
        self.list.set_reset_behavior(value)
    }

    /// The number of items.
    #[inline]
    pub fn len(&self) -> usize {
        self.list.count()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    /// The item at `index`. Panics if the index is out of range.
    #[inline]
    pub fn get(&self, index: usize) -> T {
        self.list.get(index)
    }

    /// A copy of the items.
    #[inline]
    pub fn to_vec(&self) -> Vec<T> {
        self.list.to_vec()
    }

    /// Iterates over a snapshot of the items.
    #[inline]
    pub fn iter(&self) -> std::vec::IntoIter<T> {
        self.to_vec().into_iter()
    }

    /// Appends an item.
    #[inline]
    pub fn add(&self, item: T) {
        self.list.add(item)
    }

    /// Appends several items with a single notification.
    #[inline]
    pub fn add_range(&self, items: impl IntoIterator<Item = T>) {
        self.list.add_range(items)
    }

    /// Inserts an item at `index`.
    #[inline]
    pub fn insert(&self, index: usize, item: T) {
        self.list.insert(index, item)
    }

    /// Removes the item at `index`.
    #[inline]
    pub fn remove_at(&self, index: usize) {
        self.list.remove_at(index)
    }

    /// Removes all items.
    #[inline]
    pub fn clear(&self) {
        self.list.clear()
    }

    /// Subscribes to changes of the list. Disposing the returned handle
    /// unsubscribes.
    pub fn collection_changed(
        &self,
        handler: impl for<'a, 'b> Fn(&'a NotifyCollectionChangedEventArgs<'b, T>) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.list.add_collection_changed(Rc::new(handler));
        let weak = Rc::downgrade(&self.list);
        Disposable::create(move || {
            if let Some(list) = weak.upgrade() {
                list.remove_collection_changed(token);
            }
        })
    }

    /// Invokes `added` for every item currently in the list and for every item
    /// added later, `removed` for every item removed and `reset` when the list
    /// is reset.
    pub fn for_each_item(
        &self,
        added: impl Fn(usize, &T) + 'static,
        removed: impl Fn(usize, &T) + 'static,
        reset: impl Fn() + 'static,
    ) -> Rc<dyn IDisposable> {
        let add = move |index: usize, items: &[T]| {
            for (i, item) in items.iter().enumerate() {
                added(index + i, item);
            }
        };
        let remove = move |index: usize, items: &[T]| {
            for (i, item) in items.iter().enumerate().rev() {
                removed(index + i, item);
            }
        };

        add(0, &self.list.snapshot());

        self.collection_changed(move |e| match e.action {
            NotifyCollectionChangedAction::Add => add(e.new_starting_index as usize, e.new_items),
            NotifyCollectionChangedAction::Move => {
                if e.old_starting_index < 0 {
                    return reset();
                }

                remove(e.old_starting_index as usize, e.old_items);
                let mut new_index = e.new_starting_index;

                if new_index > e.old_starting_index {
                    new_index -= e.old_items.len() as i32 - 1;
                }

                add(new_index as usize, e.new_items);
            }
            NotifyCollectionChangedAction::Replace => {
                if e.old_starting_index < 0 {
                    return reset();
                }

                remove(e.old_starting_index as usize, e.old_items);
                add(e.new_starting_index as usize, e.new_items);
            }
            NotifyCollectionChangedAction::Remove => remove(e.old_starting_index as usize, e.old_items),
            NotifyCollectionChangedAction::Reset => reset(),
        })
    }
}

impl<T: Clone + PartialEq + 'static> MediaCollection<T> {
    /// Replaces the item at `index`.
    #[inline]
    pub fn set(&self, index: usize, value: T) {
        self.list.set(index, value)
    }

    /// Whether the list contains `item`.
    #[inline]
    pub fn contains(&self, item: &T) -> bool {
        self.list.contains(item)
    }

    /// The index of the first occurrence of `item`.
    #[inline]
    pub fn index_of(&self, item: &T) -> Option<usize> {
        self.list.index_of(item)
    }

    /// Removes the first occurrence of `item`; returns whether it was found.
    #[inline]
    pub fn remove(&self, item: &T) -> bool {
        self.list.remove(item)
    }
}

/// Invokes `callback` whenever a property changes on any item of the list.
///
/// A reset of the list is not supported and panics.
pub fn track_item_property_changed<T: ObjectType>(
    collection: &MediaCollection<Ref<T>>,
    callback: impl Fn() + 'static,
) -> Rc<dyn IDisposable> {
    type Tracked = Rc<RefCell<Vec<(Ref<FerroObject>, Rc<dyn IDisposable>)>>>;
    let tracked: Tracked = Rc::new(RefCell::new(Vec::new()));
    let callback: Rc<dyn Fn()> = Rc::new(callback);

    let tracked_add = tracked.clone();
    let tracked_remove = tracked.clone();
    // The subscription to the list itself is intentionally kept, as in the
    // reference implementation; disposing only detaches the tracked items.
    let _ = collection.for_each_item(
        move |_, item: &Ref<T>| {
            let callback = callback.clone();
            let object: &FerroObject = (**item).upcast();
            let subscription = object.property_changed(move |_| callback());
            tracked_add.borrow_mut().push((object.to_ref(), subscription));
        },
        move |_, item: &Ref<T>| {
            let object: &FerroObject = (**item).upcast();
            let object = object.to_ref();
            let position = tracked_remove.borrow().iter().position(|(o, _)| o.ptr_eq(&object));
            if let Some(position) = position {
                let (_, subscription) = tracked_remove.borrow_mut().remove(position);
                subscription.dispose();
            }
        },
        || panic!("Collection reset not supported."),
    );

    Disposable::create(move || {
        let items = std::mem::take(&mut *tracked.borrow_mut());
        for (_, subscription) in items {
            subscription.dispose();
        }
    })
}

/// Declares a named list type wrapping a [`MediaCollection`].
macro_rules! media_collection_type {
    ($(#[$meta:meta])* $name:ident, $item:ty) => {
        $(#[$meta])*
        #[derive(Clone, PartialEq)]
        pub struct $name($crate::media::MediaCollection<$item>);

        impl ::std::ops::Deref for $name {
            type Target = $crate::media::MediaCollection<$item>;
            #[inline]
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl ::std::default::Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl ::std::iter::FromIterator<$item> for $name {
            fn from_iter<I: IntoIterator<Item = $item>>(iter: I) -> Self {
                Self::from_items(iter)
            }
        }
    };
}

pub(crate) use media_collection_type;
