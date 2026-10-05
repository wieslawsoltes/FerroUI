//! The date picker with a drop-down calendar and the types of its events.

#[allow(clippy::module_inception)]
mod calendar_date_picker;
mod calendar_date_picker_date_validation_error_event_args;
mod calendar_date_picker_format;

pub use calendar_date_picker::{
    CalendarDatePicker, CalendarDatePickerImpl, CalendarDatePickerImplExt, CalendarDatePickerVTable,
};
pub use calendar_date_picker_date_validation_error_event_args::CalendarDatePickerDateValidationErrorEventArgs;
pub use calendar_date_picker_format::CalendarDatePickerFormat;

#[cfg(test)]
mod calendar_date_picker_tests;
