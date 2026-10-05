// (c) Copyright Microsoft Corporation.
// This source is subject to the Microsoft Public License (Ms-PL).
// Please see https://go.microsoft.com/fwlink/?LinkID=131993 for details.
// All other rights reserved.
//
// Ported from the Silverlight Toolkit sources as adapted by the upstream
// project; the license text is in the `NOTICE.md` of this crate.

/// Describes the available formatting options for the date of a
/// [`CalendarDatePicker`](super::CalendarDatePicker).
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CalendarDatePickerFormat {
    /// Specifies that the date should be displayed using unabbreviated
    /// days of the week and month names.
    Long = 0,

    /// Specifies that the date should be displayed using abbreviated days
    /// of the week and month names.
    Short = 1,

    /// Specifies that the date should be displayed using a custom format
    /// string.
    Custom = 2,
}
