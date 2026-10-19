//! `DeferUntilLoaded<T>` (`List/Internal/DeferUntilLoaded.cs`).

use crate::cache::internal::status_monitor;
use crate::kernel::ConnectionStatus;
use crate::list::{ChangeSet, ObservableListEx};
use ferroui_reactive::ObservableExt;
use ferroui_system::IObservable;
use std::rc::Rc;

/// The change sets of `source` from its first one; empty change sets are left out
/// (`DeferUntilLoaded.Run`).
pub(crate) fn defer_until_loaded<T: Clone + PartialEq + 'static>(source: Rc<dyn IObservable<ChangeSet<T>>>) -> Rc<dyn IObservable<ChangeSet<T>>> {
    status_monitor(source.clone())
        .where_(|status: &ConnectionStatus| *status == ConnectionStatus::Loaded)
        .take(1)
        .select(|_| ChangeSet::<T>::empty())
        .concat(source)
        .not_empty()
}
