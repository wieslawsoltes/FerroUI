//! `OnChangeAction` (`Cache/ObservableCacheEx.OnChangeAction.cs`): runs an action for each change
//! that a predicate accepts, before the change set is forwarded. Upstream's methods are private;
//! `OnItemAdded`, `OnItemRefreshed`, `OnItemRemoved` and `OnItemUpdated` build on them.

use crate::cache::{Change, ChangeReason, ChangeSet};
use ferroui_reactive::ObservableExt;
use ferroui_system::IObservable;
use std::rc::Rc;

/// Runs `change_action` for each change that `predicate` accepts
/// (`OnChangeAction(Predicate<Change>, Action<Change>)`).
pub(crate) fn on_change_action<T: 'static, K: 'static>(
    source: Rc<dyn IObservable<ChangeSet<T, K>>>,
    predicate: impl Fn(&Change<T, K>) -> bool + 'static,
    change_action: impl Fn(&Change<T, K>) + 'static,
) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
    source.do_(move |changes: &ChangeSet<T, K>| {
        for change in changes {
            if !predicate(change) {
                continue;
            }
            change_action(change);
        }
    })
}

/// Runs `action` with the item and the key of each change of `reason`
/// (`OnChangeAction(ChangeReason, Action<TObject, TKey>)`).
pub(crate) fn on_change_reason<T: 'static, K: 'static>(
    source: Rc<dyn IObservable<ChangeSet<T, K>>>,
    reason: ChangeReason,
    action: impl Fn(&T, &K) + 'static,
) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
    on_change_action(source, move |change| change.reason() == reason, move |change| action(change.current(), change.key()))
}
