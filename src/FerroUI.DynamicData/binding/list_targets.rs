//! The lists of .NET that the binding operators fill as an `IList<T>`: `ObservableCollection<T>`
//! and `List<T>` (`ListEx` branches on these types upstream).

use crate::list::IList;
use ferroui_system::collections::object_model::ObservableCollection;
use std::cell::RefCell;
use std::rc::Rc;

/// An `ObservableCollection<T>` as a list of DynamicData: its changes raise its notifications, and
/// moves use its `Move` (`ListEx.Clone`, `SortExtensions.Move`).
impl<T: Clone + PartialEq + 'static> IList<T> for ObservableCollection<T> {
    fn count(&self) -> usize {
        ObservableCollection::count(self)
    }

    fn get(&self, index: usize) -> T {
        ObservableCollection::get(self, index)
    }

    fn set(&mut self, index: usize, item: T) {
        ObservableCollection::set(self, index, item);
    }

    fn insert(&mut self, index: usize, item: T) {
        ObservableCollection::insert(self, index, item);
    }

    fn remove_at(&mut self, index: usize) {
        ObservableCollection::remove_at(self, index);
    }

    fn clear(&mut self) {
        ObservableCollection::clear(self);
    }

    fn add(&mut self, item: T) {
        ObservableCollection::add(self, item);
    }

    fn index_of(&self, item: &T) -> Option<usize> {
        ObservableCollection::index_of(self, item)
    }

    fn remove(&mut self, item: &T) -> bool {
        ObservableCollection::remove(self, item)
    }

    fn move_item(&mut self, original: usize, destination: usize) {
        ObservableCollection::move_item(self, original, destination);
    }
}

/// A shared `List<T>` (the reference of .NET) as a list of DynamicData, with the ranges of
/// `List<T>`.
impl<T: Clone + PartialEq + 'static> IList<T> for Rc<RefCell<Vec<T>>> {
    fn count(&self) -> usize {
        self.borrow().len()
    }

    fn get(&self, index: usize) -> T {
        self.borrow()[index].clone()
    }

    fn set(&mut self, index: usize, item: T) {
        self.borrow_mut()[index] = item;
    }

    fn insert(&mut self, index: usize, item: T) {
        self.borrow_mut().insert(index, item);
    }

    fn remove_at(&mut self, index: usize) {
        self.borrow_mut().remove(index);
    }

    fn clear(&mut self) {
        self.borrow_mut().clear();
    }

    fn index_of(&self, item: &T) -> Option<usize> {
        self.borrow().iter().position(|candidate| candidate == item)
    }

    fn add_range(&mut self, items: Vec<T>) {
        self.borrow_mut().extend(items);
    }

    fn add_or_insert_range(&mut self, items: Vec<T>, index: Option<usize>) {
        let mut list = self.borrow_mut();
        match index {
            Some(index) => {
                list.splice(index..index, items);
            }
            None => list.extend(items),
        }
    }

    fn remove_range(&mut self, index: usize, count: usize) {
        self.borrow_mut().drain(index..index + count);
    }

    fn supports_ranges(&self) -> bool {
        true
    }
}
