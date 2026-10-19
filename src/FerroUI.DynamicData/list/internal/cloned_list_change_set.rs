//! `ClonedListChangeSet<TObject>` (`List/Internal/ClonedListChangeSet.cs`): a sequence of change
//! sets with the list that they make, kept as they pass.

use crate::list::{clone_changes_with, ChangeSet};
use ferroui_reactive::ObservableExt;
use ferroui_system::IObservable;
use std::cell::RefCell;
use std::rc::Rc;

/// The sequence and the list it makes (`ClonedListChangeSet<TObject>`): equal only to itself, as
/// upstream's class.
pub(crate) struct ClonedListChangeSet<T: 'static> {
    list: Rc<RefCell<Vec<T>>>,
    source: Rc<dyn IObservable<ChangeSet<T>>>,
}

impl<T: 'static> Clone for ClonedListChangeSet<T> {
    fn clone(&self) -> Self {
        Self { list: self.list.clone(), source: self.source.clone() }
    }
}

impl<T: 'static> PartialEq for ClonedListChangeSet<T> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.list, &other.list)
    }
}

#[allow(dead_code)]
impl<T: Clone + PartialEq + 'static> ClonedListChangeSet<T> {
    /// The sequence of `source`, which applies each change set to the list, comparing the items by
    /// `equality_comparer` (`ClonedListChangeSet(IObservable<IChangeSet>, IEqualityComparer?)`).
    pub(crate) fn new(source: Rc<dyn IObservable<ChangeSet<T>>>, equality_comparer: Option<Rc<dyn Fn(&T, &T) -> bool>>) -> Self {
        let list = Rc::new(RefCell::new(Vec::new()));
        let target = list.clone();
        let equality_comparer: Rc<dyn Fn(&T, &T) -> bool> = equality_comparer.unwrap_or_else(|| Rc::new(|a: &T, b: &T| a == b));
        let source = source.do_(move |changes: &ChangeSet<T>| clone_changes_with(&mut *target.borrow_mut(), changes.changes(), &*equality_comparer));
        Self { list, source }
    }

    /// The items of the list (`List`).
    pub(crate) fn list(&self) -> Vec<T> {
        self.list.borrow().clone()
    }

    /// The list.
    pub(crate) fn list_cell(&self) -> &Rc<RefCell<Vec<T>>> {
        &self.list
    }

    /// The sequence (`Source`).
    pub(crate) fn source(&self) -> &Rc<dyn IObservable<ChangeSet<T>>> {
        &self.source
    }
}
