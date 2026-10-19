//! `SumEx` (`Aggregation/SumEx.cs`): the running sum of the values of the items.
//!
//! The overloads of upstream for `int`, `long`, `double`, `float` and their nullable forms are one
//! generic method over [`AggregateValue`], whose sum has the type of the number (a `None` adds
//! zero). The operators are methods of [`AggregationCacheEx`](super::AggregationCacheEx)
//! (`sum_cache`), [`AggregationListEx`](super::AggregationListEx) (`sum_list`) and
//! [`AggregateChangeSetEx`](super::AggregateChangeSetEx) (`sum_aggregate`): `Sum` is the name of
//! an operator of `ferroui_reactive::ObservableExt`, which applies to the same sequences.
//!
//! Not ported: the `decimal` overloads - no decimal type (phase 1b).

use super::{accumulate, AggregateChangeSets, AggregateNumber, AggregateValue};
use ferroui_system::IObservable;
use std::rc::Rc;

/// `Sum<T>(IObservable<IAggregateChangeSet<T>>, Func<T, TValue>)`: the sum of the values of the
/// items after each change set; the integers wrap on overflow, as C#'s unchecked additions do.
pub(crate) fn sum<T: 'static, V: AggregateValue>(source: AggregateChangeSets<T>, value_selector: impl Fn(&T) -> V + 'static) -> Rc<dyn IObservable<V::Number>> {
    accumulate(
        source,
        <V::Number as AggregateNumber>::ZERO,
        move |item: &T| value_selector(item).number(),
        |current: V::Number, value: V::Number| current.plus(value),
        |current: V::Number, value: V::Number| current.minus(value),
    )
}
