use crate::items_source::{ItemsChangedEventArgs, ItemsSource};
use crate::ItemsSourceView;
use ferroui_base::collections::{FerroList, ResetBehavior};
use ferroui_base::utilities::HandlerList;
use ferroui_base::BoxedValue;
use std::cell::{Cell, RefCell};
use std::ops::Deref;
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Items,
    ItemsSource,
}

type DefaultCollection = FerroList<Option<BoxedValue>>;

/// Holds the list of items that constitute the content of an
/// `ItemsControl`.
///
/// The value is a shared handle: clones refer to the same collection, and
/// handles compare by identity.
#[derive(Clone)]
pub struct ItemCollection(Rc<ItemCollectionData>);

struct ItemCollectionData {
    view: Rc<ItemsSourceView>,
    mode: Cell<Mode>,
    /// The writable collection behind the view while in items mode.
    writable: RefCell<Option<Rc<DefaultCollection>>>,
    source_changed: HandlerList<dyn Fn()>,
}

impl PartialEq for ItemCollection {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl std::fmt::Debug for ItemCollection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ItemCollection({})", self.0.view.count())
    }
}

impl Deref for ItemCollection {
    type Target = ItemsSourceView;

    fn deref(&self) -> &ItemsSourceView {
        &self.0.view
    }
}

impl ItemCollection {
    pub(crate) fn new() -> Self {
        Self(Rc::new(ItemCollectionData {
            view: ItemsSourceView::new(ItemsSourceView::uninitialized_source()),
            mode: Cell::new(Mode::Items),
            writable: RefCell::new(None),
            source_changed: HandlerList::new(),
        }))
    }

    /// The read-only view of the collection.
    pub fn view(&self) -> &Rc<ItemsSourceView> {
        &self.0.view
    }

    /// Replaces the item at the specified index.
    ///
    /// Panics if the collection is in items source mode.
    pub fn set(&self, index: usize, value: Option<BoxedValue>) {
        self.writable_source().set(index, value)
    }

    /// Whether the collection is in items source mode.
    pub fn is_read_only(&self) -> bool {
        self.0.mode.get() == Mode::ItemsSource
    }

    /// Raised when the source of the collection has been replaced.
    pub(crate) fn add_source_changed(&self, handler: Rc<dyn Fn()>) -> u64 {
        self.0.source_changed.add(handler)
    }

    /// Adds an item to the items control.
    ///
    /// Returns the position into which the new element was inserted.
    /// Panics if the collection is in items source mode.
    pub fn add(&self, value: Option<BoxedValue>) -> i32 {
        let source = self.writable_source();
        source.add(value);
        source.count() as i32 - 1
    }

    /// Clears the collection and releases the references on all items
    /// currently in the collection.
    ///
    /// Panics if the collection is in items source mode.
    pub fn clear(&self) {
        self.writable_source().clear()
    }

    /// Inserts an element into the collection at the specified index.
    ///
    /// Panics if the collection is in items source mode.
    pub fn insert(&self, index: usize, value: Option<BoxedValue>) {
        self.writable_source().insert(index, value)
    }

    /// Removes the item at the specified index of the collection or view.
    ///
    /// Panics if the collection is in items source mode.
    pub fn remove_at(&self, index: usize) {
        self.writable_source().remove_at(index)
    }

    /// Removes the specified item reference from the collection or view.
    /// Returns true if the item was removed.
    ///
    /// Panics if the collection is in items source mode.
    pub fn remove(&self, value: &Option<BoxedValue>) -> bool {
        let source = self.writable_source();
        let index = self.0.view.index_of(value);
        if index >= 0 {
            source.remove_at(index as usize);
            true
        } else {
            false
        }
    }

    fn writable_source(&self) -> Rc<DefaultCollection> {
        if self.is_read_only() {
            Self::throw_is_items_source();
        }
        if self.0.view.try_get_initialized_source().is_none() {
            self.set_source_default();
        }
        self.0.writable.borrow().clone().expect("the collection is in items mode")
    }

    pub(crate) fn set_items_source(&self, value: Option<ItemsSource>) {
        if self.0.mode.get() != Mode::ItemsSource && self.0.view.count() > 0 {
            panic!("Items collection must be empty before using ItemsSource.");
        }

        match value {
            Some(value) => {
                self.0.mode.set(Mode::ItemsSource);
                *self.0.writable.borrow_mut() = None;
                self.set_source(value);
            }
            None => {
                self.0.mode.set(Mode::Items);
                self.set_source_default();
            }
        }
    }

    fn set_source_default(&self) {
        let collection = Self::create_default_collection();
        *self.0.writable.borrow_mut() = Some(collection.clone());
        self.set_source(collection.into());
    }

    fn set_source(&self, source: ItemsSource) {
        let old_source = self.0.view.source();

        self.0.view.set_source(source.clone());

        // The notifications carry the sources themselves, as windows: no
        // item is read here. The removed items are those of the old source,
        // which the view no longer holds.
        if old_source.count() > 0 {
            self.0.view.raise_collection_changed(&ItemsChangedEventArgs::remove(&old_source, 0));
        }
        if source.count() > 0 {
            self.0.view.raise_collection_changed(&ItemsChangedEventArgs::add(&source, 0));
        }
        if !self.0.source_changed.is_empty() {
            for (_, handler) in self.0.source_changed.snapshot().iter() {
                handler();
            }
        }
    }

    fn create_default_collection() -> Rc<DefaultCollection> {
        let collection = DefaultCollection::new();
        collection.set_reset_behavior(ResetBehavior::Remove);
        Rc::new(collection)
    }

    fn throw_is_items_source() -> ! {
        panic!(
            "Operation is not valid while ItemsSource is in use.\
             Access and modify elements with ItemsControl.ItemsSource instead."
        );
    }
}
