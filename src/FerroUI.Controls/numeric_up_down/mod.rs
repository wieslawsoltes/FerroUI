//! The numeric up-down control: a text box with spin buttons that edits a
//! decimal number.

#[allow(clippy::module_inception)]
mod numeric_up_down;
mod numeric_up_down_value_changed_event_args;

pub use numeric_up_down::{NumericUpDown, NumericUpDownImpl, NumericUpDownImplExt, NumericUpDownVTable};
pub use numeric_up_down_value_changed_event_args::NumericUpDownValueChangedEventArgs;

#[cfg(test)]
mod numeric_up_down_tests;
