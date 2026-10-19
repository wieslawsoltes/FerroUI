//! `OnItemRemoved<T>` (`List/Internal/OnItemRemoved.cs`): runs an action for each item removed, a
//! replaced one included, before the change set is forwarded; and, unless told otherwise, for every
//! item left when the subscription ends.

use crate::list::{clone_changes, ChangeSet, ListChangeReason};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IObservable, IObserver};
use std::cell::RefCell;
use std::rc::Rc;

/// `OnItemRemoved.Create`.
pub(crate) fn on_item_removed<T: Clone + PartialEq + 'static>(
    source: Rc<dyn IObservable<ChangeSet<T>>>,
    remove_action: Rc<dyn Fn(&T)>,
    invoke_on_unsubscribe: bool,
) -> Rc<dyn IObservable<ChangeSet<T>>> {
    let action = remove_action.clone();
    let removal_processor = source.do_(move |change_set: &ChangeSet<T>| {
        for change in change_set {
            match change.reason() {
                ListChangeReason::Clear | ListChangeReason::RemoveRange => {
                    for item in change.range() {
                        action(item);
                    }
                }
                ListChangeReason::Remove => action(change.item().current()),
                ListChangeReason::Replace => action(change.item().previous().expect("a replacement has a previous item")),
                _ => {}
            }
        }
    });
    if !invoke_on_unsubscribe {
        return removal_processor;
    }
    Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<T>>>| {
        let items = Rc::new(RefCell::new(Vec::<T>::new()));
        let (target, left, remove_action) = (items.clone(), items, remove_action.clone());
        removal_processor
            .do_(move |change_set: &ChangeSet<T>| clone_changes(&mut *target.borrow_mut(), change_set))
            .finally(move || {
                let left = left.borrow().clone();
                for item in &left {
                    remove_action(item);
                }
            })
            .subscribe_safe(observer)
    })
}
