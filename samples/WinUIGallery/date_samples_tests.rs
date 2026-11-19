//! The DatePicker, TimePicker, CalendarDatePicker and CalendarView pages: the source's examples
//! with their code behind (`date_samples.rs`).

use crate::date_samples::{calendar_selection_mode, time_of_day};
use crate::{examples, tests::start};
use ferroui_base::animation::SignedDuration;
use ferroui_base::utilities::{DateTime, DateTimeOffset};
use ferroui_base::Ref;
use ferroui_controls::{
    Calendar, CalendarDatePicker, CalendarSelectionMode, ComboBox, Control, DatePicker, TimePicker,
};

fn page(id: &str) -> Ref<Control> {
    examples::create(examples::find(id).unwrap()).unwrap_or_else(|error| panic!("{id}: {error}"))
}

#[test]
fn date_picker_shows_two_months_from_now_within_five_years() {
    let _application = start();
    let root = page("DatePicker");
    assert_eq!(
        root.get_control::<DatePicker>("Control1").selected_date(),
        None
    );
    let picker = root.get_control::<DatePicker>("Control2");
    assert_eq!(picker.day_format(), "d (ddd)");
    assert!(!picker.year_visible());
    let now = DateTimeOffset::now();
    let date = picker.selected_date().unwrap();
    let expected = now.add_months(2);
    assert_eq!(
        (date.year(), date.month()),
        (expected.year(), expected.month())
    );
    assert_eq!(picker.min_year().year(), now.year());
    assert_eq!(picker.max_year().year(), now.year() + 5);
}

#[test]
fn time_pickers_keep_the_increment_the_clocks_and_the_time_of_day() {
    let _application = start();
    let root = page("TimePicker");
    assert_eq!(
        root.get_control::<TimePicker>("Control2")
            .minute_increment(),
        15
    );
    let twelve = root.get_control::<TimePicker>("TwelveHourPicker");
    let twenty_four = root.get_control::<TimePicker>("TwentyFourHourPicker");
    assert_eq!(twelve.clock_identifier(), "12HourClock");
    assert_eq!(twenty_four.clock_identifier(), "24HourClock");
    assert!(twelve.selected_time().is_some());
    assert_eq!(twelve.selected_time(), twenty_four.selected_time());
    assert_eq!(
        time_of_day(DateTime::new_with_time(2026, 11, 18, 13, 45, 30)),
        SignedDuration::from_ticks((13 * 3600 + 45 * 60 + 30) * 10_000_000)
    );
}

#[test]
fn calendar_date_picker_has_the_source_placeholder() {
    let _application = start();
    let root = page("CalendarDatePicker");
    let picker = root.get_control::<CalendarDatePicker>("Control1");
    assert_eq!(picker.placeholder_text().as_deref(), Some("Pick a date"));
    assert_eq!(picker.selected_date(), None);
}

#[test]
fn calendar_view_selection_mode_option_sets_the_calendar_mode() {
    let _application = start();
    let root = page("CalendarView");
    let calendar = root.get_control::<Calendar>("Control1");
    assert_eq!(calendar.selection_mode(), CalendarSelectionMode::SingleDate);
    let modes = root.get_control::<ComboBox>("selectionMode");
    modes.set_selected_index(2);
    assert_eq!(
        calendar.selection_mode(),
        CalendarSelectionMode::MultipleRange
    );
    modes.set_selected_index(0);
    assert_eq!(calendar.selection_mode(), CalendarSelectionMode::None);
    modes.set_selected_index(1);
    assert_eq!(calendar.selection_mode(), CalendarSelectionMode::SingleDate);
    assert_eq!(calendar_selection_mode("Range"), None);
}
