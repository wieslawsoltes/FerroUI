//! `StdDevEx` (`Aggregation/StdDevEx.cs`): the running sample standard deviation of the values of
//! the items.
//!
//! The overloads of upstream for `int`, `long`, `double` and `float` are one generic method over
//! [`StdDevValue`]; the deviation is a `double` for each. `fallbackValue`, the value for fewer than
//! two items, is an argument of every form (upstream's forms over aggregate change sets and of
//! `float` default it to zero). The operators are methods of
//! [`AggregationCacheEx`](super::AggregationCacheEx), [`AggregationListEx`](super::AggregationListEx)
//! and [`AggregateChangeSetEx`](super::AggregateChangeSetEx).
//!
//! Not ported: the `decimal` overloads - no decimal type (phase 1b).

use super::{AggregateChangeSets, AggregateType, StdDev};
use ferroui_reactive::ObservableExt;
use ferroui_system::IObservable;
use std::rc::Rc;

/// A value whose standard deviation the aggregations compute (`int`, `long`, `float`, `double`).
pub trait StdDevValue: Copy + 'static {
    /// The type of the sums of the values and of their squares: `i64` for `i32` and `i64` (upstream
    /// widens `int` to `long`), the type itself for `f32` and `f64`.
    type Moment: Copy + 'static;

    /// The sum of no values.
    const ZERO: Self::Moment;

    /// The value as a moment.
    fn moment(self) -> Self::Moment;

    /// `left + right`, wrapping on overflow for the integers (C#'s unchecked arithmetic).
    fn plus(left: Self::Moment, right: Self::Moment) -> Self::Moment;

    /// `left - right`, wrapping on overflow for the integers.
    fn minus(left: Self::Moment, right: Self::Moment) -> Self::Moment;

    /// `left * right`, wrapping on overflow for the integers.
    fn times(left: Self::Moment, right: Self::Moment) -> Self::Moment;

    /// The sample standard deviation of `count` values (at least two) whose sum is
    /// `sum_of_items` and the sum of whose squares is `sum_of_squares`.
    fn deviation(count: i32, sum_of_items: Self::Moment, sum_of_squares: Self::Moment) -> f64;

    /// The value as the result (the implicit conversion of upstream's `fallbackValue`).
    fn to_f64(self) -> f64;
}

/// `IntegralStdDev`: the moments cross-multiplied in exact integer arithmetic, so that values far
/// from zero, whose moments exceed the integers a `double` holds exactly, keep their variance.
///
/// Deviation (DEVIATIONS.md, DynamicData): the products are computed in `i128` where upstream uses
/// `decimal` (phase 1b), and the quotient of the sum of the squared deviations by
/// `count * (count - 1)` is its integral part plus its fraction in `f64` (exact under the same
/// condition as upstream's: the accumulators have not overflowed).
fn integral_std_dev(count: i32, sum_of_items: i64, sum_of_squares: i64) -> f64 {
    let count = i128::from(count);
    let sum_of_items = i128::from(sum_of_items);
    let scaled_sum_of_squared_deviations = i128::from(sum_of_squares) * count - sum_of_items * sum_of_items;
    let divisor = count * (count - 1);
    let quotient = scaled_sum_of_squared_deviations / divisor;
    let remainder = scaled_sum_of_squared_deviations % divisor;
    (quotient as f64 + remainder as f64 / divisor as f64).sqrt()
}

impl StdDevValue for i32 {
    type Moment = i64;
    const ZERO: i64 = 0;

    fn moment(self) -> i64 {
        i64::from(self)
    }

    fn plus(left: i64, right: i64) -> i64 {
        left.wrapping_add(right)
    }

    fn minus(left: i64, right: i64) -> i64 {
        left.wrapping_sub(right)
    }

    fn times(left: i64, right: i64) -> i64 {
        left.wrapping_mul(right)
    }

    fn deviation(count: i32, sum_of_items: i64, sum_of_squares: i64) -> f64 {
        integral_std_dev(count, sum_of_items, sum_of_squares)
    }

    fn to_f64(self) -> f64 {
        f64::from(self)
    }
}

impl StdDevValue for i64 {
    type Moment = i64;
    const ZERO: i64 = 0;

    fn moment(self) -> i64 {
        self
    }

    fn plus(left: i64, right: i64) -> i64 {
        left.wrapping_add(right)
    }

    fn minus(left: i64, right: i64) -> i64 {
        left.wrapping_sub(right)
    }

    fn times(left: i64, right: i64) -> i64 {
        left.wrapping_mul(right)
    }

    fn deviation(count: i32, sum_of_items: i64, sum_of_squares: i64) -> f64 {
        integral_std_dev(count, sum_of_items, sum_of_squares)
    }

    fn to_f64(self) -> f64 {
        self as f64
    }
}

impl StdDevValue for f64 {
    type Moment = f64;
    const ZERO: f64 = 0.0;

    fn moment(self) -> f64 {
        self
    }

    fn plus(left: f64, right: f64) -> f64 {
        left + right
    }

    fn minus(left: f64, right: f64) -> f64 {
        left - right
    }

    fn times(left: f64, right: f64) -> f64 {
        left * right
    }

    fn deviation(count: i32, sum_of_items: f64, sum_of_squares: f64) -> f64 {
        let count = f64::from(count);
        ((sum_of_squares - (sum_of_items * sum_of_items) / count) / (count - 1.0)).sqrt()
    }

    fn to_f64(self) -> f64 {
        self
    }
}

impl StdDevValue for f32 {
    type Moment = f32;
    const ZERO: f32 = 0.0;

    fn moment(self) -> f32 {
        self
    }

    fn plus(left: f32, right: f32) -> f32 {
        left + right
    }

    fn minus(left: f32, right: f32) -> f32 {
        left - right
    }

    fn times(left: f32, right: f32) -> f32 {
        left * right
    }

    /// The variance in `float`, as upstream computes it, and its square root in `double`.
    fn deviation(count: i32, sum_of_items: f32, sum_of_squares: f32) -> f64 {
        let variance = (sum_of_squares - (sum_of_items * sum_of_items) / count as f32) / (count - 1) as f32;
        f64::from(variance).sqrt()
    }

    fn to_f64(self) -> f64 {
        f64::from(self)
    }
}

/// `StdDev<T>(IObservable<IAggregateChangeSet<T>>, Func<T, TValue>, TValue fallbackValue)` with
/// `StdDevCalc`: the count, the sum and the sum of the squares of the values, accumulated from
/// zero, and the sample standard deviation; `fallback_value` for fewer than two items.
pub(crate) fn std_dev<T: 'static, V: StdDevValue>(source: AggregateChangeSets<T>, value_selector: impl Fn(&T) -> V + 'static, fallback_value: V) -> Rc<dyn IObservable<f64>> {
    let fallback_value = fallback_value.to_f64();
    source
        .scan_with(StdDev::new(0, V::ZERO, V::ZERO), move |state: StdDev<V::Moment>, changes| {
            changes.items().fold(state, |current, aggregate_item| {
                let item = value_selector(aggregate_item.item()).moment();
                let square = V::times(item, item);
                match aggregate_item.aggregate_type() {
                    AggregateType::Add => {
                        StdDev::new(current.count() + 1, V::plus(current.sum_of_items(), item), V::plus(current.sum_of_squares(), square))
                    }
                    AggregateType::Remove => {
                        StdDev::new(current.count() - 1, V::minus(current.sum_of_items(), item), V::minus(current.sum_of_squares(), square))
                    }
                }
            })
        })
        .select(move |values: StdDev<V::Moment>| {
            if values.count() < 2 {
                fallback_value
            } else {
                V::deviation(values.count(), values.sum_of_items(), values.sum_of_squares())
            }
        })
}
