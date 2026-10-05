//! The calendar, the parts of its template and the collections of its dates.

#[allow(clippy::module_inception)]
mod calendar;
mod calendar_blackout_dates_collection;
mod calendar_button;
mod calendar_date_range;
mod calendar_day_button;
mod calendar_extensions;
mod calendar_item;
mod date_time_helper;
mod selected_dates_collection;

pub use calendar::{
    Calendar, CalendarDateChangedEventArgs, CalendarMode, CalendarModeChangedEventArgs, CalendarSelectionMode,
};
pub use calendar_blackout_dates_collection::CalendarBlackoutDatesCollection;
pub use calendar_button::CalendarButton;
pub use calendar_date_range::CalendarDateRange;
pub use calendar_day_button::CalendarDayButton;
pub(crate) use calendar_extensions::CalendarExtensions;
pub(crate) use calendar_item::date_of;
pub use calendar_item::CalendarItem;
pub(crate) use date_time_helper::DateTimeHelper;
pub use selected_dates_collection::SelectedDatesCollection;

#[cfg(test)]
mod calendar_tests;
