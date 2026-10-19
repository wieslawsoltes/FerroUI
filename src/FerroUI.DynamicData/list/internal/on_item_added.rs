//! `OnItemAdded<T>` (`List/Internal/OnItemAdded.cs`): runs an action for each item added, a
//! replacement included, before the change set is forwarded.

use crate::list::{ChangeSet, ListChangeReason};
use ferroui_reactive::ObservableExt;
use ferroui_system::IObservable;
use std::rc::Rc;

/// `OnItemAdded.Create`.
pub(crate) fn on_item_added<T: 'static>(source: Rc<dyn IObservable<ChangeSet<T>>>, add_action: Rc<dyn Fn(&T)>) -> Rc<dyn IObservable<ChangeSet<T>>> {
    source.do_(move |change_set: &ChangeSet<T>| {
        for change in change_set {
            match change.reason() {
                ListChangeReason::Add | ListChangeReason::Replace => add_action(change.item().current()),
                ListChangeReason::AddRange => {
                    for item in change.range() {
                        add_action(item);
                    }
                }
                _ => {}
            }
        }
    })
}
