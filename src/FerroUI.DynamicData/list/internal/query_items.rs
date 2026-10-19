//! The body of `ToCollection` and `ToSortedCollection` of lists: `QueryWhenChanged`
//! (`List/Internal/QueryWhenChanged.cs`) with a selector of the items.

use crate::list::{clone_changes, ChangeSet};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IObservable, IObserver};
use std::cell::RefCell;
use std::rc::Rc;

/// The items of the list after each change set, chosen by `selector` (`new
/// QueryWhenChanged<T>(source).Run()` and `Select`): a list per subscription, cloned from each
/// change set.
///
/// As upstream's, the subscription forwards only the values: an error of the source has no handler
/// (and panics), and the end of the source is not forwarded.
pub(crate) fn query_items<T: Clone + PartialEq + 'static, R: 'static>(source: Rc<dyn IObservable<ChangeSet<T>>>, selector: Rc<dyn Fn(Vec<T>) -> R>) -> Rc<dyn IObservable<R>> {
    Observable::create(move |observer: Rc<dyn IObserver<R>>| {
        let list = RefCell::new(Vec::new());
        let selector = selector.clone();
        source.subscribe_next(move |changes: ChangeSet<T>| {
            let items = {
                let mut list = list.borrow_mut();
                clone_changes(&mut *list, &changes);
                list.clone()
            };
            observer.on_next(selector(items));
        })
    })
}
