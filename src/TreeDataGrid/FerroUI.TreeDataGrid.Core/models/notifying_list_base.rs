//! `Models/NotifyingListBase.cs`: a notifying list with batch insert, remove and reset, which
//! raise one notification for a batch. The base of the column lists.

use super::NotifyingBase;
use ferroui_base::collections::{NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const INDEXER_NAME: &str = "Item[]";

/// A handler of [`NotifyingListBase::add_collection_changed`].
pub type ListChangedHandler<T> = dyn for<'a> Fn(&NotifyCollectionChangedEventArgs<'a, T>);

#[derive(Clone, Copy, PartialEq, Eq)]
enum BatchUpdateType {
    None,
    Insert,
    Remove,
    Reset,
}

/// A list that raises `CollectionChanged` and `PropertyChanged` (`Count`, `Item[]`).
pub struct NotifyingListBase<T: Clone + 'static> {
    items: RefCell<Vec<T>>,
    handlers: RefCell<Vec<(u64, Rc<ListChangedHandler<T>>)>>,
    next_token: Cell<u64>,
    notifying: NotifyingBase,
    busy: Cell<u32>,
    batch_update: Cell<BatchUpdateType>,
}

impl<T: Clone + 'static> Default for NotifyingListBase<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone + 'static> NotifyingListBase<T> {
    pub fn new() -> Self {
        Self {
            items: RefCell::new(Vec::new()),
            handlers: RefCell::new(Vec::new()),
            next_token: Cell::new(1),
            notifying: NotifyingBase::new(),
            busy: Cell::new(0),
            batch_update: Cell::new(BatchUpdateType::None),
        }
    }

    /// The `PropertyChanged` of the list.
    pub fn notifying(&self) -> &NotifyingBase {
        &self.notifying
    }

    pub fn add_collection_changed(&self, handler: Rc<ListChangedHandler<T>>) -> u64 {
        let token = self.next_token.get();
        self.next_token.set(token + 1);
        self.handlers.borrow_mut().push((token, handler));
        token
    }

    pub fn remove_collection_changed(&self, token: u64) -> bool {
        let mut handlers = self.handlers.borrow_mut();
        let before = handlers.len();
        handlers.retain(|(t, _)| *t != token);
        handlers.len() != before
    }

    pub fn count(&self) -> usize {
        self.items.borrow().len()
    }

    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }

    pub fn get(&self, index: usize) -> T {
        self.items.borrow()[index].clone()
    }

    pub fn try_get(&self, index: usize) -> Option<T> {
        self.items.borrow().get(index).cloned()
    }

    pub fn to_vec(&self) -> Vec<T> {
        self.items.borrow().clone()
    }

    /// Reads the items without copying them; the closure may not change the list.
    pub fn with_items<R>(&self, f: impl FnOnce(&[T]) -> R) -> R {
        f(&self.items.borrow())
    }

    pub fn position(&self, mut predicate: impl FnMut(&T) -> bool) -> Option<usize> {
        self.items.borrow().iter().position(|item| predicate(item))
    }

    pub fn add(&self, item: T) {
        let index = self.count();
        self.insert(index, item);
    }

    /// Inserts the items `action` hands to its argument, from `index`, with one notification
    /// (`InsertRange(int, Action<Action<T>>)`).
    pub fn insert_range(&self, index: usize, action: impl FnOnce(&mut dyn FnMut(T))) {
        self.check_reentrancy();
        self.batch_update.set(BatchUpdateType::Insert);
        let mut i = index;
        {
            let mut insert = |item: T| {
                self.insert_item(i, item);
                i += 1;
            };
            action(&mut insert);
        }
        self.batch_update.set(BatchUpdateType::None);
        if i > index {
            self.raise_count_and_indexer();
            let added: Vec<T> = self.items.borrow()[index..i].to_vec();
            self.raise(NotifyCollectionChangedAction::Add, &added, &[], index as i32, -1);
        }
    }

    /// Removes `count` items from `index` with one notification.
    pub fn remove_range(&self, index: usize, count: usize) {
        if count == 0 {
            return;
        }
        self.check_reentrancy();
        self.batch_update.set(BatchUpdateType::Remove);
        let removed: Vec<T> = self.items.borrow()[index..index + count].to_vec();
        for _ in 0..count {
            self.remove_item(index);
        }
        self.batch_update.set(BatchUpdateType::None);
        self.raise_count_and_indexer();
        self.raise(NotifyCollectionChangedAction::Remove, &[], &removed, -1, index as i32);
    }

    /// Changes the list in `action` and raises one reset notification.
    pub fn reset(&self, action: impl FnOnce(&Self)) {
        self.check_reentrancy();
        self.batch_update.set(BatchUpdateType::Reset);
        action(self);
        self.batch_update.set(BatchUpdateType::None);
        self.raise_count_and_indexer();
        self.raise(NotifyCollectionChangedAction::Reset, &[], &[], -1, -1);
    }

    pub fn clear(&self) {
        self.check_reentrancy();
        self.items.borrow_mut().clear();
        match self.batch_update.get() {
            BatchUpdateType::None => {
                self.raise_count_and_indexer();
                self.raise(NotifyCollectionChangedAction::Reset, &[], &[], -1, -1);
            }
            BatchUpdateType::Reset => {}
            _ => panic!("Operation not permitted during a batch update."),
        }
    }

    pub fn remove_at(&self, index: usize) {
        self.remove_item(index);
    }

    /// Removes the first item for which `predicate` holds; false when there is none.
    pub fn remove_where(&self, predicate: impl FnMut(&T) -> bool) -> bool {
        match self.position(predicate) {
            Some(index) => {
                self.remove_at(index);
                true
            }
            None => false,
        }
    }

    fn remove_item(&self, index: usize) {
        self.check_reentrancy();
        let removed = self.items.borrow_mut().remove(index);
        match self.batch_update.get() {
            BatchUpdateType::None => {
                self.raise_count_and_indexer();
                self.raise(NotifyCollectionChangedAction::Remove, &[], std::slice::from_ref(&removed), -1, index as i32);
            }
            BatchUpdateType::Reset | BatchUpdateType::Remove => {}
            _ => panic!("Operation not permitted during a batch update."),
        }
    }

    pub fn insert(&self, index: usize, item: T) {
        self.insert_item(index, item);
    }

    fn insert_item(&self, index: usize, item: T) {
        self.check_reentrancy();
        self.items.borrow_mut().insert(index, item.clone());
        match self.batch_update.get() {
            BatchUpdateType::None => {
                self.raise_count_and_indexer();
                self.raise(NotifyCollectionChangedAction::Add, std::slice::from_ref(&item), &[], index as i32, -1);
            }
            BatchUpdateType::Reset | BatchUpdateType::Insert => {}
            _ => panic!("Operation not permitted during a batch update."),
        }
    }

    pub fn set(&self, index: usize, item: T) {
        self.check_reentrancy();
        let original = std::mem::replace(&mut self.items.borrow_mut()[index], item.clone());
        match self.batch_update.get() {
            BatchUpdateType::None => {
                self.notifying.raise_property_changed(INDEXER_NAME);
                self.raise(
                    NotifyCollectionChangedAction::Replace,
                    std::slice::from_ref(&item),
                    std::slice::from_ref(&original),
                    index as i32,
                    index as i32,
                );
            }
            BatchUpdateType::Reset => {}
            _ => panic!("Operation not permitted during a batch update."),
        }
    }

    pub fn move_item(&self, old_index: usize, new_index: usize) {
        self.check_reentrancy();
        let removed = {
            let mut items = self.items.borrow_mut();
            let removed = items.remove(old_index);
            items.insert(new_index, removed.clone());
            removed
        };
        match self.batch_update.get() {
            BatchUpdateType::None => {
                self.notifying.raise_property_changed(INDEXER_NAME);
                self.raise(
                    NotifyCollectionChangedAction::Move,
                    std::slice::from_ref(&removed),
                    std::slice::from_ref(&removed),
                    new_index as i32,
                    old_index as i32,
                );
            }
            BatchUpdateType::Reset => {}
            _ => panic!("Operation not permitted during a batch update."),
        }
    }

    fn raise_count_and_indexer(&self) {
        self.notifying.raise_property_changed("Count");
        self.notifying.raise_property_changed(INDEXER_NAME);
    }

    fn raise(&self, action: NotifyCollectionChangedAction, new_items: &[T], old_items: &[T], new_index: i32, old_index: i32) {
        let handlers: Vec<Rc<ListChangedHandler<T>>> = self.handlers.borrow().iter().map(|(_, h)| h.clone()).collect();
        if handlers.is_empty() {
            return;
        }
        let args = NotifyCollectionChangedEventArgs {
            action,
            new_items,
            old_items,
            new_starting_index: new_index,
            old_starting_index: old_index,
        };
        self.busy.set(self.busy.get() + 1);
        struct Guard<'a>(&'a Cell<u32>);
        impl Drop for Guard<'_> {
            fn drop(&mut self) {
                self.0.set(self.0.get() - 1);
            }
        }
        let _guard = Guard(&self.busy);
        for handler in handlers {
            handler(&args);
        }
    }

    /// Changes are allowed while a notification is raised only when one handler listens: a
    /// re-entrant change would make the arguments of later handlers invalid.
    pub fn check_reentrancy(&self) {
        if self.busy.get() > 0 && self.handlers.borrow().len() > 1 {
            panic!("NotifyingList reentrancy not allowed.");
        }
    }
}
