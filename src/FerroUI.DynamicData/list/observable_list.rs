//! `AnonymousObservableList<T>` (`List/Internal/AnonymousObservableList.cs`): a list that can be
//! observed and not edited, made of a sequence of change sets.

use super::{ChangeSet, IChangeSet, IObservableList, SourceList};
use ferroui_system::{IDisposable, IObservable};
use std::rc::Rc;

/// A read-only list of the items of a sequence of change sets (`AnonymousObservableList<T>`, the
/// result of `AsObservableList`).
pub struct ObservableList<T> {
    source_list: SourceList<T>,
}

impl<T> Clone for ObservableList<T> {
    fn clone(&self) -> Self {
        Self { source_list: self.source_list.clone() }
    }
}

impl<T: Clone + PartialEq + 'static> ObservableList<T> {
    /// The list of the items of `source` (`AnonymousObservableList(IObservable<IChangeSet<T>>)`).
    pub fn from_source<C: IChangeSet<T>>(source: Rc<dyn IObservable<C>>) -> Self {
        Self { source_list: SourceList::from_source(source) }
    }

    /// A read-only view of `list` (`AnonymousObservableList(ISourceList<T>)`).
    pub fn from_list(list: &SourceList<T>) -> Self {
        Self { source_list: list.clone() }
    }
}

impl<T: Clone + PartialEq + 'static> IObservableList<T> for ObservableList<T> {
    fn count(&self) -> usize {
        self.source_list.count()
    }

    fn count_changed(&self) -> Rc<dyn IObservable<usize>> {
        self.source_list.count_changed()
    }

    fn items(&self) -> Vec<T> {
        self.source_list.items()
    }

    fn connect_with(&self, predicate: Option<Rc<dyn Fn(&T) -> bool>>) -> Rc<dyn IObservable<ChangeSet<T>>> {
        self.source_list.connect_with(predicate)
    }

    fn preview_with(&self, predicate: Option<Rc<dyn Fn(&T) -> bool>>) -> Rc<dyn IObservable<ChangeSet<T>>> {
        self.source_list.preview_with(predicate)
    }
}

impl<T: Clone + PartialEq + 'static> IDisposable for ObservableList<T> {
    fn dispose(&self) {
        self.source_list.dispose();
    }
}
