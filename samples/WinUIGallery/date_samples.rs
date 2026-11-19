//! The code behind of the pinned DatePicker, TimePicker and CalendarView pages in Rust:
//! `OnNavigatedTo` of `DatePickerPage.xaml.cs`, the time of day the TimePicker page binds, and
//! `SelectionMode_SelectionChanged` of `CalendarViewPage.xaml.cs`. The CalendarDatePicker page
//! has no code-behind upstream. The pickers and the calendar own their input; the sample owns
//! only its values and options.

use ferroui_base::animation::SignedDuration;
use ferroui_base::utilities::{DateTime, DateTimeOffset};
use ferroui_base::Ref;
use ferroui_controls::primitives::SelectingItemsControl;
use ferroui_controls::{
    Calendar, CalendarSelectionMode, ComboBox, Control, DatePicker, TimePicker,
};

pub(crate) fn wire(page_id: &str, root: &Ref<Control>) {
    match page_id {
        "DatePicker" => wire_date_picker(root),
        "TimePicker" => wire_time_picker(root),
        "CalendarView" => wire_calendar_view(root),
        _ => {}
    }
}

/// `OnNavigatedTo` of `DatePickerPage.xaml.cs`: the second picker shows the date two months from
/// now, within the years from this one to five years from now.
fn wire_date_picker(root: &Ref<Control>) {
    let picker = root.get_control::<DatePicker>("Control2");
    let now = DateTimeOffset::now();
    picker.set_selected_date(Some(now.add_months(2)));
    picker.set_min_year(now);
    picker.set_max_year(now.add_years(5));
}

/// The source's `sys:DateTime.Now.TimeOfDay`: the time elapsed since midnight of today.
pub(crate) fn time_of_day(now: DateTime) -> SignedDuration {
    now.subtract_date_time(DateTime::new(now.year(), now.month(), now.day()))
}

/// The OneTime `{x:Bind sys:DateTime.Now.TimeOfDay}` of the two clock pickers of the TimePicker
/// page, set when the page is created.
fn wire_time_picker(root: &Ref<Control>) {
    let now = time_of_day(DateTime::now());
    for name in ["TwelveHourPicker", "TwentyFourHourPicker"] {
        root.get_control::<TimePicker>(name)
            .set_selected_time(Some(now));
    }
}

/// The selection mode of WinUI's `CalendarViewSelectionMode` name (None, Single, Multiple).
pub(crate) fn calendar_selection_mode(name: &str) -> Option<CalendarSelectionMode> {
    match name {
        "None" => Some(CalendarSelectionMode::None),
        "Single" => Some(CalendarSelectionMode::SingleDate),
        "Multiple" => Some(CalendarSelectionMode::MultipleRange),
        _ => None,
    }
}

/// `SelectionMode_SelectionChanged` of `CalendarViewPage.xaml.cs`: the selected name is the
/// selection mode of the calendar.
fn wire_calendar_view(root: &Ref<Control>) {
    let combo_box = root.get_control::<ComboBox>("selectionMode");
    let weak_combo_box = combo_box.downgrade();
    let weak_calendar = root.get_control::<Calendar>("Control1").downgrade();
    combo_box.property_changed(move |change| {
        if change.property() != SelectingItemsControl::selected_index_property().as_property() {
            return;
        }
        let (Some(combo_box), Some(calendar)) = (weak_combo_box.upgrade(), weak_calendar.upgrade())
        else {
            return;
        };
        let item = combo_box.selected_item();
        let name = item
            .as_ref()
            .and_then(|item| item.downcast_ref::<String>())
            .map(String::as_str)
            .unwrap_or_default();
        if let Some(mode) = calendar_selection_mode(name) {
            calendar.set_selection_mode(mode);
        }
    });
}
