//! `ObservableListEx` (`List/ObservableListEx*.cs`): the operators of the sequences of change sets
//! of lists.
//!
//! As for caches (`ObservableCacheEx`), each upstream operator is a provided method from the macro
//! of its module in `ops/`, the trait is implemented for the sequences of change sets of any form
//! ([`IChangeSet`]), and an operator whose name is that of an operator of
//! `ferroui_reactive::ObservableExt` takes the suffix `_list` (`and_list`, `cast_list`,
//! `switch_list`, `count_list`, ...). The LINQ-named aliases of `ObservableListAlias` are not ported.

use super::{ChangeSet, IChangeSet};
use ferroui_reactive::ObservableExt;
use ferroui_system::IObservable;
use std::any::Any;
use std::rc::Rc;

/// The operators of lists (`ObservableListEx`).
pub trait ObservableListEx<T: Clone + PartialEq + 'static> {
    /// The sequence as plain change sets.
    fn change_sets(&self) -> Rc<dyn IObservable<ChangeSet<T>>>;

    super::ops::as_aggregator::as_aggregator_methods!();
    super::ops::as_observable_list::as_observable_list_methods!();
    super::ops::collect_update_stats::collect_update_stats_methods!();
    super::ops::not_empty::not_empty_methods!();
}

impl<T: Clone + PartialEq + 'static, C: IChangeSet<T>> ObservableListEx<T> for Rc<dyn IObservable<C>> {
    fn change_sets(&self) -> Rc<dyn IObservable<ChangeSet<T>>> {
        if let Some(plain) = (self as &dyn Any).downcast_ref::<Rc<dyn IObservable<ChangeSet<T>>>>() {
            return plain.clone();
        }
        self.select(|changes: C| changes.change_set().clone())
    }
}
