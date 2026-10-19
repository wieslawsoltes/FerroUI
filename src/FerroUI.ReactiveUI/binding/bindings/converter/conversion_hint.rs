//! The conversion hints that the built-in converters read: an `int` (a width or a number of
//! decimals) and a `string` (a format string).

use std::any::Any;

/// The hint as an `i32` (`case int value`).
pub(crate) fn hint_as_i32(conversion_hint: Option<&dyn Any>) -> Option<i32> {
    conversion_hint?.downcast_ref::<i32>().copied()
}

/// The hint as text (`case string format`): a `String` or a `&'static str`.
pub(crate) fn hint_as_str(conversion_hint: Option<&dyn Any>) -> Option<&str> {
    let hint = conversion_hint?;
    hint.downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| hint.downcast_ref::<&'static str>().copied())
}
