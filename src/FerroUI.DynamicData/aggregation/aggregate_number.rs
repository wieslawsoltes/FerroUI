//! The numbers of the aggregations: the value types of the overloads of `Sum` (`SumEx`) and `Avg`
//! (`AvgEx`).
//!
//! Upstream has one overload per numeric type (`int`, `long`, `double`, `decimal`, `float`) and
//! its nullable form. The port has one generic method each, over the traits of this module:
//! [`AggregateNumber`], implemented for `i32`, `i64`, `f32` and `f64`, and [`AggregateValue`], the
//! value a selector returns: a number, or an `Option` of one, whose `None` counts as zero
//! (upstream's `GetValueOrDefault()`, which keeps the item in the count of an average).
//!
//! The arithmetic is that of C#'s unchecked context: the integers wrap on overflow.
//!
//! Not ported: the `decimal` overloads - no decimal type (phase 1b).

use std::fmt::Debug;

/// A number that the aggregations add up and average (`int`, `long`, `float`, `double`).
pub trait AggregateNumber: Copy + PartialEq + PartialOrd + Debug + 'static {
    /// The type of an average: `f64` for `i32`, `i64` and `f64` (`double`), `f32` for `f32`
    /// (`float`), as upstream's overloads of `Avg` return.
    type Average: Copy + PartialEq + PartialOrd + Debug + 'static;

    /// Zero, the sum of no values.
    const ZERO: Self;

    /// `self + other`, wrapping on overflow for the integers.
    fn plus(self, other: Self) -> Self;

    /// `self - other`, wrapping on overflow for the integers.
    fn minus(self, other: Self) -> Self;

    /// The average of `count` values (at least one) whose sum is `sum`: `sum / (double)count` for
    /// the integers and `double`, `sum / count` in `float` for `float`.
    fn average(sum: Self, count: i32) -> Self::Average;

    /// The number as an average (the implicit conversion of upstream's `emptyValue`).
    fn to_average(self) -> Self::Average;
}

impl AggregateNumber for i32 {
    type Average = f64;
    const ZERO: i32 = 0;

    fn plus(self, other: i32) -> i32 {
        self.wrapping_add(other)
    }

    fn minus(self, other: i32) -> i32 {
        self.wrapping_sub(other)
    }

    fn average(sum: i32, count: i32) -> f64 {
        f64::from(sum) / f64::from(count)
    }

    fn to_average(self) -> f64 {
        f64::from(self)
    }
}

impl AggregateNumber for i64 {
    type Average = f64;
    const ZERO: i64 = 0;

    fn plus(self, other: i64) -> i64 {
        self.wrapping_add(other)
    }

    fn minus(self, other: i64) -> i64 {
        self.wrapping_sub(other)
    }

    fn average(sum: i64, count: i32) -> f64 {
        sum as f64 / f64::from(count)
    }

    fn to_average(self) -> f64 {
        self as f64
    }
}

impl AggregateNumber for f32 {
    type Average = f32;
    const ZERO: f32 = 0.0;

    fn plus(self, other: f32) -> f32 {
        self + other
    }

    fn minus(self, other: f32) -> f32 {
        self - other
    }

    fn average(sum: f32, count: i32) -> f32 {
        sum / count as f32
    }

    fn to_average(self) -> f32 {
        self
    }
}

impl AggregateNumber for f64 {
    type Average = f64;
    const ZERO: f64 = 0.0;

    fn plus(self, other: f64) -> f64 {
        self + other
    }

    fn minus(self, other: f64) -> f64 {
        self - other
    }

    fn average(sum: f64, count: i32) -> f64 {
        sum / f64::from(count)
    }

    fn to_average(self) -> f64 {
        self
    }
}

/// The value a selector of `Sum` or `Avg` returns: a number, or an `Option` of a number whose
/// `None` counts as zero (the nullable overloads, `valueSelector(t).GetValueOrDefault()`).
pub trait AggregateValue: Copy + 'static {
    /// The number the value counts as.
    type Number: AggregateNumber;

    /// The number the value counts as: the value itself, or zero for a `None`.
    fn number(self) -> Self::Number;
}

macro_rules! impl_aggregate_value {
    ($($ty:ty),*) => {
        $(
            impl AggregateValue for $ty {
                type Number = $ty;

                fn number(self) -> $ty {
                    self
                }
            }
        )*
    };
}

impl_aggregate_value!(i32, i64, f32, f64);

impl<X: AggregateNumber> AggregateValue for Option<X> {
    type Number = X;

    fn number(self) -> X {
        self.unwrap_or(X::ZERO)
    }
}
