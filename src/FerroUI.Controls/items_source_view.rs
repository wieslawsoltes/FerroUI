// This source file is adapted from the WinUI project.
// (https://github.com/microsoft/microsoft-ui-xaml)

use crate::items_source::{unbox_item, ItemsChangedEventArgs, ItemsChangedHandler, ItemsSource};
use crate::utils::{CollectionChangedEventManager, ICollectionChangedListener};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{BoxedValue, PropertyValue};
use std::cell::{Cell, RefCell};
use std::marker::PhantomData;
use std::ops::Deref;
use std::rc::{Rc, Weak};

thread_local! {
    static EMPTY: Rc<ItemsSourceView> = ItemsSourceView::new(ItemsSource::from_items([]));
    // This is a sentinel value and must be unique.
    static UNINITIALIZED_SOURCE: ItemsSource = ItemsSource::from_items([]);
}

/// Represents a standardized view of the supported interactions between an
/// items collection and an items control.
pub struct ItemsSourceView {
    this: Weak<ItemsSourceView>,
    source: RefCell<ItemsSource>,
    collection_changed: HandlerList<ItemsChangedHandler>,
    pre_collection_changed: HandlerList<ItemsChangedHandler>,
    post_collection_changed: HandlerList<ItemsChangedHandler>,
    listening: Cell<bool>,
}

impl ItemsSourceView {
    /// Gets an empty [`ItemsSourceView`].
    pub fn empty() -> Rc<ItemsSourceView> {
        EMPTY.with(Rc::clone)
    }

    /// Gets an instance representing an uninitialized source.
    pub(crate) fn uninitialized_source() -> ItemsSource {
        UNINITIALIZED_SOURCE.with(Clone::clone)
    }

    /// Initializes a new instance of the view for the specified data source.
    pub(crate) fn new(source: ItemsSource) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            source: RefCell::new(source),
            collection_changed: HandlerList::new(),
            pre_collection_changed: HandlerList::new(),
            post_collection_changed: HandlerList::new(),
            listening: Cell::new(false),
        })
    }

    /// Gets the number of items in the collection.
    #[inline]
    pub fn count(&self) -> usize {
        self.source.borrow().count()
    }

    /// Gets the source collection.
    #[inline]
    pub fn source(&self) -> ItemsSource {
        self.source.borrow().clone()
    }

    pub(crate) fn try_get_initialized_source(&self) -> Option<ItemsSource> {
        let source = self.source();
        if source.ptr_eq(&Self::uninitialized_source()) {
            None
        } else {
            Some(source)
        }
    }

    /// Occurs when the collection has changed to indicate the reason for
    /// the change and which items changed. Returns the token for
    /// [`remove_collection_changed`](Self::remove_collection_changed).
    pub fn add_collection_changed(&self, handler: Rc<ItemsChangedHandler>) -> u64 {
        self.add_listener_if_necessary();
        self.collection_changed.add(handler)
    }

    pub fn remove_collection_changed(&self, token: u64) {
        self.collection_changed.remove(token);
        self.remove_listener_if_necessary();
    }

    /// Occurs when a collection is about to notify its
    /// [`collection changed`](Self::add_collection_changed) handlers.
    pub(crate) fn add_pre_collection_changed(&self, handler: Rc<ItemsChangedHandler>) -> u64 {
        self.add_listener_if_necessary();
        self.pre_collection_changed.add(handler)
    }

    pub(crate) fn remove_pre_collection_changed(&self, token: u64) {
        self.pre_collection_changed.remove(token);
        self.remove_listener_if_necessary();
    }

    /// Occurs when a collection has finished changing and all
    /// [`collection changed`](Self::add_collection_changed) handlers have
    /// been notified.
    pub(crate) fn add_post_collection_changed(&self, handler: Rc<ItemsChangedHandler>) -> u64 {
        self.add_listener_if_necessary();
        self.post_collection_changed.add(handler)
    }

    pub(crate) fn remove_post_collection_changed(&self, token: u64) {
        self.post_collection_changed.remove(token);
        self.remove_listener_if_necessary();
    }

    /// Retrieves the item at the specified index. Panics if out of range.
    #[inline]
    pub fn get_at(&self, index: usize) -> Option<BoxedValue> {
        let source = self.source.borrow().clone();
        source.get_at(index)
    }

    /// Whether the collection contains the item.
    pub fn contains(&self, item: &Option<BoxedValue>) -> bool {
        self.index_of(item) >= 0
    }

    /// The index of the item, or `-1`.
    pub fn index_of(&self, item: &Option<BoxedValue>) -> i32 {
        let source = self.source.borrow().clone();
        source.index_of(item)
    }

    /// Gets or creates an [`ItemsSourceView`] for the specified source.
    ///
    /// Returns [`empty`](Self::empty) if `items` is `None`, otherwise a new
    /// view. (A caller that already holds a view uses it as is.)
    pub fn get_or_create(items: Option<&ItemsSource>) -> Rc<ItemsSourceView> {
        match items {
            None => Self::empty(),
            Some(items) => Self::new(items.clone()),
        }
    }

    /// Gets or creates a typed view for the specified source.
    pub fn get_or_create_of<T: PropertyValue>(items: Option<&ItemsSource>) -> ItemsSourceViewOf<T> {
        match items {
            None => ItemsSourceViewOf::empty(),
            Some(items) => ItemsSourceViewOf::new(items.clone()),
        }
    }

    /// The items, enumerated by index over the live collection.
    pub fn iter(&self) -> impl Iterator<Item = Option<BoxedValue>> {
        let source = self.source();
        let mut index = 0;
        std::iter::from_fn(move || {
            if index < source.count() {
                index += 1;
                Some(source.get_at(index - 1))
            } else {
                None
            }
        })
    }

    /// A copy of the items.
    pub fn to_vec(&self) -> Vec<Option<BoxedValue>> {
        self.source().to_vec()
    }

    pub(crate) fn raise_collection_changed(&self, e: &ItemsChangedEventArgs<'_>) {
        Self::invoke(&self.pre_collection_changed, e);
        Self::invoke(&self.collection_changed, e);
        Self::invoke(&self.post_collection_changed, e);
    }

    pub(crate) fn set_source(&self, source: ItemsSource) {
        let listener: Weak<dyn ICollectionChangedListener> = self.this.clone();

        if self.listening.get() {
            let old = self.source();
            if old.is_notifying() {
                CollectionChangedEventManager::remove_listener(&old, &listener);
            }
        }

        *self.source.borrow_mut() = source.clone();

        if self.listening.get() && source.is_notifying() {
            CollectionChangedEventManager::add_listener(&source, listener);
        }
    }

    fn add_listener_if_necessary(&self) {
        if !self.listening.get() {
            let source = self.source();
            if source.is_notifying() {
                CollectionChangedEventManager::add_listener(&source, self.this.clone());
            }
            self.listening.set(true);
        }
    }

    fn remove_listener_if_necessary(&self) {
        if self.listening.get() && self.collection_changed.is_empty() && self.post_collection_changed.is_empty() {
            let source = self.source();
            if source.is_notifying() {
                let listener: Weak<dyn ICollectionChangedListener> = self.this.clone();
                CollectionChangedEventManager::remove_listener(&source, &listener);
            }
            self.listening.set(false);
        }
    }

    #[inline]
    fn invoke(handlers: &HandlerList<ItemsChangedHandler>, e: &ItemsChangedEventArgs<'_>) {
        if !handlers.is_empty() {
            for (_, handler) in handlers.snapshot().iter() {
                handler(e);
            }
        }
    }
}

impl ICollectionChangedListener for ItemsSourceView {
    fn pre_changed(&self, e: &ItemsChangedEventArgs<'_>) {
        Self::invoke(&self.pre_collection_changed, e);
    }

    fn changed(&self, e: &ItemsChangedEventArgs<'_>) {
        Self::invoke(&self.collection_changed, e);
    }

    fn post_changed(&self, e: &ItemsChangedEventArgs<'_>) {
        Self::invoke(&self.post_collection_changed, e);
    }
}

/// A typed [`ItemsSourceView`]: the items are read as `T`.
///
/// With `T` = `BoxedValue` the items are read untyped.
pub struct ItemsSourceViewOf<T: PropertyValue> {
    view: Rc<ItemsSourceView>,
    marker: PhantomData<fn() -> T>,
}

impl<T: PropertyValue> Clone for ItemsSourceViewOf<T> {
    fn clone(&self) -> Self {
        Self { view: self.view.clone(), marker: PhantomData }
    }
}

impl<T: PropertyValue> Deref for ItemsSourceViewOf<T> {
    type Target = ItemsSourceView;

    fn deref(&self) -> &ItemsSourceView {
        &self.view
    }
}

impl<T: PropertyValue> ItemsSourceViewOf<T> {
    /// Gets an empty view.
    pub fn empty() -> Self {
        Self { view: ItemsSourceView::empty(), marker: PhantomData }
    }

    /// Initializes a new instance of the view for the specified data source.
    pub(crate) fn new(source: ItemsSource) -> Self {
        Self { view: ItemsSourceView::new(source), marker: PhantomData }
    }

    /// Reads an existing view as a typed view over the same source.
    pub fn from_view(view: &ItemsSourceView) -> Self {
        Self::new(view.source())
    }

    /// The untyped view.
    pub fn view(&self) -> &Rc<ItemsSourceView> {
        &self.view
    }

    /// Retrieves the item at the specified index: `None` for a null item.
    /// Panics if the index is out of range or the item is not a `T`.
    pub fn get_at(&self, index: usize) -> Option<T> {
        let item = self.view.get_at(index);
        if item.is_none() {
            return None;
        }
        match unbox_item::<T>(&item) {
            Some(value) => Some(value),
            None => panic!("Unable to cast the item to type '{}'.", std::any::type_name::<T>()),
        }
    }

    /// The items, enumerated by index over the live collection.
    pub fn iter(&self) -> impl Iterator<Item = Option<T>> + '_ {
        let mut index = 0;
        std::iter::from_fn(move || {
            if index < self.view.count() {
                index += 1;
                Some(self.get_at(index - 1))
            } else {
                None
            }
        })
    }
}
