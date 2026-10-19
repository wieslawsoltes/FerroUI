//! `QueryWhenChanged<T>` (`List/Internal/QueryWhenChanged.cs`).

use crate::list::{clone_changes, ChangeSet};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IObservable, IObserver};
use std::cell::RefCell;
use std::rc::Rc;

/// `QueryWhenChanged.Run`: a copy of the items after each change set (upstream's
/// `ReadOnlyCollectionLight<T>`). As upstream's, the subscription takes only the values: an error
/// panics, and the end is not passed on.
pub(crate) fn query_when_changed<T: Clone + PartialEq + 'static>(source: Rc<dyn IObservable<ChangeSet<T>>>) -> Rc<dyn IObservable<Vec<T>>> {
    Observable::create(move |observer: Rc<dyn IObserver<Vec<T>>>| {
        let list: RefCell<Vec<T>> = RefCell::new(Vec::new());
        source.subscribe_next(move |changes: ChangeSet<T>| {
            let snapshot = {
                let mut list = list.borrow_mut();
                clone_changes(&mut *list, &changes);
                list.clone()
            };
            observer.on_next(snapshot);
        })
    })
}
