//! `TreeDataGridItemsSourceView.cs`: the view of a collection of models that the rows and the
//! selection read and listen to.
//!
//! Adapted, as the reference, from the WinUI project (<https://github.com/microsoft/microsoft-ui-xaml>),
//! MIT License, Copyright (c) Microsoft Corporation.

use crate::model_collection::{ModelCollection, Models, ModelsChangedArgs};
use crate::utils::{CollectionChangedEventManager, ICollectionChangedListener};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// A handler of [`TreeDataGridItemsSourceView::add_collection_changed`].
pub type ItemsChangedHandler<M> = dyn for<'a> Fn(&ModelsChangedArgs<'a, M>);

/// A standardized view of a collection of models: indexed access and its change notifications,
/// raised in the `changed` phase of [`CollectionChangedEventManager`].
pub struct TreeDataGridItemsSourceView<M: 'static> {
    weak_self: Weak<Self>,
    inner: RefCell<Option<Models<M>>>,
    handlers: RefCell<Vec<(u64, Rc<ItemsChangedHandler<M>>)>>,
    next_token: Cell<u64>,
    is_listening: Cell<bool>,
}

impl<M: 'static> TreeDataGridItemsSourceView<M> {
    /// A view of `source`.
    pub fn new(source: Models<M>) -> Rc<Self> {
        Rc::new_cyclic(|weak_self| Self {
            weak_self: weak_self.clone(),
            inner: RefCell::new(Some(source)),
            handlers: RefCell::new(Vec::new()),
            next_token: Cell::new(1),
            is_listening: Cell::new(false),
        })
    }

    /// A view of no models.
    pub fn empty() -> Rc<Self> {
        Self::new(Rc::new(Vec::<Rc<M>>::new()))
    }

    /// The view of `items`, or an empty view for `None` (`GetOrCreate`).
    pub fn get_or_create(items: Option<Models<M>>) -> Rc<Self> {
        match items {
            Some(items) => Self::new(items),
            None => Self::empty(),
        }
    }

    /// The collection the view shows.
    ///
    /// # Panics
    /// After [`dispose`](Self::dispose) (`ObjectDisposedException`).
    pub fn inner(&self) -> Models<M> {
        self.inner.borrow().clone().unwrap_or_else(|| panic!("Cannot access a disposed object. Object name: 'TreeDataGridItemsSourceView'."))
    }

    /// The number of models.
    pub fn count(&self) -> usize {
        self.inner().count()
    }

    /// The model at `index`.
    pub fn get_at(&self, index: usize) -> Rc<M> {
        self.inner().get(index)
    }

    /// The index of `model`, or -1.
    pub fn index_of(&self, model: &Rc<M>) -> i32 {
        self.inner().index_of(model).map_or(-1, |i| i as i32)
    }

    /// The models.
    pub fn iter(&self) -> impl Iterator<Item = Rc<M>> {
        let inner = self.inner();
        (0..inner.count()).map(move |i| inner.get(i))
    }

    /// `HasKeyIndexMapping`: not implemented by the reference either.
    pub fn has_key_index_mapping(&self) -> bool {
        false
    }

    /// Subscribes to the changes of the collection (`CollectionChanged`); the first handler makes the
    /// view a listener of the collection.
    pub fn add_collection_changed(&self, handler: Rc<ItemsChangedHandler<M>>) -> u64 {
        let inner = self.inner();
        if !self.is_listening.get() && inner.is_notifying() {
            CollectionChangedEventManager::add_listener(&inner, self.as_listener());
            self.is_listening.set(true);
        }
        let token = self.next_token.get();
        self.next_token.set(token + 1);
        self.handlers.borrow_mut().push((token, handler));
        token
    }

    /// Removes a handler of [`add_collection_changed`](Self::add_collection_changed); the last one
    /// stops listening to the collection.
    pub fn remove_collection_changed(&self, token: u64) {
        let inner = self.inner();
        let empty = {
            let mut handlers = self.handlers.borrow_mut();
            handlers.retain(|(t, _)| *t != token);
            handlers.is_empty()
        };
        if empty && self.is_listening.get() {
            CollectionChangedEventManager::remove_listener(&inner, &self.as_listener());
            self.is_listening.set(false);
        }
    }

    /// Whether the view has handlers of its changes.
    pub fn has_collection_changed_handlers(&self) -> bool {
        !self.handlers.borrow().is_empty()
    }

    /// Stops listening and releases the collection.
    pub fn dispose(&self) {
        if self.is_listening.get() {
            if let Some(inner) = self.inner.borrow().clone() {
                CollectionChangedEventManager::remove_listener(&inner, &self.as_listener());
            }
            self.is_listening.set(false);
        }
        *self.inner.borrow_mut() = None;
    }

    /// Adds another listener of the collection of the view (`AddListener`).
    pub fn add_listener(&self, listener: Weak<dyn ICollectionChangedListener<M>>) {
        CollectionChangedEventManager::add_listener(&self.inner(), listener);
    }

    /// Removes a listener added with [`add_listener`](Self::add_listener).
    pub fn remove_listener(&self, listener: &Weak<dyn ICollectionChangedListener<M>>) {
        CollectionChangedEventManager::remove_listener(&self.inner(), listener);
    }

    fn as_listener(&self) -> Weak<dyn ICollectionChangedListener<M>> {
        self.weak_self.clone() as Weak<dyn ICollectionChangedListener<M>>
    }

    /// Raises the view's change notification.
    pub fn on_items_source_changed(&self, args: &ModelsChangedArgs<'_, M>) {
        let handlers: Vec<Rc<ItemsChangedHandler<M>>> = self.handlers.borrow().iter().map(|(_, h)| h.clone()).collect();
        for handler in handlers {
            handler(args);
        }
    }
}

impl<M: 'static> ICollectionChangedListener<M> for TreeDataGridItemsSourceView<M> {
    fn changed(&self, _collection: &Models<M>, e: &ModelsChangedArgs<'_, M>) {
        self.on_items_source_changed(e);
    }
}
