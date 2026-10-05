// (c) Copyright Microsoft Corporation.
// This source is subject to the Microsoft Public License (Ms-PL).
// Please see https://go.microsoft.com/fwlink/?LinkID=131993 for details.
// All other rights reserved.
//
// Ported from the Silverlight Toolkit sources as adapted by the upstream
// project; the license text is in the `NOTICE.md` of this crate.

use ferroui_base::utilities::DateTime;
use std::rc::Rc;

/// Represents a range of dates in a [`Calendar`](crate::Calendar).
///
/// A range is a reference object: handles are `Rc<CalendarDateRange>` and
/// compare by identity.
#[derive(Debug)]
pub struct CalendarDateRange {
    start: DateTime,
    end: DateTime,
}

impl CalendarDateRange {
    /// Initializes a new instance of the [`CalendarDateRange`] class with a
    /// single date: `day` is the date to be represented by the range.
    pub fn new(day: DateTime) -> Rc<Self> {
        Rc::new(Self { start: day, end: day })
    }

    /// Initializes a new instance of the [`CalendarDateRange`] class with a
    /// range of dates: `start` is the start of the range to be represented
    /// and `end` the end of it.
    pub fn new_range(start: DateTime, end: DateTime) -> Rc<Self> {
        if DateTime::compare(end, start) >= 0 {
            Rc::new(Self { start, end })
        } else {
            // Always use the start for ranges on the same day
            Rc::new(Self { start, end: start })
        }
    }

    /// Gets the first date in the represented range.
    pub fn start(&self) -> DateTime {
        self.start
    }

    /// Gets the last date in the represented range.
    pub fn end(&self) -> DateTime {
        self.end
    }

    /// Returns true if any day in the given range is contained in the
    /// current range.
    pub(crate) fn contains_any(&self, range: &CalendarDateRange) -> bool {
        let start = DateTime::compare(self.start, range.start);

        // Check if any part of the supplied range is contained by this
        // range or if the supplied range completely covers this range.
        (start <= 0 && DateTime::compare(self.end, range.start) >= 0)
            || (start >= 0 && DateTime::compare(self.start, range.end) <= 0)
    }
}

impl PartialEq for CalendarDateRange {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
