//! The `ArgumentOutOfRangeException.ThrowIfLessThan` and `ThrowIfGreaterThan` guards of the
//! runtime that the reference calls on its time arguments. Not from the reference: the helpers of
//! the runtime it relies on.
//!
//! Deviation (DEVIATIONS.md, Behaviors): the exception of the reference is a panic here, with the
//! message of the runtime: a negative duration or key times out of order are errors of the
//! caller, as they are in the reference.

use ferroui_base::animation::TimeSpan;

/// Panics when `value` is less than `other` (`ArgumentOutOfRangeException.ThrowIfLessThan`).
#[track_caller]
pub(crate) fn throw_if_less_than(value: TimeSpan, other: TimeSpan, param_name: &str) {
    if value < other {
        panic!(
            "{param_name} ('{value}') must be greater than or equal to '{other}'. (Parameter '{param_name}')\nActual value was {value}."
        );
    }
}

/// Panics when `value` is greater than `other` (`ArgumentOutOfRangeException.ThrowIfGreaterThan`).
#[track_caller]
pub(crate) fn throw_if_greater_than(value: TimeSpan, other: TimeSpan, param_name: &str) {
    if value > other {
        panic!(
            "{param_name} ('{value}') must be less than or equal to '{other}'. (Parameter '{param_name}')\nActual value was {value}."
        );
    }
}
