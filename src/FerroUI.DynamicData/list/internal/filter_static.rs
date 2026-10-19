//! `FilterStatic<T>` (`List/Internal/FilterStatic.cs`): the items of a list that a predicate
//! accepts, without reevaluation.

use crate::list::{clear_or_remove_many, remove_many, replace_or_add, ChangeAwareList, ChangeSet, IList, ListChangeReason};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::IObservable;
use std::cell::RefCell;
use std::rc::Rc;

pub(crate) fn filter_static<T: Clone + PartialEq + 'static>(source: Rc<dyn IObservable<ChangeSet<T>>>, predicate: Rc<dyn Fn(&T) -> bool>) -> Rc<dyn IObservable<ChangeSet<T>>> {
    Observable::defer(move || {
        let filtered = Rc::new(RefCell::new(ChangeAwareList::<T>::new()));
        let predicate = predicate.clone();
        source
            .select(move |changes: ChangeSet<T>| {
                let mut filtered = filtered.borrow_mut();
                process(&mut filtered, &changes, &*predicate);
                filtered.capture_changes()
            })
            .where_(|changes: &ChangeSet<T>| changes.count() != 0)
    })
}

/// `Process`: refreshes and moves are not followed, as upstream's static filter does not.
fn process<T: Clone + PartialEq>(filtered: &mut ChangeAwareList<T>, changes: &ChangeSet<T>, predicate: &dyn Fn(&T) -> bool) {
    for change in changes {
        match change.reason() {
            ListChangeReason::Add => {
                let item = change.item();
                if predicate(item.current()) {
                    filtered.add(item.current().clone());
                }
            }
            ListChangeReason::AddRange => {
                let matches: Vec<T> = change.range().iter().filter(|item| predicate(item)).cloned().collect();
                filtered.add_range(matches);
            }
            ListChangeReason::Replace => {
                let item = change.item();
                let previous = item.previous().expect("a replacement has a previous item");
                if predicate(item.current()) {
                    replace_or_add(filtered, previous, item.current().clone());
                } else {
                    IList::remove(filtered, previous);
                }
            }
            ListChangeReason::Remove => {
                IList::remove(filtered, change.item().current());
            }
            ListChangeReason::RemoveRange => remove_many(filtered, change.range().items()),
            ListChangeReason::Clear => clear_or_remove_many(filtered, change),
            ListChangeReason::Refresh | ListChangeReason::Moved => {}
        }
    }
}
