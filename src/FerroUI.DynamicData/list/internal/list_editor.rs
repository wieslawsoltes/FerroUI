//! The list an edit of a `SourceList` receives: it borrows the list for each of its operations, so
//! that an edit started from inside another edit (a nested edit, `ReaderWriter.WriteNested`)
//! changes the same list.

use crate::list::{ChangeAwareList, IExtendedList, IList};
use std::cell::RefCell;

pub(crate) struct ListEditor<'a, T> {
    list: &'a RefCell<ChangeAwareList<T>>,
}

impl<'a, T> ListEditor<'a, T> {
    pub(crate) fn new(list: &'a RefCell<ChangeAwareList<T>>) -> Self {
        Self { list }
    }
}

impl<T: Clone + PartialEq> IList<T> for ListEditor<'_, T> {
    fn count(&self) -> usize {
        self.list.borrow().count()
    }

    fn get(&self, index: usize) -> T {
        self.list.borrow().get(index).clone()
    }

    fn set(&mut self, index: usize, item: T) {
        self.list.borrow_mut().set(index, item);
    }

    fn insert(&mut self, index: usize, item: T) {
        self.list.borrow_mut().insert(index, item);
    }

    fn remove_at(&mut self, index: usize) {
        self.list.borrow_mut().remove_at(index);
    }

    fn clear(&mut self) {
        self.list.borrow_mut().clear();
    }

    fn add(&mut self, item: T) {
        self.list.borrow_mut().add(item);
    }

    fn index_of(&self, item: &T) -> Option<usize> {
        self.list.borrow().index_of(item)
    }

    fn remove(&mut self, item: &T) -> bool {
        self.list.borrow_mut().remove(item)
    }

    fn add_range(&mut self, items: Vec<T>) {
        self.list.borrow_mut().add_range(items);
    }

    fn add_or_insert_range(&mut self, items: Vec<T>, index: Option<usize>) {
        IList::add_or_insert_range(&mut *self.list.borrow_mut(), items, index);
    }

    fn remove_range(&mut self, index: usize, count: usize) {
        self.list.borrow_mut().remove_range(index, count);
    }

    fn supports_ranges(&self) -> bool {
        true
    }

    fn move_item(&mut self, original: usize, destination: usize) {
        IExtendedList::move_index(&mut *self.list.borrow_mut(), original, destination);
    }

    fn refresh_at(&mut self, index: usize, _item: T) {
        self.list.borrow_mut().refresh_at(index);
    }
}

impl<T: Clone + PartialEq> IExtendedList<T> for ListEditor<'_, T> {
    fn insert_range(&mut self, items: Vec<T>, index: usize) {
        IExtendedList::insert_range(&mut *self.list.borrow_mut(), items, index);
    }

    fn move_index(&mut self, original: usize, destination: usize) {
        IExtendedList::move_index(&mut *self.list.borrow_mut(), original, destination);
    }

}
