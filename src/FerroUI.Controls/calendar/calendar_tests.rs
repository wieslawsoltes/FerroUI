//! Port of the reference `CalendarTests`.

use super::{
    Calendar, CalendarDateRange, CalendarDayButton, CalendarItem, CalendarMode, CalendarSelectionMode, DateTimeHelper,
};
use crate::templates::{FuncControlTemplate, FuncTemplate, FuncTemplateNameScopeExtensions, ITemplateOf};
use crate::test_support::{test_scope, TestRoot};
use crate::{ContentControl, Control, Grid, Panel, SelectionChangedEventArgs, TextBlock};
use ferroui_base::input::{
    KeyModifiers, Pointer, PointerPointProperties, PointerPressedEventArgs, PointerType, PointerUpdateKind,
    RawInputModifiers,
};
use ferroui_base::interactivity::Interactive;
use ferroui_base::utilities::{CalendarWeekRule, CultureInfo, DateTime, DayOfWeek, GregorianCalendar};
use ferroui_base::{BoxedValue, Point, Ref, Visual};
use std::cell::{Cell, RefCell};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

fn compare_dates(first: DateTime, second: DateTime) -> bool {
    first.year() == second.year() && first.month() == second.month() && first.day() == second.day()
}

/// Runs an action that the reference expects to throw and returns the
/// message of the panic that stands for the exception.
fn panic_message(action: impl FnOnce()) -> String {
    let payload = catch_unwind(AssertUnwindSafe(action)).expect_err("the action panics");
    payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| payload.downcast_ref::<&str>().map(|message| message.to_string()))
        .unwrap_or_default()
}

/// Asserts that an action fails the way the reference fails with an
/// exception for an argument that is out of range.
fn assert_argument_out_of_range(action: impl FnOnce()) {
    let message = panic_message(action);
    assert!(message.contains("is not valid."), "{message}");
}

/// Asserts that an action fails the way the reference fails with an
/// exception for an invalid operation.
fn assert_invalid_operation(action: impl FnOnce()) {
    let message = panic_message(action);
    assert!(
        message.contains("The SelectedDates collection can be changed only in a multiple selection mode")
            || message.contains("The SelectedDate property cannot be set when the selection mode is None"),
        "{message}"
    );
}

fn today() -> DateTime {
    DateTime::today()
}

fn days_from_today(days: i32) -> DateTime {
    DateTime::today().add_days(days as f64)
}

#[test]
fn selected_dates_changed_should_fire_when_selected_date_set() {
    let _scope = test_scope();
    let handled = Rc::new(Cell::new(false));
    let calendar = Calendar::new();
    calendar.set_selection_mode(CalendarSelectionMode::SingleDate);
    let _subscription = calendar.selected_dates_changed({
        let handled = handled.clone();
        move |_| handled.set(true)
    });
    let value = DateTime::new(2000, 10, 10);
    calendar.set_selected_date(Some(value));
    assert!(handled.get());
}

#[test]
fn display_date_changed_should_fire_when_display_date_set() {
    let _scope = test_scope();
    let handled = Rc::new(Cell::new(false));
    let calendar = Calendar::new();
    calendar.set_selection_mode(CalendarSelectionMode::SingleDate);
    let _subscription = calendar.display_date_changed({
        let handled = handled.clone();
        move |_| handled.set(true)
    });
    let value = DateTime::new(2000, 10, 10);
    calendar.set_display_date(value);
    assert!(handled.get());
}

#[test]
fn setting_selected_date_to_blackout_date_should_throw() {
    let _scope = test_scope();
    let calendar = Calendar::new();
    calendar.blackout_dates().add_dates_in_past();

    assert_argument_out_of_range(|| calendar.set_selected_date(Some(days_from_today(-1))));
}

#[test]
fn setting_selected_date_to_blackout_date_should_throw_range() {
    let _scope = test_scope();
    let calendar = Calendar::new();
    calendar.blackout_dates().add(CalendarDateRange::new_range(today(), days_from_today(10)));

    calendar.set_selected_date(Some(days_from_today(-1)));
    assert!(compare_dates(calendar.selected_date().unwrap(), days_from_today(-1)));
    assert!(compare_dates(calendar.selected_date().unwrap(), calendar.selected_dates().get(0)));

    calendar.set_selected_date(Some(days_from_today(11)));
    assert!(compare_dates(calendar.selected_date().unwrap(), days_from_today(11)));
    assert!(compare_dates(calendar.selected_date().unwrap(), calendar.selected_dates().get(0)));

    assert_argument_out_of_range(|| calendar.set_selected_date(Some(days_from_today(5))));
}

#[test]
fn adding_blackout_dates_containing_selected_date_should_throw() {
    let _scope = test_scope();
    let calendar = Calendar::new();
    calendar.set_selected_date(Some(days_from_today(5)));

    assert_argument_out_of_range(|| {
        calendar.blackout_dates().add(CalendarDateRange::new_range(today(), days_from_today(10)))
    });
}

#[test]
fn display_date_start_end_should_constrain_display_date() {
    let _scope = test_scope();
    let calendar = Calendar::new();
    calendar.set_selection_mode(CalendarSelectionMode::SingleDate);
    calendar.set_display_date_start(Some(DateTime::new(2005, 12, 30)));

    let value = DateTime::new(2005, 12, 15);
    calendar.set_display_date(value);
    assert!(compare_dates(calendar.display_date(), calendar.display_date_start().unwrap()));

    let value = DateTime::new(2005, 12, 30);
    calendar.set_display_date(value);
    assert!(compare_dates(calendar.display_date(), value));

    let value = DateTime::MAX_VALUE;
    calendar.set_display_date(value);
    assert!(compare_dates(calendar.display_date(), value));

    calendar.set_display_date_end(Some(DateTime::new(2010, 12, 30)));
    assert!(compare_dates(calendar.display_date(), calendar.display_date_end().unwrap()));
}

#[test]
fn setting_display_date_end_should_alter_dispaly_date_and_display_date_start() {
    let _scope = test_scope();
    let calendar = Calendar::new();
    let value = DateTime::new(2000, 1, 30);

    calendar.set_display_date(value);
    calendar.set_display_date_end(Some(value));
    calendar.set_display_date_start(Some(value));
    assert!(compare_dates(calendar.display_date_start().unwrap(), value));
    assert!(compare_dates(calendar.display_date_end().unwrap(), value));

    let value = value.add_months(2);
    calendar.set_display_date_start(Some(value));
    assert!(compare_dates(calendar.display_date_start().unwrap(), value));
    assert!(compare_dates(calendar.display_date_end().unwrap(), value));
    assert!(compare_dates(calendar.display_date(), value));
}

#[test]
fn display_date_range_end_will_contain_selected_date() {
    let _scope = test_scope();
    let calendar = Calendar::new();
    calendar.set_selection_mode(CalendarSelectionMode::SingleDate);

    calendar.set_selected_date(Some(DateTime::MAX_VALUE));
    assert!(compare_dates(calendar.selected_date().unwrap(), DateTime::MAX_VALUE));

    calendar.set_display_date_end(Some(DateTime::MAX_VALUE.add_days(-1.0)));
    assert!(compare_dates(calendar.display_date_end().unwrap(), DateTime::MAX_VALUE));
}

/// What the tests record of the `SelectedDatesChanged` event.
#[derive(Default)]
struct SelectedDatesChangedTracker {
    /// The days added to the SelectedDates collection.
    added_days: RefCell<Option<Vec<Option<BoxedValue>>>>,
    /// The days removed from the SelectedDates collection.
    removed_days: RefCell<Option<Vec<Option<BoxedValue>>>>,
    /// The number of times the SelectedDatesChanged event has been fired.
    count: Cell<i32>,
}

impl SelectedDatesChangedTracker {
    /// Handle the SelectedDatesChanged event.
    fn on_selected_dates_changed(&self, e: &SelectionChangedEventArgs) {
        *self.added_days.borrow_mut() = Some(e.added_items().to_vec());
        *self.removed_days.borrow_mut() = Some(e.removed_items().to_vec());
        self.count.set(self.count.get() + 1);
    }

    /// Clear the variables used to track the SelectedDatesChanged event.
    fn reset(&self) {
        if let Some(added_days) = self.added_days.borrow_mut().as_mut() {
            added_days.clear();
        }

        if let Some(removed_days) = self.removed_days.borrow_mut().as_mut() {
            removed_days.clear();
        }

        self.count.set(0);
    }

    fn added_count(&self) -> usize {
        self.added_days.borrow().as_ref().expect("the event has been raised").len()
    }

    fn removed_count(&self) -> usize {
        self.removed_days.borrow().as_ref().expect("the event has been raised").len()
    }
}

#[test]
fn single_date_selection_behavior() {
    let _scope = test_scope();
    let tracker = Rc::new(SelectedDatesChangedTracker::default());
    tracker.reset();
    let calendar = Calendar::new();
    let _subscription = calendar.selected_dates_changed({
        let tracker = tracker.clone();
        move |e| tracker.on_selected_dates_changed(e)
    });
    calendar.set_selection_mode(CalendarSelectionMode::SingleDate);
    calendar.set_selected_date(Some(today()));
    assert!(compare_dates(calendar.selected_date().unwrap(), today()));
    assert!(calendar.selected_dates().count() == 1);
    assert!(compare_dates(calendar.selected_dates().get(0), today()));
    assert!(tracker.count.get() == 1);
    assert!(tracker.added_days.borrow().is_some());
    assert!(tracker.added_count() == 1);
    assert!(tracker.removed_days.borrow().is_some());
    assert!(tracker.removed_count() == 0);
    tracker.reset();

    calendar.set_selected_date(Some(today()));
    assert!(compare_dates(calendar.selected_date().unwrap(), today()));
    assert!(calendar.selected_dates().count() == 1);
    assert!(compare_dates(calendar.selected_dates().get(0), today()));
    assert!(tracker.count.get() == 0);

    calendar.clear_value(Calendar::selected_date_property());

    calendar.set_selection_mode(CalendarSelectionMode::None);
    assert!(calendar.selected_dates().count() == 0);
    assert!(calendar.selected_date().is_none());

    calendar.set_selection_mode(CalendarSelectionMode::SingleDate);

    calendar.selected_dates().add(days_from_today(1));
    assert!(compare_dates(calendar.selected_date().unwrap(), days_from_today(1)));
    assert!(calendar.selected_dates().count() == 1);

    assert_invalid_operation(|| calendar.selected_dates().add(days_from_today(2)));
}

#[test]
fn single_range_selection_behavior() {
    let _scope = test_scope();
    let tracker = Rc::new(SelectedDatesChangedTracker::default());
    tracker.reset();
    let calendar = Calendar::new();
    let _subscription = calendar.selected_dates_changed({
        let tracker = tracker.clone();
        move |e| tracker.on_selected_dates_changed(e)
    });
    calendar.set_selection_mode(CalendarSelectionMode::SingleRange);
    calendar.set_selected_date(Some(today()));
    assert!(compare_dates(calendar.selected_date().unwrap(), today()));
    assert!(calendar.selected_dates().count() == 1);
    assert!(compare_dates(calendar.selected_dates().get(0), today()));
    assert!(tracker.count.get() == 1);
    assert!(tracker.added_days.borrow().is_some());
    assert!(tracker.added_count() == 1);
    assert!(tracker.removed_days.borrow().is_some());
    assert!(tracker.removed_count() == 0);
    tracker.reset();

    calendar.selected_dates().clear();
    assert!(calendar.selected_date().is_none());
    tracker.reset();

    calendar.selected_dates().add_range(today(), days_from_today(10));
    assert!(compare_dates(calendar.selected_date().unwrap(), today()));
    assert!(calendar.selected_dates().count() == 11);
    tracker.reset();

    calendar.selected_dates().add_range(today(), days_from_today(10));
    assert!(calendar.selected_dates().count() == 11);
    assert!(tracker.count.get() == 0);
    tracker.reset();

    calendar.selected_dates().add_range(days_from_today(-20), today());
    assert!(compare_dates(calendar.selected_date().unwrap(), days_from_today(-20)));
    assert!(calendar.selected_dates().count() == 21);
    assert!(tracker.count.get() == 1);
    assert!(tracker.added_count() == 21);
    assert!(tracker.removed_count() == 11);
    tracker.reset();

    calendar.selected_dates().add(days_from_today(100));
    assert!(compare_dates(calendar.selected_date().unwrap(), days_from_today(100)));
    assert!(calendar.selected_dates().count() == 1);
}

#[test]
fn allow_tap_range_selection_should_disable_tap_to_select_range() {
    let _scope = test_scope();
    let calendar = Calendar::new();
    assert!(calendar.allow_tap_range_selection()); // Default should be true

    calendar.set_allow_tap_range_selection(false);
    assert!(!calendar.allow_tap_range_selection());
}

#[test]
fn tap_range_selection_should_work_in_single_range_mode() {
    let _scope = test_scope();
    let calendar = Calendar::new();
    calendar.set_selection_mode(CalendarSelectionMode::SingleRange);
    calendar.set_allow_tap_range_selection(true);

    let start_date = DateTime::new(2023, 10, 10);
    let end_date = DateTime::new(2023, 10, 15);

    // First tap should select start date
    let first_tap_result = calendar.process_tap_range_selection(start_date);
    assert!(first_tap_result);
    assert_eq!(1, calendar.selected_dates().count());
    assert!(calendar.selected_dates().contains(start_date));

    // Second tap should complete the range
    let second_tap_result = calendar.process_tap_range_selection(end_date);
    assert!(second_tap_result);
    assert_eq!(6, calendar.selected_dates().count()); // 5 days inclusive
    assert!(calendar.selected_dates().contains(start_date));
    assert!(calendar.selected_dates().contains(end_date));
}

#[test]
fn tap_range_selection_should_not_work_in_single_date_mode() {
    let _scope = test_scope();
    let calendar = Calendar::new();
    calendar.set_selection_mode(CalendarSelectionMode::SingleDate);
    calendar.set_allow_tap_range_selection(true);

    let date = DateTime::new(2023, 10, 10);
    let result = calendar.process_tap_range_selection(date);
    assert!(!result); // Should not handle tap range selection
}

#[test]
fn tap_range_selection_should_handle_blackout_dates() {
    let _scope = test_scope();
    let calendar = Calendar::new();
    calendar.set_selection_mode(CalendarSelectionMode::SingleRange);
    calendar.set_allow_tap_range_selection(true);

    let start_date = DateTime::new(2023, 10, 10);
    let blackout_date = DateTime::new(2023, 10, 12);
    let end_date = DateTime::new(2023, 10, 15);

    // Add blackout date in the middle
    calendar.blackout_dates().add(CalendarDateRange::new_range(blackout_date, blackout_date));

    // First tap
    calendar.process_tap_range_selection(start_date);
    assert_eq!(1, calendar.selected_dates().count());

    // Second tap should restart selection due to blackout date
    calendar.process_tap_range_selection(end_date);
    assert_eq!(1, calendar.selected_dates().count());
    assert!(calendar.selected_dates().contains(end_date));
    assert!(!calendar.selected_dates().contains(start_date));
}

#[test]
fn tap_range_selection_should_handle_reverse_order_dates() {
    let _scope = test_scope();
    let calendar = Calendar::new();
    calendar.set_selection_mode(CalendarSelectionMode::SingleRange);
    calendar.set_allow_tap_range_selection(true);

    let later_date = DateTime::new(2023, 10, 15);
    let earlier_date = DateTime::new(2023, 10, 10);

    // First tap on later date
    calendar.process_tap_range_selection(later_date);
    assert_eq!(1, calendar.selected_dates().count());

    // Second tap on earlier date should still create correct range
    calendar.process_tap_range_selection(earlier_date);
    assert_eq!(6, calendar.selected_dates().count());
    assert!(calendar.selected_dates().contains(earlier_date));
    assert!(calendar.selected_dates().contains(later_date));
}

fn day_button(date: DateTime) -> Ref<CalendarDayButton> {
    let button = CalendarDayButton::new();
    button.set_data_context(Some(Rc::new(date) as BoxedValue));
    button
}

#[test]
fn calendar_item_should_reset_mouse_down_flag_on_detach_from_visual_tree() {
    let _scope = test_scope();
    let calendar = Calendar::new();
    calendar.set_selection_mode(CalendarSelectionMode::SingleDate);

    let calendar_item = CalendarItem::new();
    calendar_item.set_owner(Some(&calendar));

    // Attach CalendarItem to a visual tree
    let root = TestRoot::with_child(calendar_item.clone());

    // Create a day button and simulate mouse left button down,
    // which sets the internal mouse left button down flag to true.
    let date1 = DateTime::new(2024, 1, 15);
    let day_button1 = day_button(date1);

    let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
    let props = PointerPointProperties::new(RawInputModifiers::LEFT_MOUSE_BUTTON, PointerUpdateKind::LeftButtonPressed);
    let root_visual: Ref<Visual> = root.clone().upcast();
    let press_args = PointerPressedEventArgs::new(
        day_button1.clone().upcast::<Interactive>(),
        pointer,
        &root_visual,
        Point::default(),
        0,
        props,
        KeyModifiers::NONE,
        1,
    );

    calendar_item.cell_mouse_left_button_down(&day_button1, &press_args);

    // date1 should now be selected
    assert_eq!(1, calendar.selected_dates().count());
    assert_eq!(date1, calendar.selected_dates().get(0));

    // Detach CalendarItem from visual tree (simulates popup closing
    // during date selection without a pointer released event).
    root.set_child(None::<Ref<Control>>);

    // Create a different day button and simulate mouse enter.
    // Before the fix, the mouse left button down flag would still be true,
    // causing hover to auto-select dates.
    let date2 = DateTime::new(2024, 1, 20);
    let day_button2 = day_button(date2);

    calendar_item.cell_mouse_entered(&day_button2);

    // The selected date should NOT have changed to date2,
    // because the mouse-down flag was reset when detaching.
    assert_eq!(1, calendar.selected_dates().count());
    assert_eq!(date1, calendar.selected_dates().get(0));
}

#[test]
fn calendar_item_should_reset_year_view_mouse_down_flag_on_detach_from_visual_tree() {
    let _scope = test_scope();
    let calendar = Calendar::new();
    let calendar_item = CalendarItem::new();
    calendar_item.set_owner(Some(&calendar));

    // Attach CalendarItem to a visual tree
    let root = TestRoot::with_child(calendar_item.clone());

    // Set the year view mouse down flag directly, since the handler of the
    // mouse down of a month button is private.
    calendar_item.set_is_mouse_left_button_down_year_view(true);
    assert!(calendar_item.is_mouse_left_button_down_year_view());

    // Detach CalendarItem from visual tree
    root.set_child(None::<Ref<Control>>);

    // Verify the flag was reset
    assert!(!calendar_item.is_mouse_left_button_down_year_view());
}

// --- Week number tests ---

#[test]
fn is_week_number_visible_defaults_to_false() {
    let _scope = test_scope();
    let calendar = Calendar::new();
    assert!(!calendar.is_week_number_visible());
}

#[test]
fn week_number_rule_defaults_to_culture_calendar_week_rule() {
    let _scope = test_scope();
    let calendar = Calendar::new();
    let rule: CalendarWeekRule = calendar.week_number_rule();
    assert_eq!(CultureInfo::current_culture().date_time_format().calendar_week_rule(), rule);
}

#[test]
fn is_week_number_visible_can_be_set() {
    let _scope = test_scope();
    let calendar = Calendar::new();
    calendar.set_is_week_number_visible(true);
    assert!(calendar.is_week_number_visible());
}

#[test]
fn week_number_rule_can_be_set() {
    let _scope = test_scope();
    let calendar = Calendar::new();
    calendar.set_week_number_rule(CalendarWeekRule::FirstFourDayWeek);
    assert_eq!(CalendarWeekRule::FirstFourDayWeek, calendar.week_number_rule());
}

#[test]
fn get_week_of_year_returns_correct_week_number() {
    let rows = [
        // ISO 8601: week 1 of 2023 starts on Monday 2 Jan 2023
        (2023, 1, 2, CalendarWeekRule::FirstFourDayWeek, DayOfWeek::Monday, 1),
        // 2022-12-31 is still in ISO week 52 of 2022
        (2022, 12, 31, CalendarWeekRule::FirstFourDayWeek, DayOfWeek::Monday, 52),
        // 2018-12-31 is a Monday and is ISO week 1 of 2019, not week 53 of 2018
        (2018, 12, 31, CalendarWeekRule::FirstFourDayWeek, DayOfWeek::Monday, 1),
        // US rule: week 1 always starts on Jan 1
        (2023, 1, 1, CalendarWeekRule::FirstDay, DayOfWeek::Sunday, 1),
        (2023, 12, 31, CalendarWeekRule::FirstDay, DayOfWeek::Sunday, 53),
    ];

    for (year, month, day, rule, first_day_of_week, expected_week) in rows {
        let calendar = GregorianCalendar::new();
        let date = DateTime::new(year, month, day);
        let week = DateTimeHelper::get_week_of_year(date, rule, first_day_of_week, &calendar);
        assert_eq!(expected_week, week, "{year}-{month}-{day} {rule:?} {first_day_of_week:?}");
    }
}

// ------------------------------------------------------------------------
//  ISO-week fallback tests - verify that the week of the year is the
//  expected week number for edge-case dates.
// ------------------------------------------------------------------------
#[test]
fn get_week_of_year_iso_week_fallback_returns_correct_week() {
    let rows = [
        // 31 Dec 2018 is part of ISO-week 1 of 2019
        (2018, 12, 31, CalendarWeekRule::FirstFourDayWeek, DayOfWeek::Monday, 1),
        // 1 Jan 2020 is also ISO-week 1 (Monday)
        (2020, 1, 1, CalendarWeekRule::FirstFourDayWeek, DayOfWeek::Monday, 1),
        // 29 Dec 2014 (Monday) should be week 1 of 2015 (ISO)
        (2014, 12, 29, CalendarWeekRule::FirstFourDayWeek, DayOfWeek::Monday, 1),
        // 30 Dec 2019 (Monday) is week 1 of 2020 (ISO)
        (2019, 12, 30, CalendarWeekRule::FirstFourDayWeek, DayOfWeek::Monday, 1),
    ];

    for (year, month, day, rule, first_day, expected_week) in rows {
        let calendar = GregorianCalendar::new();
        let date = DateTime::new(year, month, day);
        // The helper is the internal helper used by the calendar.
        // It falls back to ISO-week calculation when the week rule of the
        // culture does not produce a valid week.
        let week = DateTimeHelper::get_week_of_year(date, rule, first_day, &calendar);
        assert_eq!(expected_week, week, "{year}-{month}-{day}");
    }
}

/// The calendar item among the visual descendants of a calendar.
fn calendar_item_of(calendar: &Calendar) -> Option<Ref<CalendarItem>> {
    calendar.get_visual_descendants().find_map(|x| x.cast::<CalendarItem>())
}

/// The first content control of a grid that is in the given row.
fn label_in_row(grid: &Grid, row: i32) -> Ref<ContentControl> {
    grid.children()
        .snapshot()
        .iter()
        .filter_map(|x| x.cast::<ContentControl>())
        .find(|x| Grid::get_row(x) == row)
        .expect("the row has a label")
}

fn content_number(label: &ContentControl) -> Option<i32> {
    label.content().and_then(|content| content.downcast_ref::<i32>().copied())
}

// ------------------------------------------------------------------------
//  Property change refresh tests - ensure the control updates its
//  visual state when week-number related properties are changed.
// ------------------------------------------------------------------------
#[test]
fn changing_is_week_number_visible_toggles_has_week_numbers_pseudo_class() {
    let _scope = test_scope();
    let calendar = create_test_calendar();

    calendar.apply_template();
    calendar.set_display_mode(CalendarMode::Month);
    calendar.set_is_week_number_visible(false);

    // Grab the CalendarItem from the visual tree
    let calendar_item = calendar_item_of(&calendar);
    assert!(calendar_item.is_some());
    let calendar_item = calendar_item.unwrap();

    calendar_item.apply_template();

    // Assert pseudo-class is NOT set
    assert!(!calendar_item.classes().contains(":hasweeknumbers"));

    // Turn on week numbers
    calendar.set_is_week_number_visible(true);

    // Assert pseudo-class IS set
    assert!(calendar_item.classes().contains(":hasweeknumbers"));
}

#[test]
fn changing_week_number_rule_refreshes_week_number_labels() {
    let _scope = test_scope();
    let calendar = create_test_calendar();

    // Apply template so that the internal CalendarItem is created
    calendar.apply_template();

    calendar.set_display_mode(CalendarMode::Month);
    calendar.set_is_week_number_visible(true);
    // Use ISO-rule (FirstFourDayWeek) for the test
    calendar.set_week_number_rule(CalendarWeekRule::FirstFourDayWeek);
    calendar.set_first_day_of_week(DayOfWeek::Monday);
    calendar.set_display_date(DateTime::new(2021, 1, 4)); // First Monday of 2021

    // Grab the CalendarItem from the visual tree
    let calendar_item = calendar_item_of(&calendar);
    assert!(calendar_item.is_some());
    let calendar_item = calendar_item.unwrap();

    calendar_item.apply_template();

    // The first week-number label should display "1" for the first week of 2021
    let week_labels_grid = calendar_item
        .get_visual_descendants()
        .filter_map(|x| x.cast::<Grid>())
        .find(|x| x.name().as_deref() == Some("PART_ElementWeekNumberLabels"));
    assert!(week_labels_grid.is_some());
    let week_labels_grid = week_labels_grid.unwrap();
    let first_label = label_in_row(&week_labels_grid, 2);
    assert_eq!(Some(1), content_number(&first_label));

    // Change the rule to use FirstDay (which for 2021-01-04 would be week 2)
    calendar.set_week_number_rule(CalendarWeekRule::FirstDay);

    // Force a layout pass - in unit-tests this is enough to trigger the update
    calendar.invalidate_measure();
    calendar.update_layout();

    // After the rule change the first visible week number should now be "2"
    let first_label = label_in_row(&week_labels_grid, 2);
    assert_eq!(Some(2), content_number(&first_label));
}

fn create_test_calendar() -> Ref<Calendar> {
    let cal = Calendar::new();
    let template = FuncControlTemplate::for_type::<Calendar>(|c, scope| {
        let calendar_item = CalendarItem::new();
        calendar_item.set_name(Some("PART_CalendarItem".to_string()));
        calendar_item.set_owner(Some(c));
        let day_title_template: Rc<dyn ITemplateOf<Option<Ref<Control>>>> =
            FuncTemplate::new(|| Some(TextBlock::new().upcast::<Control>()));
        calendar_item.set_day_title_template(Some(day_title_template));
        calendar_item.set_template(Some(FuncControlTemplate::for_type::<CalendarItem>(|_, item_scope| {
            let named_grid = |name: &str| {
                let grid = Grid::new();
                grid.set_name(Some(name.to_string()));
                grid.register_in_name_scope(&**item_scope)
            };

            let grid = Grid::new();
            grid.children().add(named_grid("PART_MonthView"));
            grid.children().add(named_grid("PART_YearView"));
            grid.children().add(named_grid("PART_ElementWeekNumberLabels"));
            grid.upcast()
        })));

        let panel = Panel::new();
        panel.set_name(Some("PART_Root".to_string()));
        panel.children().add(calendar_item.register_in_name_scope(&**scope));
        panel.register_in_name_scope(&**scope).upcast()
    });
    cal.set_template(Some(template));

    cal
}

// ============================================================================
//  ADDITIONAL TESTS - NOT PORTS OF REFERENCE TESTS.
//
//  The reference tests do not reach keyboard navigation, range selection with
//  the keyboard or the mouse, the population of the views or most of the
//  blackout paths. The tests of this module do; every expected value is
//  derived from the logic of the reference sources (formats and week days
//  were checked against .NET 10 with the en-US culture).
// ============================================================================
mod additional {
    use super::super::calendar_item::date_of;
    use super::*;
    use crate::Button;
    use ferroui_base::input::{
        InputElement, IPointer, Key, KeyEventArgs, MouseButton, PointerReleasedEventArgs, PointerWheelEventArgs,
    };
    use ferroui_base::reactive::IDisposable;
    use ferroui_base::utilities::TestCultureDataProvider;
    use ferroui_base::{FerroLocator, Vector};

    /// The scope of an additional test: a dispatcher, and `en-US` as the
    /// current culture (restored when dropped).
    struct Scope {
        culture: CultureInfo,
        ui_culture: CultureInfo,
        locator: Rc<dyn IDisposable>,
        _test: crate::test_support::TestScope,
    }

    impl Drop for Scope {
        fn drop(&mut self) {
            CultureInfo::set_current_culture(self.culture.clone());
            CultureInfo::set_current_ui_culture(self.ui_culture.clone());
            self.locator.dispose();
        }
    }

    fn start() -> Scope {
        let test = test_scope();
        let locator = FerroLocator::enter_scope();
        let scope = Scope {
            culture: CultureInfo::current_culture(),
            ui_culture: CultureInfo::current_ui_culture(),
            locator,
            _test: test,
        };
        TestCultureDataProvider::register();
        let culture = CultureInfo::get_culture_info("en-US");
        CultureInfo::set_current_culture(culture.clone());
        CultureInfo::set_current_ui_culture(culture);
        scope
    }

    fn date(year: i32, month: i32, day: i32) -> DateTime {
        DateTime::new(year, month, day)
    }

    /// A calendar whose template has every part of the calendar item: the
    /// three buttons of the header, the month view and the year view.
    fn create_calendar(display_date: DateTime) -> (Ref<Calendar>, Ref<CalendarItem>) {
        let cal = Calendar::new();
        // The default is the one of the culture at class initialisation.
        cal.set_first_day_of_week(DayOfWeek::Sunday);
        cal.set_display_date(display_date);
        let template = FuncControlTemplate::for_type::<Calendar>(|c, scope| {
            let calendar_item = CalendarItem::new();
            calendar_item.set_name(Some("PART_CalendarItem".to_string()));
            calendar_item.set_owner(Some(c));
            let day_title_template: Rc<dyn ITemplateOf<Option<Ref<Control>>>> =
                FuncTemplate::new(|| Some(TextBlock::new().upcast::<Control>()));
            calendar_item.set_day_title_template(Some(day_title_template));
            calendar_item.set_template(Some(FuncControlTemplate::for_type::<CalendarItem>(|_, item_scope| {
                let grid = Grid::new();
                for name in ["PART_HeaderButton", "PART_PreviousButton", "PART_NextButton"] {
                    let button = Button::new();
                    button.set_name(Some(name.to_string()));
                    grid.children().add(button.register_in_name_scope(&**item_scope));
                }
                for name in ["PART_MonthView", "PART_YearView"] {
                    let view = Grid::new();
                    view.set_name(Some(name.to_string()));
                    grid.children().add(view.register_in_name_scope(&**item_scope));
                }
                grid.upcast()
            })));

            let panel = Panel::new();
            panel.set_name(Some("PART_Root".to_string()));
            panel.children().add(calendar_item.register_in_name_scope(&**scope));
            panel.register_in_name_scope(&**scope).upcast()
        });
        cal.set_template(Some(template));
        cal.apply_template();
        let item = calendar_item_of(&cal).expect("the calendar has a calendar item");
        item.apply_template();
        (cal, item)
    }

    fn raise_key(calendar: &Calendar, key: Key, modifiers: KeyModifiers, up: bool) -> bool {
        let mut args = KeyEventArgs::new();
        args.set_routed_event(Some(if up { InputElement::key_up_event() } else { InputElement::key_down_event() }));
        args.key = key;
        args.key_modifiers = modifiers;
        calendar.raise_event(&args);
        args.handled()
    }

    /// Presses a key on the calendar; returns whether the calendar handled it.
    fn press(calendar: &Calendar, key: Key) -> bool {
        raise_key(calendar, key, KeyModifiers::NONE, false)
    }

    fn press_with(calendar: &Calendar, key: Key, modifiers: KeyModifiers) -> bool {
        raise_key(calendar, key, modifiers, false)
    }

    fn release_shift(calendar: &Calendar) {
        raise_key(calendar, Key::LeftShift, KeyModifiers::NONE, true);
    }

    fn wheel(calendar: &Ref<Calendar>, delta_y: f64, modifiers: KeyModifiers) -> bool {
        let pointer: Rc<dyn IPointer> = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
        let e = PointerWheelEventArgs::new(
            calendar.clone(),
            pointer,
            calendar,
            Point::default(),
            0,
            PointerPointProperties::new(RawInputModifiers::NONE, PointerUpdateKind::Other),
            modifiers,
            Vector::new(0.0, delta_y),
        );
        calendar.raise_event(&e);
        e.handled()
    }

    fn text_of(content: Option<BoxedValue>) -> String {
        content.and_then(|content| content.downcast_ref::<String>().cloned()).expect("the content is a text")
    }

    fn header(item: &CalendarItem) -> String {
        text_of(item.header_button().expect("the item has a header button").content())
    }

    /// The day button at an index of the month view (the days start at 7).
    fn day(item: &CalendarItem, index: usize) -> Ref<CalendarDayButton> {
        item.month_view().unwrap().children().get(index).cast::<CalendarDayButton>().expect("a day button")
    }

    fn day_date(item: &CalendarItem, index: usize) -> DateTime {
        date_of(&day(item, index)).expect("the day button has a date")
    }

    /// The month or year button at an index of the year view.
    fn year_button(item: &CalendarItem, index: usize) -> Ref<crate::calendar::CalendarButton> {
        item.year_view()
            .unwrap()
            .children()
            .get(index)
            .cast::<crate::calendar::CalendarButton>()
            .expect("a calendar button")
    }

    fn selected_indexes(item: &CalendarItem) -> Vec<usize> {
        (7..49).filter(|index| day(item, *index).is_selected()).collect()
    }

    fn pressed_args(source: &Ref<CalendarDayButton>, modifiers: KeyModifiers) -> PointerPressedEventArgs {
        let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
        let props =
            PointerPointProperties::new(RawInputModifiers::LEFT_MOUSE_BUTTON, PointerUpdateKind::LeftButtonPressed);
        PointerPressedEventArgs::new(
            source.clone().upcast::<Interactive>(),
            pointer,
            source,
            Point::default(),
            0,
            props,
            modifiers,
            1,
        )
    }

    fn released_args(source: &Ref<CalendarDayButton>) -> PointerReleasedEventArgs {
        let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
        let props = PointerPointProperties::new(RawInputModifiers::NONE, PointerUpdateKind::LeftButtonReleased);
        PointerReleasedEventArgs::new(
            source.clone().upcast::<Interactive>(),
            pointer,
            source,
            Point::default(),
            0,
            props,
            KeyModifiers::NONE,
            MouseButton::Left,
        )
    }

    fn tracker_of(calendar: &Calendar) -> Rc<SelectedDatesChangedTracker> {
        let tracker = Rc::new(SelectedDatesChangedTracker::default());
        // The calendar keeps the handler for as long as it lives.
        let _ = calendar.selected_dates_changed({
            let tracker = tracker.clone();
            move |e| tracker.on_selected_dates_changed(e)
        });
        tracker
    }

    // --- population of the views ---------------------------------------------

    #[test]
    fn additional_month_view_is_populated_from_the_display_date() {
        let _scope = start();
        let (calendar, item) = create_calendar(date(2024, 2, 15));

        let month_view = item.month_view().unwrap();
        let year_view = item.year_view().unwrap();
        // 7 day titles and 6 rows of 7 days; 3 rows of 4 months or years.
        assert_eq!(49, month_view.children().count());
        assert_eq!(12, year_view.children().count());
        assert!(month_view.is_visible());
        assert!(!year_view.is_visible());

        assert_eq!("February 2024", header(&item));

        let titles: Vec<String> =
            (0..7).map(|index| text_of(month_view.children().get(index).data_context())).collect();
        assert_eq!(vec!["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"], titles);

        // 1 February 2024 is a Thursday: four days of January come first.
        assert_eq!(date(2024, 1, 28), day_date(&item, 7));
        assert_eq!("28", text_of(day(&item, 7).content()));
        assert_eq!(7, day(&item, 7).index());
        assert!(day(&item, 7).is_inactive());
        assert_eq!(date(2024, 2, 1), day_date(&item, 11));
        assert!(!day(&item, 11).is_inactive());
        assert_eq!(date(2024, 2, 29), day_date(&item, 39));
        assert!(!day(&item, 39).is_inactive());
        assert_eq!(date(2024, 3, 1), day_date(&item, 40));
        assert!(day(&item, 40).is_inactive());
        assert_eq!(date(2024, 3, 9), day_date(&item, 48));
        assert_eq!(48, day(&item, 48).index());

        // With Monday as the first day three days of January come first.
        calendar.set_first_day_of_week(DayOfWeek::Monday);
        let titles: Vec<String> =
            (0..7).map(|index| text_of(month_view.children().get(index).data_context())).collect();
        assert_eq!(vec!["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"], titles);
        assert_eq!(date(2024, 1, 29), day_date(&item, 7));
        assert_eq!(date(2024, 2, 1), day_date(&item, 10));

        // A month that starts on the first day of the week shows a whole
        // week of the previous month (1 September 2024 is a Sunday).
        calendar.set_first_day_of_week(DayOfWeek::Sunday);
        calendar.set_display_date(date(2024, 9, 10));
        assert_eq!("September 2024", header(&item));
        assert_eq!(date(2024, 8, 25), day_date(&item, 7));
        assert_eq!(date(2024, 9, 1), day_date(&item, 14));
    }

    #[test]
    fn additional_display_range_disables_days_and_navigation_buttons() {
        let _scope = start();
        let (calendar, item) = create_calendar(date(2024, 2, 15));
        let previous = item.previous_button().unwrap();
        let next = item.next_button().unwrap();

        assert!(previous.is_enabled());
        assert!(next.is_enabled());
        // The buttons get a text for accessibility and cannot be focused.
        assert_eq!("previous button", text_of(previous.content()));
        assert_eq!("next button", text_of(next.content()));
        assert!(!previous.focusable() && !next.focusable() && !item.header_button().unwrap().focusable());

        calendar.set_display_date_start(Some(date(2024, 2, 10)));
        calendar.set_display_date_end(Some(date(2024, 2, 20)));

        // The first day of the month is not after the start; the first day
        // of the next month is after the end.
        assert!(!previous.is_enabled());
        assert!(!next.is_enabled());

        // 1 February is at 11.
        assert!(!day(&item, 19).is_enabled()); // 9 February
        assert_eq!(0.0, day(&item, 19).opacity());
        assert!(day(&item, 20).is_enabled()); // 10 February
        assert_eq!(1.0, day(&item, 20).opacity());
        assert!(day(&item, 30).is_enabled()); // 20 February
        assert!(!day(&item, 31).is_enabled()); // 21 February
        assert_eq!(0.0, day(&item, 31).opacity());
    }

    #[test]
    fn additional_blackout_and_selected_days_are_marked_in_the_month_view() {
        let _scope = start();
        let (calendar, item) = create_calendar(date(2024, 2, 15));

        calendar.blackout_dates().add(CalendarDateRange::new_range(date(2024, 2, 3), date(2024, 2, 4)));
        assert!(!day(&item, 12).is_blackout());
        assert!(day(&item, 13).is_blackout());
        assert!(day(&item, 14).is_blackout());
        assert!(!day(&item, 15).is_blackout());
        assert!(day(&item, 13).classes().contains(":blackout"));

        calendar.set_selected_date(Some(date(2024, 2, 6)));
        assert_eq!(vec![16], selected_indexes(&item));
        assert!(day(&item, 16).classes().contains(":selected"));

        calendar.blackout_dates().clear();
        assert!(!day(&item, 13).is_blackout());
    }

    // --- keyboard: month mode -------------------------------------------------

    #[test]
    fn additional_arrow_keys_move_the_selected_date_in_month_mode() {
        let _scope = start();
        let (calendar, item) = create_calendar(date(2024, 2, 15));
        calendar.set_selected_date(Some(date(2024, 2, 15)));

        assert!(press(&calendar, Key::Right));
        assert_eq!(Some(date(2024, 2, 16)), calendar.selected_date());
        assert!(press(&calendar, Key::Down));
        assert_eq!(Some(date(2024, 2, 23)), calendar.selected_date());
        assert!(press(&calendar, Key::Left));
        assert_eq!(Some(date(2024, 2, 22)), calendar.selected_date());
        assert!(press(&calendar, Key::Up));
        assert_eq!(Some(date(2024, 2, 15)), calendar.selected_date());
        assert_eq!(1, calendar.selected_dates().count());
        assert_eq!(vec![25], selected_indexes(&item));

        assert!(press(&calendar, Key::Home));
        assert_eq!(Some(date(2024, 2, 1)), calendar.selected_date());
        assert!(press(&calendar, Key::End));
        assert_eq!(Some(date(2024, 2, 29)), calendar.selected_date());
        assert_eq!(date(2024, 2, 15), calendar.display_date());

        // Leaving the month displays the month of the new date.
        assert!(press(&calendar, Key::Right));
        assert_eq!(Some(date(2024, 3, 1)), calendar.selected_date());
        assert_eq!(date(2024, 3, 1), calendar.display_date());
        assert_eq!("March 2024", header(&item));

        // Page down and page up change the month, not the selection; the
        // date the keys start from becomes the first of the month.
        assert!(press(&calendar, Key::PageDown));
        assert_eq!(date(2024, 4, 1), calendar.display_date());
        assert_eq!(Some(date(2024, 3, 1)), calendar.selected_date());
        assert_eq!(Some(date(2024, 4, 1)), calendar.last_selected_date());
        assert!(press(&calendar, Key::Right));
        assert_eq!(Some(date(2024, 4, 2)), calendar.selected_date());
        assert!(press(&calendar, Key::PageUp));
        assert_eq!(date(2024, 3, 1), calendar.display_date());
        assert_eq!(Some(date(2024, 4, 2)), calendar.selected_date());
        assert_eq!(Some(date(2024, 3, 1)), calendar.last_selected_date());

        // Enter, space and other keys are not handled in month mode.
        assert!(!press(&calendar, Key::Enter));
        assert!(!press(&calendar, Key::Space));
        assert!(!press(&calendar, Key::A));
        // Ctrl+Down does nothing in month mode, but is handled.
        assert!(press_with(&calendar, Key::Down, KeyModifiers::CONTROL));
        assert_eq!(CalendarMode::Month, calendar.display_mode());
        assert_eq!(Some(date(2024, 4, 2)), calendar.selected_date());

        // A disabled calendar handles no key.
        calendar.set_is_enabled(false);
        assert!(!press(&calendar, Key::Right));
        assert_eq!(Some(date(2024, 4, 2)), calendar.selected_date());
    }

    #[test]
    fn additional_keys_are_swallowed_when_the_last_selected_date_is_not_displayed() {
        let _scope = start();
        let (calendar, item) = create_calendar(date(2024, 2, 15));
        calendar.set_selected_date(Some(date(2024, 2, 15)));
        assert!(press(&calendar, Key::Right));
        // The key made the button of 16 February the focus button.
        assert!(calendar.focus_button().is_some_and(|button| button.ptr_eq(&day(&item, 26))));

        // A date of another month is selected while February is displayed.
        calendar.set_selected_date(Some(date(2024, 4, 20)));
        assert_eq!(date(2024, 2, 15), calendar.display_date());

        assert!(press(&calendar, Key::Right));
        assert_eq!(Some(date(2024, 4, 20)), calendar.selected_date());
        assert_eq!(date(2024, 2, 15), calendar.display_date());
    }

    #[test]
    fn additional_arrow_key_onto_a_blackout_date_is_ignored() {
        let _scope = start();
        let (calendar, _item) = create_calendar(date(2024, 2, 15));
        calendar.blackout_dates().add(CalendarDateRange::new(date(2024, 2, 10)));
        calendar.set_selected_date(Some(date(2024, 2, 9)));

        assert!(press(&calendar, Key::Right));
        assert_eq!(Some(date(2024, 2, 9)), calendar.selected_date());
        assert_eq!(Some(date(2024, 2, 9)), calendar.last_selected_date());

        // A date outside the display range is not selected either.
        calendar.set_display_date_start(Some(date(2024, 2, 9)));
        assert!(press(&calendar, Key::Left));
        assert_eq!(Some(date(2024, 2, 9)), calendar.selected_date());
        assert_eq!(Some(date(2024, 2, 9)), calendar.display_date_start());
    }

    // --- keyboard: year and decade modes --------------------------------------

    #[test]
    fn additional_keyboard_navigation_in_year_mode() {
        let _scope = start();
        let (calendar, item) = create_calendar(date(2024, 2, 15));
        let modes = Rc::new(RefCell::new(Vec::new()));
        let _subscription = calendar.display_mode_changed({
            let modes = modes.clone();
            move |e| modes.borrow_mut().push((e.old_mode(), e.new_mode()))
        });

        assert!(press_with(&calendar, Key::Up, KeyModifiers::CONTROL));
        assert_eq!(CalendarMode::Year, calendar.display_mode());
        assert_eq!(vec![(CalendarMode::Month, CalendarMode::Year)], *modes.borrow());
        assert!(!item.month_view().unwrap().is_visible());
        assert!(item.year_view().unwrap().is_visible());
        assert_eq!("2024", header(&item));
        assert!(item.header_button().unwrap().is_enabled());
        assert_eq!(date(2024, 2, 1), calendar.selected_month());

        assert_eq!("Jan", text_of(year_button(&item, 0).content()));
        assert_eq!(Some(date(2024, 1, 1)), date_of(&year_button(&item, 0)));
        assert_eq!("Dec", text_of(year_button(&item, 11).content()));
        assert_eq!(Some(date(2024, 12, 1)), date_of(&year_button(&item, 11)));
        // The month of the display date is selected, the selected month has
        // the focus.
        assert!(year_button(&item, 1).is_selected());
        assert!(!year_button(&item, 0).is_selected());
        assert!(calendar.focus_calendar_button().is_some_and(|button| button.ptr_eq(&year_button(&item, 1))));

        assert!(press(&calendar, Key::Right));
        assert_eq!(date(2024, 3, 1), calendar.selected_month());
        assert!(calendar.focus_calendar_button().is_some_and(|button| button.ptr_eq(&year_button(&item, 2))));
        // The selected button stays the month of the display date.
        assert!(year_button(&item, 1).is_selected());
        assert!(press(&calendar, Key::Down));
        assert_eq!(date(2024, 7, 1), calendar.selected_month());
        assert!(press(&calendar, Key::Left));
        assert_eq!(date(2024, 6, 1), calendar.selected_month());
        assert!(press(&calendar, Key::Up));
        assert_eq!(date(2024, 2, 1), calendar.selected_month());
        assert!(press(&calendar, Key::Home));
        assert_eq!(date(2024, 1, 1), calendar.selected_month());
        assert!(press(&calendar, Key::End));
        assert_eq!(date(2024, 12, 1), calendar.selected_month());

        // Moving past December shows the next year.
        assert!(press(&calendar, Key::Right));
        assert_eq!(date(2025, 1, 1), calendar.selected_month());
        assert_eq!("2025", header(&item));
        assert!(press(&calendar, Key::Left));
        assert_eq!(date(2024, 12, 1), calendar.selected_month());
        assert_eq!("2024", header(&item));

        // Page up shows January of the previous year; with shift the same
        // month of the next year.
        assert!(press(&calendar, Key::PageUp));
        assert_eq!(date(2023, 1, 1), calendar.selected_month());
        assert_eq!("2023", header(&item));
        assert!(press_with(&calendar, Key::PageDown, KeyModifiers::SHIFT));
        assert_eq!(date(2024, 1, 1), calendar.selected_month());
        assert!(press(&calendar, Key::PageDown));
        assert_eq!(date(2025, 1, 1), calendar.selected_month());
        assert!(press_with(&calendar, Key::PageUp, KeyModifiers::SHIFT));
        assert_eq!(date(2024, 1, 1), calendar.selected_month());

        // The display date did not change while the months were walked.
        assert_eq!(date(2024, 2, 15), calendar.display_date());

        // Enter displays the selected month.
        assert!(press(&calendar, Key::Down));
        assert_eq!(date(2024, 5, 1), calendar.selected_month());
        assert!(press(&calendar, Key::Enter));
        assert_eq!(CalendarMode::Month, calendar.display_mode());
        assert_eq!(date(2024, 5, 1), calendar.display_date());
        assert_eq!("May 2024", header(&item));
        assert!(item.month_view().unwrap().is_visible());
        assert!(!item.year_view().unwrap().is_visible());
        assert_eq!(Some(date(2024, 5, 1)), calendar.last_selected_date());
        assert_eq!(
            vec![(CalendarMode::Month, CalendarMode::Year), (CalendarMode::Year, CalendarMode::Month)],
            *modes.borrow()
        );

        // Space and Ctrl+Down do the same.
        assert!(press_with(&calendar, Key::Up, KeyModifiers::CONTROL));
        assert!(press(&calendar, Key::Right));
        assert!(press(&calendar, Key::Space));
        assert_eq!(CalendarMode::Month, calendar.display_mode());
        assert_eq!(date(2024, 6, 1), calendar.display_date());
        assert!(press_with(&calendar, Key::Up, KeyModifiers::CONTROL));
        assert!(press(&calendar, Key::Right));
        assert!(press_with(&calendar, Key::Down, KeyModifiers::CONTROL));
        assert_eq!(CalendarMode::Month, calendar.display_mode());
        assert_eq!(date(2024, 7, 1), calendar.display_date());
        assert_eq!("July 2024", header(&item));
    }

    #[test]
    fn additional_keyboard_navigation_in_decade_mode() {
        let _scope = start();
        let (calendar, item) = create_calendar(date(2024, 2, 15));

        assert!(press_with(&calendar, Key::Up, KeyModifiers::CONTROL));
        assert!(press_with(&calendar, Key::Up, KeyModifiers::CONTROL));
        assert_eq!(CalendarMode::Decade, calendar.display_mode());
        // Leaving year mode displays the selected month.
        assert_eq!(date(2024, 2, 1), calendar.display_date());
        assert_eq!(date(2024, 2, 1), calendar.selected_year());
        assert_eq!("2020-2029", header(&item));
        assert!(!item.header_button().unwrap().is_enabled());
        assert!(item.year_view().unwrap().is_visible());

        // The year before the decade, the ten years, the year after it.
        assert_eq!("2019", text_of(year_button(&item, 0).content()));
        assert_eq!(Some(date(2019, 1, 1)), date_of(&year_button(&item, 0)));
        assert!(year_button(&item, 0).is_inactive());
        assert!(!year_button(&item, 1).is_inactive());
        assert_eq!("2029", text_of(year_button(&item, 10).content()));
        assert!(!year_button(&item, 10).is_inactive());
        assert_eq!("2030", text_of(year_button(&item, 11).content()));
        assert!(year_button(&item, 11).is_inactive());
        assert!(year_button(&item, 5).is_selected());
        assert!(calendar.focus_calendar_button().is_some_and(|button| button.ptr_eq(&year_button(&item, 5))));

        // There is no larger mode.
        assert!(press_with(&calendar, Key::Up, KeyModifiers::CONTROL));
        assert_eq!(CalendarMode::Decade, calendar.display_mode());
        assert_eq!(date(2024, 2, 1), calendar.selected_year());

        assert!(press(&calendar, Key::Right));
        assert_eq!(date(2025, 2, 1), calendar.selected_year());
        assert!(calendar.focus_calendar_button().is_some_and(|button| button.ptr_eq(&year_button(&item, 6))));
        assert!(press(&calendar, Key::Down));
        assert_eq!(date(2029, 2, 1), calendar.selected_year());
        assert!(press(&calendar, Key::Right));
        assert_eq!(date(2030, 2, 1), calendar.selected_year());
        assert_eq!("2030-2039", header(&item));
        assert!(press(&calendar, Key::Up));
        assert_eq!(date(2026, 2, 1), calendar.selected_year());
        assert_eq!("2020-2029", header(&item));
        assert!(press(&calendar, Key::Left));
        assert_eq!(date(2025, 2, 1), calendar.selected_year());
        assert!(press(&calendar, Key::Home));
        assert_eq!(date(2020, 1, 1), calendar.selected_year());
        assert!(press(&calendar, Key::End));
        assert_eq!(date(2029, 1, 1), calendar.selected_year());

        // Page down shows the next decade; with shift the year ten years on.
        assert!(press(&calendar, Key::PageDown));
        assert_eq!(date(2030, 1, 1), calendar.selected_year());
        assert_eq!("2030-2039", header(&item));
        assert!(press_with(&calendar, Key::PageUp, KeyModifiers::SHIFT));
        assert_eq!(date(2020, 1, 1), calendar.selected_year());
        assert!(press_with(&calendar, Key::PageDown, KeyModifiers::SHIFT));
        assert_eq!(date(2030, 1, 1), calendar.selected_year());
        assert!(press(&calendar, Key::PageUp));
        assert_eq!(date(2020, 1, 1), calendar.selected_year());
        assert_eq!("2020-2029", header(&item));

        // Enter shows the months of the selected year.
        assert!(press(&calendar, Key::Right));
        assert!(press(&calendar, Key::Enter));
        assert_eq!(CalendarMode::Year, calendar.display_mode());
        assert_eq!(date(2021, 1, 1), calendar.display_date());
        assert_eq!(date(2021, 1, 1), calendar.selected_month());
        assert_eq!("2021", header(&item));
        assert!(year_button(&item, 0).is_selected());

        // Ctrl+Up and Ctrl+Down walk the modes.
        assert!(press_with(&calendar, Key::Up, KeyModifiers::CONTROL));
        assert_eq!(CalendarMode::Decade, calendar.display_mode());
        assert!(press_with(&calendar, Key::Down, KeyModifiers::CONTROL));
        assert_eq!(CalendarMode::Year, calendar.display_mode());
        assert!(press_with(&calendar, Key::Down, KeyModifiers::CONTROL));
        assert_eq!(CalendarMode::Month, calendar.display_mode());
        assert_eq!("January 2021", header(&item));
    }

    #[test]
    fn additional_display_range_limits_the_year_and_decade_views() {
        let _scope = start();
        let (calendar, item) = create_calendar(date(2024, 2, 15));
        calendar.set_display_date_start(Some(date(2023, 11, 20)));
        calendar.set_display_date_end(Some(date(2024, 6, 10)));

        calendar.set_display_mode(CalendarMode::Year);
        // The months of 2024 up to June can be chosen.
        assert!(year_button(&item, 5).is_enabled());
        assert_eq!(1.0, year_button(&item, 5).opacity());
        assert!(!year_button(&item, 6).is_enabled());
        assert_eq!(0.0, year_button(&item, 6).opacity());
        // 2023 is in the range, 2025 is not.
        assert!(item.previous_button().unwrap().is_enabled());
        assert!(!item.next_button().unwrap().is_enabled());

        // The selected month cannot leave the range.
        assert!(press(&calendar, Key::End));
        assert_eq!(date(2024, 6, 1), calendar.selected_month());
        assert!(press(&calendar, Key::PageUp));
        assert_eq!(date(2023, 11, 1), calendar.selected_month());
        assert_eq!("2023", header(&item));
        assert!(!year_button(&item, 9).is_enabled());
        assert!(year_button(&item, 10).is_enabled());

        calendar.set_display_mode(CalendarMode::Decade);
        assert_eq!("2020-2029", header(&item));
        assert!(!item.previous_button().unwrap().is_enabled());
        assert!(!item.next_button().unwrap().is_enabled());
        assert!(!year_button(&item, 3).is_enabled()); // 2022
        assert!(year_button(&item, 4).is_enabled()); // 2023
        assert!(year_button(&item, 5).is_enabled()); // 2024
        assert!(!year_button(&item, 6).is_enabled()); // 2025
    }

    // --- mouse: header and year view ------------------------------------------

    #[test]
    fn additional_header_and_navigation_buttons_change_mode_and_page() {
        let _scope = start();
        let (calendar, item) = create_calendar(date(2024, 2, 15));
        let header_button = item.header_button().unwrap();
        let previous = item.previous_button().unwrap();
        let next = item.next_button().unwrap();

        item.next_button_click(&next);
        assert_eq!(date(2024, 3, 1), calendar.display_date());
        item.previous_button_click(&previous);
        item.previous_button_click(&previous);
        assert_eq!(date(2024, 1, 1), calendar.display_date());
        assert_eq!("January 2024", header(&item));

        item.header_button_click(&header_button);
        assert_eq!(CalendarMode::Year, calendar.display_mode());
        assert_eq!(date(2024, 1, 1), calendar.selected_month());
        item.next_button_click(&next);
        assert_eq!(date(2025, 1, 1), calendar.selected_month());
        assert_eq!("2025", header(&item));

        item.header_button_click(&header_button);
        assert_eq!(CalendarMode::Decade, calendar.display_mode());
        assert_eq!("2020-2029", header(&item));
        item.previous_button_click(&previous);
        assert_eq!(date(2010, 1, 1), calendar.selected_year());
        assert_eq!("2010-2019", header(&item));

        // The header button is disabled in decade mode: a click does nothing.
        item.header_button_click(&header_button);
        assert_eq!(CalendarMode::Decade, calendar.display_mode());

        // Pressing the mouse over a year selects it (what the handler of the
        // mouse down does), releasing it shows its months; likewise a month
        // and its days.
        let year = year_button(&item, 4);
        assert_eq!(Some(date(2013, 1, 1)), date_of(&year));
        item.update_year_view_selection(Some(&year));
        assert_eq!(date(2013, 1, 1), calendar.selected_year());
        assert!(calendar.focus_calendar_button().is_some_and(|button| button.ptr_eq(&year)));
        item.month_calendar_button_mouse_up(Some(&year));
        assert_eq!(CalendarMode::Year, calendar.display_mode());
        assert_eq!(date(2013, 1, 1), calendar.selected_month());
        assert_eq!("2013", header(&item));

        let month = year_button(&item, 8);
        assert_eq!(Some(date(2013, 9, 1)), date_of(&month));
        item.update_year_view_selection(Some(&month));
        assert_eq!(date(2013, 9, 1), calendar.selected_month());
        item.month_calendar_button_mouse_up(Some(&month));
        assert_eq!(CalendarMode::Month, calendar.display_mode());
        assert_eq!(date(2013, 9, 1), calendar.display_date());
        assert_eq!("September 2013", header(&item));
    }

    #[test]
    fn additional_mouse_wheel_changes_page_and_mode() {
        let _scope = start();
        let (calendar, item) = create_calendar(date(2024, 2, 15));

        // Up is the previous page, down the next one.
        assert!(wheel(&calendar, 1.0, KeyModifiers::NONE));
        assert_eq!(date(2024, 1, 1), calendar.display_date());
        assert!(wheel(&calendar, -1.0, KeyModifiers::NONE));
        assert!(wheel(&calendar, -1.0, KeyModifiers::NONE));
        assert_eq!(date(2024, 3, 1), calendar.display_date());

        // With control, down is the next larger mode and up the next smaller.
        assert!(wheel(&calendar, 1.0, KeyModifiers::CONTROL));
        assert_eq!(CalendarMode::Month, calendar.display_mode());
        assert!(wheel(&calendar, -1.0, KeyModifiers::CONTROL));
        assert_eq!(CalendarMode::Year, calendar.display_mode());
        assert_eq!("2024", header(&item));
        assert!(wheel(&calendar, -1.0, KeyModifiers::NONE));
        assert_eq!(date(2025, 1, 1), calendar.selected_month());
        assert!(wheel(&calendar, -1.0, KeyModifiers::CONTROL));
        assert_eq!(CalendarMode::Decade, calendar.display_mode());
        assert!(wheel(&calendar, 1.0, KeyModifiers::CONTROL));
        assert_eq!(CalendarMode::Year, calendar.display_mode());
        assert!(wheel(&calendar, 1.0, KeyModifiers::CONTROL));
        assert_eq!(CalendarMode::Month, calendar.display_mode());
        assert_eq!(date(2025, 1, 1), calendar.display_date());
    }

    // --- range selection with the keyboard ------------------------------------

    /// A calendar in single range mode without tap selection whose only
    /// selected date is 1 February 2024, selected with the Home key.
    fn create_range_calendar(mode: CalendarSelectionMode) -> (Ref<Calendar>, Ref<CalendarItem>) {
        let (calendar, item) = create_calendar(date(2024, 2, 15));
        calendar.set_selection_mode(mode);
        calendar.set_allow_tap_range_selection(false);
        (calendar, item)
    }

    #[test]
    fn additional_shift_arrow_keys_select_a_range() {
        let _scope = start();
        let (calendar, item) = create_range_calendar(CalendarSelectionMode::SingleRange);
        let tracker = tracker_of(&calendar);

        assert!(press(&calendar, Key::Home));
        assert_eq!(vec![date(2024, 2, 1)], calendar.selected_dates().to_vec());
        assert_eq!(Some(date(2024, 2, 1)), calendar.selected_date());
        assert_eq!(Some(date(2024, 2, 1)), calendar.last_selected_date());
        assert_eq!(Some(date(2024, 2, 1)), calendar.hover_start());
        assert_eq!(Some(date(2024, 2, 1)), calendar.hover_end());
        assert_eq!(Some(11), calendar.hover_start_index());
        assert_eq!(Some(11), calendar.hover_end_index());
        assert_eq!(1, tracker.count.get());
        tracker.reset();

        for (presses, end_day) in [(1, 2), (2, 3), (3, 4)] {
            assert!(press_with(&calendar, Key::Right, KeyModifiers::SHIFT));
            // The days are highlighted; they are selected when shift is
            // released.
            assert_eq!(0, calendar.selected_dates().count());
            assert_eq!(Some(date(2024, 2, end_day)), calendar.hover_end());
            assert_eq!(Some(11 + presses), calendar.hover_end_index());
            assert_eq!((11..=11 + presses as usize).collect::<Vec<_>>(), selected_indexes(&item));
        }
        assert_eq!(Some(date(2024, 2, 1)), calendar.hover_start());
        assert_eq!(Some(date(2024, 2, 4)), calendar.last_selected_date());
        assert!(calendar.focus_button().is_some_and(|button| button.ptr_eq(&day(&item, 14))));
        assert_eq!(0, tracker.count.get());

        // Shift+Left shrinks the range again.
        assert!(press_with(&calendar, Key::Left, KeyModifiers::SHIFT));
        assert_eq!(Some(date(2024, 2, 3)), calendar.hover_end());
        assert_eq!(vec![11, 12, 13], selected_indexes(&item));
        assert!(press_with(&calendar, Key::Right, KeyModifiers::SHIFT));

        release_shift(&calendar);
        assert_eq!(
            vec![date(2024, 2, 1), date(2024, 2, 2), date(2024, 2, 3), date(2024, 2, 4)],
            calendar.selected_dates().to_vec()
        );
        assert_eq!(Some(date(2024, 2, 1)), calendar.selected_date());
        assert_eq!(vec![11, 12, 13, 14], selected_indexes(&item));
        assert_eq!(1, tracker.count.get());
        assert_eq!(4, tracker.added_count());
        assert_eq!(1, tracker.removed_count());
        tracker.reset();

        // Releasing shift again changes nothing.
        release_shift(&calendar);
        assert_eq!(0, tracker.count.get());

        // An arrow key without shift selects a single date again.
        assert!(press(&calendar, Key::Right));
        assert_eq!(vec![date(2024, 2, 5)], calendar.selected_dates().to_vec());
        assert_eq!(Some(date(2024, 2, 5)), calendar.selected_date());
        assert_eq!(Some(date(2024, 2, 5)), calendar.hover_start());
        assert_eq!(1, tracker.count.get());
        assert_eq!(1, tracker.added_count());
        assert_eq!(4, tracker.removed_count());
    }

    #[test]
    fn additional_shift_down_and_shift_end_select_a_range() {
        let _scope = start();
        let (calendar, item) = create_range_calendar(CalendarSelectionMode::MultipleRange);

        assert!(press(&calendar, Key::Home));
        assert!(press_with(&calendar, Key::Down, KeyModifiers::SHIFT));
        assert_eq!(Some(date(2024, 2, 8)), calendar.hover_end());
        assert_eq!(Some(18), calendar.hover_end_index());
        assert_eq!((11..=18).collect::<Vec<_>>(), selected_indexes(&item));

        // The index of the end is looked up for Home, End and the page keys.
        assert!(press_with(&calendar, Key::End, KeyModifiers::SHIFT));
        assert_eq!(Some(date(2024, 2, 29)), calendar.hover_end());
        assert_eq!(Some(39), calendar.hover_end_index());
        assert_eq!((11..=39).collect::<Vec<_>>(), selected_indexes(&item));

        release_shift(&calendar);
        assert_eq!(29, calendar.selected_dates().count());
        assert_eq!(date(2024, 2, 1), calendar.selected_dates().get(0));
        assert_eq!(date(2024, 2, 29), calendar.selected_dates().get(28));
    }

    #[test]
    fn additional_shift_range_can_reach_into_the_previous_month() {
        let _scope = start();
        let (calendar, item) = create_range_calendar(CalendarSelectionMode::SingleRange);

        assert!(press(&calendar, Key::Home));
        assert!(press_with(&calendar, Key::Left, KeyModifiers::SHIFT));

        // January is displayed: 1 January 2024 is a Monday, so that
        // 31 January is at 38 and 1 February at 39.
        assert_eq!(date(2024, 1, 1), calendar.display_date());
        assert_eq!("January 2024", header(&item));
        assert_eq!(Some(date(2024, 1, 31)), calendar.hover_end());
        assert_eq!(Some(38), calendar.hover_end_index());
        assert_eq!(Some(39), calendar.hover_start_index());
        assert_eq!(vec![38, 39], selected_indexes(&item));

        release_shift(&calendar);
        // The dates are added from the start of the range to its end.
        assert_eq!(vec![date(2024, 2, 1), date(2024, 1, 31)], calendar.selected_dates().to_vec());
        assert_eq!(Some(date(2024, 2, 1)), calendar.selected_date());
    }

    #[test]
    fn additional_shift_range_stops_at_blackout_dates() {
        let _scope = start();

        // The key that would move onto a blackout date does nothing.
        let (calendar, item) = create_range_calendar(CalendarSelectionMode::SingleRange);
        calendar.blackout_dates().add(CalendarDateRange::new(date(2024, 2, 4)));
        assert!(press(&calendar, Key::Home));
        assert!(press_with(&calendar, Key::Right, KeyModifiers::SHIFT));
        assert!(press_with(&calendar, Key::Right, KeyModifiers::SHIFT));
        assert!(press_with(&calendar, Key::Right, KeyModifiers::SHIFT));
        assert_eq!(Some(date(2024, 2, 3)), calendar.hover_end());
        assert_eq!(Some(13), calendar.hover_end_index());
        assert_eq!(vec![11, 12, 13], selected_indexes(&item));
        release_shift(&calendar);
        assert_eq!(vec![date(2024, 2, 1), date(2024, 2, 2), date(2024, 2, 3)], calendar.selected_dates().to_vec());

        // A range that would contain a blackout date does not move its end.
        let (calendar, item) = create_range_calendar(CalendarSelectionMode::SingleRange);
        calendar.blackout_dates().add(CalendarDateRange::new(date(2024, 2, 5)));
        let tracker = tracker_of(&calendar);
        assert!(press(&calendar, Key::Home));
        tracker.reset();
        assert!(press_with(&calendar, Key::Down, KeyModifiers::SHIFT));
        assert_eq!(Some(date(2024, 2, 1)), calendar.hover_end());
        assert_eq!(Some(date(2024, 2, 1)), calendar.last_selected_date());
        assert_eq!(vec![11], selected_indexes(&item));
        release_shift(&calendar);
        assert_eq!(vec![date(2024, 2, 1)], calendar.selected_dates().to_vec());
        // The same date was removed and added: no event.
        assert_eq!(0, tracker.count.get());
    }

    // --- range selection with the mouse ---------------------------------------

    #[test]
    fn additional_mouse_drag_selects_a_range_up_to_a_blackout_date() {
        let _scope = start();
        let (calendar, item) = create_range_calendar(CalendarSelectionMode::SingleRange);
        calendar.blackout_dates().add(CalendarDateRange::new(date(2024, 2, 3)));
        let mouse_ups = Rc::new(Cell::new(0));
        let _subscription = calendar.day_button_mouse_up({
            let mouse_ups = mouse_ups.clone();
            move |_| mouse_ups.set(mouse_ups.get() + 1)
        });

        // Entering a day without the button down does nothing.
        item.cell_mouse_entered(&day(&item, 15));
        assert_eq!(None, calendar.hover_end());

        // Pressing a blackout day clears the start of the range.
        calendar.set_hover_start(Some(date(2024, 2, 20)));
        item.cell_mouse_left_button_down(&day(&item, 13), &pressed_args(&day(&item, 13), KeyModifiers::NONE));
        assert_eq!(None, calendar.hover_start());

        item.cell_mouse_left_button_down(&day(&item, 11), &pressed_args(&day(&item, 11), KeyModifiers::NONE));
        assert_eq!(Some(date(2024, 2, 1)), calendar.hover_start());
        assert_eq!(Some(11), calendar.hover_start_index());
        assert_eq!(0, calendar.selected_dates().count());

        // Entering a blackout day does not move the end of the range.
        item.cell_mouse_entered(&day(&item, 13));
        assert_eq!(None, calendar.hover_end());

        // Every day of the hovered range is highlighted.
        item.cell_mouse_entered(&day(&item, 15));
        assert_eq!(Some(date(2024, 2, 5)), calendar.hover_end());
        assert_eq!(Some(15), calendar.hover_end_index());
        assert_eq!(vec![11, 12, 13, 14, 15], selected_indexes(&item));
        item.cell_mouse_entered(&day(&item, 14));
        assert_eq!(vec![11, 12, 13, 14], selected_indexes(&item));
        item.cell_mouse_entered(&day(&item, 15));

        // A single range ends before the first blackout date.
        item.cell_mouse_left_button_up(&day(&item, 15), &released_args(&day(&item, 15)));
        assert_eq!(1, mouse_ups.get());
        assert_eq!(vec![date(2024, 2, 1), date(2024, 2, 2)], calendar.selected_dates().to_vec());
        assert_eq!(Some(date(2024, 2, 2)), calendar.hover_end());
        assert_eq!(vec![11, 12], selected_indexes(&item));
        assert!(calendar.is_mouse_selection());

        // The button is up: entering a day changes nothing.
        item.cell_mouse_entered(&day(&item, 20));
        assert_eq!(Some(date(2024, 2, 2)), calendar.hover_end());

        // Releasing the mouse over a blackout day does not raise the event
        // of the calendar.
        item.cell_mouse_left_button_up(&day(&item, 13), &released_args(&day(&item, 13)));
        assert_eq!(1, mouse_ups.get());
    }

    #[test]
    fn additional_mouse_drag_skips_blackout_dates_in_multiple_range_mode() {
        let _scope = start();
        let (calendar, item) = create_range_calendar(CalendarSelectionMode::MultipleRange);
        calendar.blackout_dates().add(CalendarDateRange::new(date(2024, 2, 3)));

        item.cell_mouse_left_button_down(&day(&item, 11), &pressed_args(&day(&item, 11), KeyModifiers::NONE));
        item.cell_mouse_entered(&day(&item, 15));
        item.cell_mouse_left_button_up(&day(&item, 15), &released_args(&day(&item, 15)));
        assert_eq!(
            vec![date(2024, 2, 1), date(2024, 2, 2), date(2024, 2, 4), date(2024, 2, 5)],
            calendar.selected_dates().to_vec()
        );
        assert_eq!(vec![11, 12, 14, 15], selected_indexes(&item));

        // With control a second range is added to the first.
        item.cell_mouse_left_button_down(&day(&item, 20), &pressed_args(&day(&item, 20), KeyModifiers::CONTROL));
        item.cell_mouse_entered(&day(&item, 21));
        item.cell_mouse_left_button_up(&day(&item, 21), &released_args(&day(&item, 21)));
        assert_eq!(6, calendar.selected_dates().count());
        assert!(calendar.selected_dates().contains(date(2024, 2, 10)));
        assert!(calendar.selected_dates().contains(date(2024, 2, 11)));
        assert_eq!(vec![11, 12, 14, 15, 20, 21], selected_indexes(&item));

        // Without control the selection starts again.
        item.cell_mouse_left_button_down(&day(&item, 25), &pressed_args(&day(&item, 25), KeyModifiers::NONE));
        item.cell_mouse_left_button_up(&day(&item, 25), &released_args(&day(&item, 25)));
        assert_eq!(vec![date(2024, 2, 15)], calendar.selected_dates().to_vec());
    }

    #[test]
    fn additional_mouse_drag_moves_the_selected_date_in_single_date_mode() {
        let _scope = start();
        let (calendar, item) = create_calendar(date(2024, 2, 15));

        item.cell_mouse_left_button_down(&day(&item, 11), &pressed_args(&day(&item, 11), KeyModifiers::NONE));
        assert_eq!(Some(date(2024, 2, 1)), calendar.selected_date());
        assert!(calendar.calendar_date_picker_display_date_flag());
        item.cell_mouse_entered(&day(&item, 15));
        assert_eq!(Some(date(2024, 2, 5)), calendar.selected_date());
        assert_eq!(1, calendar.selected_dates().count());

        // Releasing over a day of the next month displays that month.
        item.cell_mouse_entered(&day(&item, 40));
        assert_eq!(Some(date(2024, 3, 1)), calendar.selected_date());
        item.cell_mouse_left_button_up(&day(&item, 40), &released_args(&day(&item, 40)));
        assert_eq!(date(2024, 3, 1), calendar.display_date());
        assert_eq!("March 2024", header(&item));
    }

    // --- blackout dates --------------------------------------------------------

    #[test]
    fn additional_blackout_dates_collection_queries() {
        let _scope = start();
        let calendar = Calendar::new();
        let blackout_dates = calendar.blackout_dates();
        blackout_dates.add(CalendarDateRange::new_range(date(2024, 2, 10), date(2024, 2, 12)));

        assert!(blackout_dates.contains_date(date(2024, 2, 10)));
        assert!(blackout_dates.contains_date(DateTime::new_with_time(2024, 2, 12, 13, 30, 0)));
        assert!(!blackout_dates.contains_date(date(2024, 2, 9)));
        assert!(!blackout_dates.contains_date(date(2024, 2, 13)));

        // A range is contained when it is one of the ranges, in any order
        // of its ends.
        assert!(blackout_dates.contains_dates(date(2024, 2, 10), date(2024, 2, 12)));
        assert!(blackout_dates.contains_dates(date(2024, 2, 12), date(2024, 2, 10)));
        assert!(!blackout_dates.contains_dates(date(2024, 2, 10), date(2024, 2, 11)));

        assert!(blackout_dates.contains_any(&CalendarDateRange::new_range(date(2024, 2, 12), date(2024, 2, 20))));
        assert!(blackout_dates.contains_any(&CalendarDateRange::new_range(date(2024, 2, 1), date(2024, 2, 10))));
        assert!(blackout_dates.contains_any(&CalendarDateRange::new_range(date(2024, 2, 1), date(2024, 2, 29))));
        assert!(blackout_dates.contains_any(&CalendarDateRange::new(date(2024, 2, 11))));
        assert!(!blackout_dates.contains_any(&CalendarDateRange::new_range(date(2024, 2, 13), date(2024, 2, 20))));
        assert!(!blackout_dates.contains_any(&CalendarDateRange::new_range(date(2024, 2, 1), date(2024, 2, 9))));

        // A range whose end is before its start is the day of its start.
        let range = CalendarDateRange::new_range(date(2024, 2, 20), date(2024, 2, 1));
        assert_eq!(date(2024, 2, 20), range.start());
        assert_eq!(date(2024, 2, 20), range.end());
    }

    #[test]
    fn additional_selecting_blackout_dates_is_rejected() {
        let _scope = start();
        let calendar = Calendar::new();
        calendar.blackout_dates().add(CalendarDateRange::new_range(date(2024, 2, 10), date(2024, 2, 12)));

        // Adding a blackout date to the selected dates throws.
        let message = panic_message(|| calendar.selected_dates().add(date(2024, 2, 11)));
        assert!(message.contains("SelectedDate value is not valid."), "{message}");
        assert_eq!(0, calendar.selected_dates().count());

        calendar.set_selected_date(Some(date(2024, 2, 9)));

        // Replacing a selected date with a blackout date is ignored.
        calendar.selected_dates().set(0, date(2024, 2, 11));
        assert_eq!(vec![date(2024, 2, 9)], calendar.selected_dates().to_vec());
        assert_eq!(Some(date(2024, 2, 9)), calendar.selected_date());

        // A range that contains a selected date cannot become a blackout
        // range, neither added nor in place of another one.
        let message = panic_message(|| calendar.blackout_dates().add(CalendarDateRange::new(date(2024, 2, 9))));
        assert!(message.contains("Value is not valid."), "{message}");
        let message = panic_message(|| {
            calendar.blackout_dates().set(0, CalendarDateRange::new_range(date(2024, 2, 8), date(2024, 2, 9)))
        });
        assert!(message.contains("Value is not valid."), "{message}");
        let message = panic_message(|| {
            calendar.blackout_dates().insert(0, CalendarDateRange::new_range(date(2024, 2, 1), date(2024, 2, 9)))
        });
        assert!(message.contains("Value is not valid."), "{message}");
        assert_eq!(1, calendar.blackout_dates().count());
        assert_eq!(date(2024, 2, 10), calendar.blackout_dates().get(0).start());

        // A range that does not is accepted.
        calendar.blackout_dates().set(0, CalendarDateRange::new_range(date(2024, 2, 10), date(2024, 2, 20)));
        assert!(calendar.blackout_dates().contains_date(date(2024, 2, 20)));
    }

    #[test]
    fn additional_adding_a_range_with_a_blackout_date_throws() {
        let _scope = start();
        let calendar = Calendar::new();
        calendar.set_selection_mode(CalendarSelectionMode::MultipleRange);
        calendar.blackout_dates().add(CalendarDateRange::new(date(2024, 2, 10)));

        let message =
            panic_message(|| calendar.selected_dates().add_range(date(2024, 2, 8), date(2024, 2, 11)));
        assert!(message.contains("SelectedDate value is not valid."), "{message}");
        // The dates before the blackout date were added.
        assert_eq!(vec![date(2024, 2, 8), date(2024, 2, 9)], calendar.selected_dates().to_vec());
    }

    #[test]
    fn additional_selection_mode_rules_of_the_selected_dates() {
        let _scope = start();

        // No selection is allowed in the mode None.
        let calendar = Calendar::new();
        calendar.set_selection_mode(CalendarSelectionMode::None);
        let message = panic_message(|| calendar.set_selected_date(Some(date(2024, 2, 9))));
        assert!(message.contains("The SelectedDate property cannot be set when the selection mode is None."));
        let calendar = Calendar::new();
        calendar.set_selection_mode(CalendarSelectionMode::None);
        let message = panic_message(|| calendar.selected_dates().add(date(2024, 2, 9)));
        assert!(message.contains("The SelectedDate property cannot be set when the selection mode is None."));
        assert_eq!(0, calendar.selected_dates().count());

        // A range of more than a day cannot be added in single date mode.
        let calendar = Calendar::new();
        let message = panic_message(|| calendar.selected_dates().add_range(date(2024, 2, 1), date(2024, 2, 2)));
        assert!(message.contains("The SelectedDates collection can be changed only in a multiple selection mode"));

        // Multiple ranges are united; a date that is selected is not added
        // again.
        let calendar = Calendar::new();
        calendar.set_selection_mode(CalendarSelectionMode::MultipleRange);
        calendar.selected_dates().add_range(date(2024, 2, 1), date(2024, 2, 3));
        calendar.selected_dates().add_range(date(2024, 2, 3), date(2024, 2, 5));
        calendar.selected_dates().add(date(2024, 2, 2));
        assert_eq!(5, calendar.selected_dates().count());
        assert_eq!(Some(date(2024, 2, 1)), calendar.selected_date());

        // Removing the first date makes the next one the selected date.
        assert!(calendar.selected_dates().remove(date(2024, 2, 1)));
        assert_eq!(Some(date(2024, 2, 2)), calendar.selected_date());
        assert!(!calendar.selected_dates().remove(date(2024, 2, 20)));

        // Changing the mode clears the selection.
        calendar.set_selection_mode(CalendarSelectionMode::SingleRange);
        assert_eq!(0, calendar.selected_dates().count());
        assert_eq!(None, calendar.selected_date());
    }

    // --- coercion of the display dates ------------------------------------------

    #[test]
    fn additional_display_date_range_is_coerced() {
        let _scope = start();

        // The end cannot be before the start.
        let calendar = Calendar::new();
        calendar.set_display_date(date(2024, 2, 15));
        calendar.set_display_date_start(Some(date(2024, 2, 10)));
        calendar.set_display_date_end(Some(date(2024, 2, 5)));
        assert_eq!(Some(date(2024, 2, 10)), calendar.display_date_end());
        assert_eq!(Some(date(2024, 2, 10)), calendar.display_date_start());

        // The start cannot be after a selected date, the end not before one.
        let calendar = Calendar::new();
        calendar.set_display_date(date(2024, 2, 15));
        calendar.set_selected_date(Some(date(2024, 2, 5)));
        calendar.set_display_date_start(Some(date(2024, 2, 10)));
        assert_eq!(Some(date(2024, 2, 5)), calendar.display_date_start());
        calendar.set_display_date_end(Some(date(2024, 2, 1)));
        assert_eq!(Some(date(2024, 2, 5)), calendar.display_date_end());

        // Selecting a date outside the range extends the range.
        let calendar = Calendar::new();
        calendar.set_display_date(date(2024, 2, 15));
        calendar.set_display_date_start(Some(date(2024, 2, 10)));
        calendar.set_display_date_end(Some(date(2024, 2, 20)));
        calendar.set_selected_date(Some(date(2024, 2, 1)));
        assert_eq!(Some(date(2024, 2, 1)), calendar.display_date_start());
        calendar.set_selected_date(Some(date(2024, 3, 3)));
        assert_eq!(Some(date(2024, 3, 3)), calendar.display_date_end());

        // A start in a later month than the display date moves the display
        // date; so does an end in an earlier month.
        let calendar = Calendar::new();
        calendar.set_display_date(date(2024, 2, 15));
        calendar.set_display_date_start(Some(date(2024, 4, 10)));
        assert_eq!(date(2024, 4, 10), calendar.display_date());
        calendar.set_display_date_start(None);
        calendar.set_display_date_end(Some(date(2024, 1, 20)));
        assert_eq!(date(2024, 1, 20), calendar.display_date());

        // A display date outside the range becomes the nearest end.
        calendar.set_display_date(date(2024, 6, 1));
        assert_eq!(date(2024, 1, 20), calendar.display_date());
        calendar.set_display_date_start(Some(date(2023, 12, 25)));
        calendar.set_display_date(date(2023, 1, 1));
        assert_eq!(date(2023, 12, 25), calendar.display_date());
    }

    #[test]
    fn additional_display_text_is_the_selected_date_in_the_general_format() {
        let _scope = start();
        let calendar = Calendar::new();
        assert_eq!("", calendar.to_string());
        calendar.set_selected_date(Some(date(2024, 2, 5)));
        // The general format of en-US has a narrow no-break space before
        // the designator.
        assert_eq!("2/5/2024 12:00:00\u{202F}AM", calendar.to_string());
    }

    #[test]
    fn additional_display_date_changed_reports_the_dates() {
        let _scope = start();
        let calendar = Calendar::new();
        calendar.set_display_date(date(2024, 2, 15));
        let changes = Rc::new(RefCell::new(Vec::new()));
        let _subscription = calendar.display_date_changed({
            let changes = changes.clone();
            move |e| changes.borrow_mut().push((e.removed_date(), e.added_date()))
        });

        calendar.set_display_date(date(2024, 3, 20));
        assert_eq!(vec![(Some(date(2024, 2, 15)), Some(date(2024, 3, 20)))], *changes.borrow());
        assert_eq!(date(2024, 3, 1), calendar.display_date_internal());
    }
}
