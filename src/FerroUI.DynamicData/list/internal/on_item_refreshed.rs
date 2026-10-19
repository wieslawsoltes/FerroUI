//! `OnItemRefreshed<T>` (`List/Internal/OnItemRefreshed.cs`): runs an action for each item
//! refreshed, before the change set is forwarded.

use crate::list::{ChangeSet, ListChangeReason};
use ferroui_reactive::ObservableExt;
use ferroui_system::IObservable;
use std::rc::Rc;

/// `OnItemRefreshed.Create`.
pub(crate) fn on_item_refreshed<T: 'static>(source: Rc<dyn IObservable<ChangeSet<T>>>, refresh_action: Rc<dyn Fn(&T)>) -> Rc<dyn IObservable<ChangeSet<T>>> {
    source.do_(move |change_set: &ChangeSet<T>| {
        for change in change_set {
            if change.reason() == ListChangeReason::Refresh {
                refresh_action(change.item().current());
            }
        }
    })
}
