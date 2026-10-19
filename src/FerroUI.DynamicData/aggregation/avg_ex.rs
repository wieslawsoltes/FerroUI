//! `AvgEx` (`Aggregation/AvgEx.cs`): the running average of the values of the items.
//!
//! The overloads of upstream for `int`, `long`, `double`, `float` and their nullable forms are the
//! generic methods `avg` and `avg_with` over [`AggregateValue`]; `emptyValue`, which defaults to
//! zero upstream, is the argument of `avg_with`. The operators themselves are methods of
//! [`AggregationCacheEx`](super::AggregationCacheEx), [`AggregationListEx`](super::AggregationListEx)
//! and [`AggregateChangeSetEx`](super::AggregateChangeSetEx).
//!
//! Not ported: the `decimal` overloads - no decimal type (phase 1b).

use super::{AggregateChangeSets, AggregateNumber, AggregateType, AggregateValue, Avg};
use ferroui_reactive::ObservableExt;
use ferroui_system::IObservable;
use std::rc::Rc;

/// `Avg<T>(IObservable<IAggregateChangeSet<T>>, Func<T, TValue>, TValue emptyValue)`: the average
/// of the values of the items after each change set, `empty_value` when there are no items. The
/// value of a `None` counts as zero, and its item counts (upstream's `GetValueOrDefault()`).
pub(crate) fn avg<T: 'static, V: AggregateValue>(
    source: AggregateChangeSets<T>,
    value_selector: impl Fn(&T) -> V + 'static,
    empty_value: V::Number,
) -> Rc<dyn IObservable<<V::Number as AggregateNumber>::Average>> {
    avg_calc(source, move |item: &T| value_selector(item).number(), empty_value.to_average())
}

/// `AvgCalc`: the count and the sum of the values, accumulated from zero, and their average.
fn avg_calc<T: 'static, N: AggregateNumber>(
    source: AggregateChangeSets<T>,
    value_selector: impl Fn(&T) -> N + 'static,
    fallback_value: N::Average,
) -> Rc<dyn IObservable<N::Average>> {
    source
        .scan_with(Avg::new(0, N::ZERO), move |state: Avg<N>, changes| {
            changes.items().fold(state, |current, aggregate_item| {
                let value = value_selector(aggregate_item.item());
                match aggregate_item.aggregate_type() {
                    AggregateType::Add => Avg::new(current.count() + 1, current.sum().plus(value)),
                    AggregateType::Remove => Avg::new(current.count() - 1, current.sum().minus(value)),
                }
            })
        })
        .select(move |values: Avg<N>| if values.count() == 0 { fallback_value } else { N::average(values.sum(), values.count()) })
}
