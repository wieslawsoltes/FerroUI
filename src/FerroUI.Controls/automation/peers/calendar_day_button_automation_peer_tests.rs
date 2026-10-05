use super::{AutomationPeer, CalendarDayButtonAutomationPeer, ControlAutomationPeer};
use crate::automation::provider::{IInvokeProvider, ISelectionItemProvider};
use crate::calendar::{Calendar, CalendarDayButton, CalendarSelectionMode};
use crate::test_support::test_scope;
use ferroui_base::utilities::DateTime;
use ferroui_base::{BoxedValue, Ref};
use std::rc::Rc;

fn date1() -> DateTime {
    DateTime::new(2026, 6, 5)
}

fn date2() -> DateTime {
    DateTime::new(2026, 6, 10)
}

// A peer holds its control weakly (the control owns the peer): the tests keep
// the day button for as long as they use its provider.
fn create_target(
    mode: CalendarSelectionMode,
    date: Option<DateTime>,
) -> (Ref<Calendar>, Ref<CalendarDayButton>, Rc<dyn ISelectionItemProvider>) {
    let calendar = Calendar::new();
    calendar.set_selection_mode(mode);
    let day_button = CalendarDayButton::new();
    day_button.set_owner(Some(&calendar));
    day_button.set_data_context(Some(Rc::new(date.unwrap_or_else(date1)) as BoxedValue));
    let peer = ControlAutomationPeer::create_peer_for_element(&day_button);
    let provider = peer.get_provider::<dyn ISelectionItemProvider>().expect("a selection item provider");
    (calendar, day_button, provider)
}

#[test]
fn creates_calendar_day_button_automation_peer() {
    let _scope = test_scope();
    let peer: Ref<AutomationPeer> = ControlAutomationPeer::create_peer_for_element(&CalendarDayButton::new());

    assert!(std::ptr::eq(peer.get_type(), CalendarDayButtonAutomationPeer::TYPE));
    assert!(peer.get_provider::<dyn ISelectionItemProvider>().is_some());
    assert!(peer.get_provider::<dyn IInvokeProvider>().is_some());
}

#[test]
fn is_selected_reflects_owner_state() {
    let _scope = test_scope();
    let (_, day_button, provider) = create_target(CalendarSelectionMode::SingleDate, None);

    assert!(!provider.is_selected());
    day_button.set_is_selected(true);
    assert!(provider.is_selected());
}

#[test]
fn selection_container_is_calendar_peer() {
    let _scope = test_scope();
    let (calendar, _day_button, provider) = create_target(CalendarSelectionMode::SingleDate, None);

    let container = provider.selection_container();

    let container = container.expect("a selection container");
    assert!(ControlAutomationPeer::create_peer_for_element(&calendar) == container.peer());
}

#[test]
fn select_sets_selected_date() {
    let _scope = test_scope();
    let (calendar, _day_button, provider) = create_target(CalendarSelectionMode::SingleDate, None);

    provider.select().unwrap();

    assert_eq!(Some(date1()), calendar.selected_date());
}

#[test]
fn select_replaces_existing_selection_in_single_date_mode() {
    let _scope = test_scope();
    let (calendar, _day_button, provider) = create_target(CalendarSelectionMode::SingleDate, None);
    calendar.set_selected_date(Some(date2()));

    provider.select().unwrap();

    assert_eq!(Some(date1()), calendar.selected_date());
    assert_eq!(1, calendar.selected_dates().count());
}

#[test]
fn select_is_no_op_when_selection_mode_none() {
    let _scope = test_scope();
    let (calendar, _day_button, provider) = create_target(CalendarSelectionMode::None, None);

    provider.select().unwrap();

    assert_eq!(None, calendar.selected_date());
    assert!(calendar.selected_dates().is_empty());
}

#[test]
fn select_is_no_op_for_blackout_day() {
    let _scope = test_scope();
    let (calendar, day_button, provider) = create_target(CalendarSelectionMode::SingleDate, None);
    day_button.set_is_blackout(true);

    provider.select().unwrap();

    assert_eq!(None, calendar.selected_date());
}

#[test]
fn add_to_selection_replaces_selection_in_single_date_mode() {
    let _scope = test_scope();
    let (calendar, _day_button, provider) = create_target(CalendarSelectionMode::SingleDate, None);
    calendar.set_selected_date(Some(date2()));

    provider.add_to_selection().unwrap();

    assert_eq!(Some(date1()), calendar.selected_date());
    assert_eq!(1, calendar.selected_dates().count());
}

#[test]
fn add_to_selection_appends_in_multiple_range_mode() {
    let _scope = test_scope();
    let (calendar, _day_button, provider) = create_target(CalendarSelectionMode::MultipleRange, None);
    calendar.selected_dates().add(date2());

    provider.add_to_selection().unwrap();

    assert_eq!(2, calendar.selected_dates().count());
    assert!(calendar.selected_dates().contains(date1()));
    assert!(calendar.selected_dates().contains(date2()));
}

#[test]
fn add_to_selection_is_no_op_when_selection_mode_none() {
    let _scope = test_scope();
    let (calendar, _day_button, provider) = create_target(CalendarSelectionMode::None, None);

    provider.add_to_selection().unwrap();

    assert!(calendar.selected_dates().is_empty());
}

#[test]
fn add_to_selection_is_no_op_for_blackout_day() {
    let _scope = test_scope();
    let (calendar, day_button, provider) = create_target(CalendarSelectionMode::MultipleRange, None);
    day_button.set_is_blackout(true);

    provider.add_to_selection().unwrap();

    assert!(calendar.selected_dates().is_empty());
}

#[test]
fn remove_from_selection_removes_date() {
    let _scope = test_scope();
    let (calendar, _day_button, provider) = create_target(CalendarSelectionMode::MultipleRange, None);
    calendar.selected_dates().add(date1());
    calendar.selected_dates().add(date2());

    provider.remove_from_selection().unwrap();

    assert_eq!(1, calendar.selected_dates().count());
    assert_eq!(date2(), calendar.selected_dates().get(0));
}

#[test]
fn remove_from_selection_is_no_op_when_selection_mode_none() {
    let _scope = test_scope();
    let (calendar, _day_button, provider) = create_target(CalendarSelectionMode::None, None);

    provider.remove_from_selection().unwrap();

    assert!(calendar.selected_dates().is_empty());
}
