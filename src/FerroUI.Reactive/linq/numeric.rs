//! The value types of the numeric aggregates `Sum` (`Linq/Observable/Sum.cs`), `Average`
//! (`Linq/Observable/Average.cs`), `Min` (`Linq/Observable/Min.cs`) and `Max`
//! (`Linq/Observable/Max.cs`).
//!
//! Upstream has one operator class per numeric type (`int`, `long`, `float`, `double`, `decimal`
//! and their nullable forms). The port has one generic operator each, over the traits of this
//! module: [`Summable`], [`Averageable`] and [`Extremum`], implemented for `i32`, `i64`, `f32`,
//! `f64` and `Option` of each (the nullable forms; [`Extremum`] also for the other types with a
//! total order of the standard library, the generic `Min<TSource>` and `Max<TSource>` with the
//! default comparer). `decimal` has no type in the port.
//!
//! A `None` of an `Option` sequence is upstream's `null`: `Sum`, `Average`, `Min` and `Max` ignore
//! it, and `Average`, `Min` and `Max` of an `Option` sequence without a `Some` give `None` where
//! the forms without `Option` end with [`SequenceError::NoElements`].

use super::aggregate::SequenceError;

/// A type `Sum` adds up (upstream's overloads of `Sum` for `int`, `long`, `float`, `double` and
/// their nullable forms). The sum has the type of the values; the sum of an `Option` sequence
/// ignores the `None` values and is always a `Some` (`0` for no values).
pub trait Summable: Sized + 'static {
    /// The running sum: the type itself, `f64` for `f32` (as LINQ to Objects, which accumulates a
    /// `float` sum into a `double`).
    type Sum: Copy + 'static;

    /// The sum of no values.
    const ZERO: Self::Sum;

    /// `sum` plus `value` (`sum` itself for a `None`), or `None` when the sum does not fit (the
    /// `checked` addition of `int` and `long`, which throws `OverflowException`).
    fn add(sum: Self::Sum, value: Self) -> Option<Self::Sum>;

    /// The sum as the value of the sequence.
    fn total(sum: Self::Sum) -> Self;
}

impl Summable for i32 {
    type Sum = i32;
    const ZERO: i32 = 0;

    fn add(sum: i32, value: i32) -> Option<i32> {
        sum.checked_add(value)
    }

    fn total(sum: i32) -> i32 {
        sum
    }
}

impl Summable for i64 {
    type Sum = i64;
    const ZERO: i64 = 0;

    fn add(sum: i64, value: i64) -> Option<i64> {
        sum.checked_add(value)
    }

    fn total(sum: i64) -> i64 {
        sum
    }
}

impl Summable for f32 {
    type Sum = f64;
    const ZERO: f64 = 0.0;

    fn add(sum: f64, value: f32) -> Option<f64> {
        Some(sum + f64::from(value))
    }

    fn total(sum: f64) -> f32 {
        sum as f32
    }
}

impl Summable for f64 {
    type Sum = f64;
    const ZERO: f64 = 0.0;

    fn add(sum: f64, value: f64) -> Option<f64> {
        Some(sum + value)
    }

    fn total(sum: f64) -> f64 {
        sum
    }
}

impl<X: Summable> Summable for Option<X> {
    type Sum = X::Sum;
    const ZERO: X::Sum = X::ZERO;

    fn add(sum: X::Sum, value: Option<X>) -> Option<X::Sum> {
        match value {
            Some(value) => X::add(sum, value),
            None => Some(sum),
        }
    }

    fn total(sum: X::Sum) -> Option<X> {
        Some(X::total(sum))
    }
}

/// A type `Average` averages (upstream's overloads of `Average` for `int`, `long`, `float`,
/// `double` and their nullable forms): the average of `i32` and `i64` values is an `f64` (their
/// sum is an `i64`, checked), of `f32` values an `f32` (their sum an `f64`), of `f64` values an
/// `f64`; the average of an `Option` sequence ignores the `None` values and is an `Option`.
pub trait Averageable: Sized + 'static {
    /// The running sum.
    type Sum: Copy + 'static;

    /// The average.
    type Output: 'static;

    /// The sum of no values.
    const ZERO: Self::Sum;

    /// `sum` plus `value`: `Ok(Some(sum))` when the value is counted, `Ok(None)` when it is
    /// ignored (a `None`), `Err(SequenceError::Overflow)` when the sum does not fit (the `checked`
    /// addition of upstream, which throws `OverflowException`).
    fn add(sum: Self::Sum, value: Self) -> Result<Option<Self::Sum>, SequenceError>;

    /// The average of `count` values (at least one) that add up to `sum`.
    fn average(sum: Self::Sum, count: i64) -> Self::Output;

    /// The value of a sequence without a counted value; `None` ends the sequence with
    /// [`SequenceError::NoElements`] (`InvalidOperationException`).
    fn empty() -> Option<Self::Output>;
}

impl Averageable for i32 {
    type Sum = i64;
    type Output = f64;
    const ZERO: i64 = 0;

    fn add(sum: i64, value: i32) -> Result<Option<i64>, SequenceError> {
        sum.checked_add(i64::from(value)).map(Some).ok_or(SequenceError::Overflow)
    }

    fn average(sum: i64, count: i64) -> f64 {
        sum as f64 / count as f64
    }

    fn empty() -> Option<f64> {
        None
    }
}

impl Averageable for i64 {
    type Sum = i64;
    type Output = f64;
    const ZERO: i64 = 0;

    fn add(sum: i64, value: i64) -> Result<Option<i64>, SequenceError> {
        sum.checked_add(value).map(Some).ok_or(SequenceError::Overflow)
    }

    fn average(sum: i64, count: i64) -> f64 {
        sum as f64 / count as f64
    }

    fn empty() -> Option<f64> {
        None
    }
}

impl Averageable for f32 {
    type Sum = f64;
    type Output = f32;
    const ZERO: f64 = 0.0;

    fn add(sum: f64, value: f32) -> Result<Option<f64>, SequenceError> {
        Ok(Some(sum + f64::from(value)))
    }

    fn average(sum: f64, count: i64) -> f32 {
        (sum / count as f64) as f32
    }

    fn empty() -> Option<f32> {
        None
    }
}

impl Averageable for f64 {
    type Sum = f64;
    type Output = f64;
    const ZERO: f64 = 0.0;

    fn add(sum: f64, value: f64) -> Result<Option<f64>, SequenceError> {
        Ok(Some(sum + value))
    }

    fn average(sum: f64, count: i64) -> f64 {
        sum / count as f64
    }

    fn empty() -> Option<f64> {
        None
    }
}

impl<X: Averageable> Averageable for Option<X> {
    type Sum = X::Sum;
    type Output = Option<X::Output>;
    const ZERO: X::Sum = X::ZERO;

    fn add(sum: X::Sum, value: Option<X>) -> Result<Option<X::Sum>, SequenceError> {
        match value {
            Some(value) => X::add(sum, value),
            None => Ok(None),
        }
    }

    fn average(sum: X::Sum, count: i64) -> Option<X::Output> {
        Some(X::average(sum, count))
    }

    fn empty() -> Option<Option<X::Output>> {
        Some(None)
    }
}

/// A type `Min` and `Max` compare with their default comparison (upstream's overloads for `int`,
/// `long`, `float`, `double` and their nullable forms, and `Min<TSource>()` and `Max<TSource>()`
/// with `Comparer<TSource>.Default`).
///
/// Implemented for the integer types, `bool`, `char`, `String`, `&'static str`, `()`, `Duration`,
/// `Instant` and `SystemTime` by their total order; for `f32` and `f64` by their order, a NaN
/// replacing the current value (as upstream's `MinDouble`, `MaxDouble`, ...); and for `Option` of
/// any of them, a `None` being ignored and a sequence without a `Some` giving `None` (upstream's
/// nullable forms and reference types). Other types implement the two comparisons, or use the
/// comparer forms (`min_with`, `max_with`).
pub trait Extremum: Sized + 'static {
    /// Whether `self` replaces `current` as the minimum (`self < current`).
    fn replaces_min(&self, current: &Self) -> bool;

    /// Whether `self` replaces `current` as the maximum (`self > current`).
    fn replaces_max(&self, current: &Self) -> bool;

    /// Whether the value is ignored (a `null` of upstream); false by default.
    fn is_null(&self) -> bool {
        false
    }

    /// The value of a sequence without a value that is not ignored; `None` (the default) ends the
    /// sequence with [`SequenceError::NoElements`] (`InvalidOperationException`).
    fn empty() -> Option<Self> {
        None
    }
}

macro_rules! impl_extremum_ord {
    ($($ty:ty),* $(,)?) => {
        $(
            impl Extremum for $ty {
                fn replaces_min(&self, current: &Self) -> bool {
                    self < current
                }

                fn replaces_max(&self, current: &Self) -> bool {
                    self > current
                }
            }
        )*
    };
}

impl_extremum_ord!(
    i8,
    i16,
    i32,
    i64,
    i128,
    isize,
    u8,
    u16,
    u32,
    u64,
    u128,
    usize,
    bool,
    char,
    String,
    &'static str,
    (),
    std::time::Duration,
    std::time::Instant,
    std::time::SystemTime,
);

macro_rules! impl_extremum_float {
    ($($ty:ty),*) => {
        $(
            impl Extremum for $ty {
                fn replaces_min(&self, current: &Self) -> bool {
                    *self < *current || self.is_nan()
                }

                fn replaces_max(&self, current: &Self) -> bool {
                    *self > *current || self.is_nan()
                }
            }
        )*
    };
}

impl_extremum_float!(f32, f64);

impl<X: Extremum> Extremum for Option<X> {
    fn replaces_min(&self, current: &Self) -> bool {
        match (self, current) {
            (Some(value), Some(current)) => value.replaces_min(current),
            _ => false,
        }
    }

    fn replaces_max(&self, current: &Self) -> bool {
        match (self, current) {
            (Some(value), Some(current)) => value.replaces_max(current),
            _ => false,
        }
    }

    fn is_null(&self) -> bool {
        self.is_none()
    }

    fn empty() -> Option<Self> {
        Some(None)
    }
}
