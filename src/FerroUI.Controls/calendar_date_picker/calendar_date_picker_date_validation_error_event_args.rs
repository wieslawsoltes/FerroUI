// (c) Copyright Microsoft Corporation.
// This source is subject to the Microsoft Public License (Ms-PL).
// Please see https://go.microsoft.com/fwlink/?LinkID=131993 for details.
// All other rights reserved.
//
// Ported from the Silverlight Toolkit sources as adapted by the upstream
// project; the license text is in the `NOTICE.md` of this crate.

use std::cell::Cell;
use std::error::Error;
use std::rc::Rc;

/// Provides data for the `DateValidationError` event of the
/// [`CalendarDatePicker`](super::CalendarDatePicker).
///
/// A copy shares the state of the args it was made from, so that what a
/// handler decides reaches the control that raised the event.
#[derive(Clone)]
pub struct CalendarDatePickerDateValidationErrorEventArgs {
    throw_exception: Rc<Cell<bool>>,
    exception: Rc<dyn Error>,
    text: String,
}

impl CalendarDatePickerDateValidationErrorEventArgs {
    /// Initializes a new instance of the
    /// [`CalendarDatePickerDateValidationErrorEventArgs`] class.
    ///
    /// `exception` is the initial exception from the `DateValidationError`
    /// event and `text` the text that caused the event.
    pub fn new(exception: Rc<dyn Error>, text: impl Into<String>) -> Self {
        Self { throw_exception: Rc::new(Cell::new(false)), exception, text: text.into() }
    }

    /// Gets the initial exception associated with the
    /// `DateValidationError` event.
    pub fn exception(&self) -> &Rc<dyn Error> {
        &self.exception
    }

    /// Gets the text that caused the `DateValidationError` event.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Gets a value indicating whether the exception should be thrown: the
    /// control then panics with the message of the exception.
    ///
    /// If set to true, the control throws the exception when the text
    /// cannot be parsed to a date or the date cannot be selected. The
    /// default is false.
    pub fn throw_exception(&self) -> bool {
        self.throw_exception.get()
    }

    /// Sets a value indicating whether the exception should be thrown.
    /// (The args always have an exception, so the check of the reference
    /// for a missing one has no counterpart.)
    pub fn set_throw_exception(&self, value: bool) {
        self.throw_exception.set(value);
    }
}

impl PartialEq for CalendarDatePickerDateValidationErrorEventArgs {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.throw_exception, &other.throw_exception)
    }
}
