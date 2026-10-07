//! Port of `Automation/ComplexControlAutomationPeerTests.cs` of the controls
//! unit tests: `AutoCompleteBoxAutomationPeerTests` and
//! `CalendarAutomationPeerTests`.

use super::{
    AutoCompleteBoxAutomationPeer, AutomationControlType, AutomationPeer, CalendarAutomationPeer,
    ControlAutomationPeer,
};
use crate::automation::provider::{IExpandCollapseProvider, IInvokeProvider, ISelectionProvider, IValueProvider};
use crate::automation::{
    AutomationPropertyChangedEventArgs, ExpandCollapsePatternIdentifiers, ExpandCollapseState,
    SelectionPatternIdentifiers, ValuePatternIdentifiers,
};
use crate::calendar::{Calendar, CalendarDayButton, CalendarItem, CalendarSelectionMode};
use crate::test_support::{string_of, test_scope};
use crate::{AutoCompleteBox, Control, Grid, ItemsSource, Panel, TextBlock};
use ferroui_base::reactive::IDisposable;
use ferroui_base::utilities::{CultureInfo, DateTime, TestCultureDataProvider};
use ferroui_base::{AnyValue, BoxedValue, FerroLocator, Ref};
use std::cell::RefCell;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

fn create_peer(control: &Control) -> Ref<AutomationPeer> {
    ControlAutomationPeer::create_peer_for_element(control)
}

fn state_of(value: Option<&BoxedValue>) -> Option<ExpandCollapseState> {
    let value: &dyn AnyValue = &**value?;
    value.downcast_ref::<ExpandCollapseState>().copied()
}

mod auto_complete_box_automation_peer_tests {
    use super::*;

    #[test]
    fn creates_auto_complete_box_automation_peer() {
        let _scope = test_scope();
        let target = AutoCompleteBox::new();
        let peer = create_peer(&target);

        assert!(std::ptr::eq(peer.get_type(), AutoCompleteBoxAutomationPeer::TYPE));
    }

    #[test]
    fn implements_i_expand_collapse_and_i_value_providers() {
        let _scope = test_scope();
        let target = AutoCompleteBox::new();
        let peer = create_peer(&target);

        assert!(peer.get_provider::<dyn IExpandCollapseProvider>().is_some());
        assert!(peer.get_provider::<dyn IValueProvider>().is_some());
        assert!(peer.get_provider::<dyn IInvokeProvider>().is_none());
    }

    #[test]
    fn control_type_is_group_and_class_name_is_auto_complete_box() {
        let _scope = test_scope();
        let target = AutoCompleteBox::new();
        let peer = create_peer(&target).cast::<AutoCompleteBoxAutomationPeer>().unwrap();

        assert_eq!(AutomationControlType::Group, peer.get_automation_control_type());
        assert_eq!("AutoCompleteBox", peer.get_class_name());
    }

    #[test]
    fn expand_collapse_tracks_is_drop_down_open() {
        let _scope = test_scope();
        let target = AutoCompleteBox::new();
        target.set_items_source(Some(ItemsSource::from_strs(["alpha"])));
        target.set_text(Some("a"));
        let peer = create_peer(&target).get_provider::<dyn IExpandCollapseProvider>().unwrap();
        assert!(peer.shows_menu());

        target.set_is_drop_down_open(false);
        assert!(!target.is_drop_down_open());

        peer.expand().unwrap();
        assert!(target.is_drop_down_open());
        assert_eq!(ExpandCollapseState::Expanded, peer.expand_collapse_state());

        peer.collapse().unwrap();
        assert!(!target.is_drop_down_open());
        assert_eq!(ExpandCollapseState::Collapsed, peer.expand_collapse_state());
    }

    #[test]
    fn value_tracks_and_sets_text() {
        let _scope = test_scope();
        let target = AutoCompleteBox::new();
        target.set_text(Some("one"));
        let peer = create_peer(&target).get_provider::<dyn IValueProvider>().unwrap();

        assert_eq!(Some("one".to_string()), peer.value());
        peer.set_value(Some("two")).unwrap();
        assert_eq!(Some("two".to_string()), target.text());
        assert_eq!(Some("two".to_string()), peer.value());
    }

    #[test]
    fn value_provider_is_mutable() {
        let _scope = test_scope();
        let target = AutoCompleteBox::new();
        let peer = create_peer(&target).get_provider::<dyn IValueProvider>().unwrap();

        assert!(!peer.is_read_only());
    }

    #[test]
    fn property_change_events_raise_for_drop_down_and_text() {
        let _scope = test_scope();
        let target = AutoCompleteBox::new();
        let peer = create_peer(&target).cast::<AutoCompleteBoxAutomationPeer>().unwrap();

        let expand_collapse_changed: Rc<RefCell<Option<AutomationPropertyChangedEventArgs>>> =
            Rc::new(RefCell::new(None));
        let value_changed: Rc<RefCell<Option<AutomationPropertyChangedEventArgs>>> = Rc::new(RefCell::new(None));
        let (expand_collapse_sink, value_sink) = (expand_collapse_changed.clone(), value_changed.clone());
        peer.property_changed(move |e| {
            if e.property() == ExpandCollapsePatternIdentifiers::expand_collapse_state_property() {
                *expand_collapse_sink.borrow_mut() = Some(e.clone());
            } else if e.property() == ValuePatternIdentifiers::value_property() {
                *value_sink.borrow_mut() = Some(e.clone());
            }
        });

        target.set_is_drop_down_open(true);
        let expand_collapse_changed = expand_collapse_changed.borrow();
        assert!(expand_collapse_changed.is_some());
        let expand_collapse_changed = expand_collapse_changed.as_ref().unwrap();
        assert!(
            expand_collapse_changed.property() == ExpandCollapsePatternIdentifiers::expand_collapse_state_property()
        );
        assert_eq!(Some(ExpandCollapseState::Collapsed), state_of(expand_collapse_changed.old_value()));
        assert_eq!(Some(ExpandCollapseState::Expanded), state_of(expand_collapse_changed.new_value()));

        target.set_text(Some("query"));
        let value_changed = value_changed.borrow();
        assert!(value_changed.is_some());
        let value_changed = value_changed.as_ref().unwrap();
        assert!(value_changed.property() == ValuePatternIdentifiers::value_property());
        assert_eq!(Some(String::new()), value_changed.old_value().and_then(string_of));
        assert_eq!(Some("query".to_string()), value_changed.new_value().and_then(string_of));
    }
}

mod calendar_automation_peer_tests {
    use super::*;

    #[test]
    fn creates_calendar_automation_peer() {
        let _scope = test_scope();
        let target = Calendar::new();
        let peer = create_peer(&target);

        assert!(std::ptr::eq(peer.get_type(), CalendarAutomationPeer::TYPE));
    }

    #[test]
    fn implements_i_selection_and_i_value_providers() {
        let _scope = test_scope();
        let target = Calendar::new();
        let peer = create_peer(&target);

        assert!(peer.get_provider::<dyn ISelectionProvider>().is_some());
        assert!(peer.get_provider::<dyn IValueProvider>().is_some());
    }

    #[test]
    fn control_type_is_calendar_and_class_name_is_calendar() {
        let _scope = test_scope();
        let target = Calendar::new();
        let peer = create_peer(&target).cast::<CalendarAutomationPeer>().unwrap();

        assert_eq!(AutomationControlType::Calendar, peer.get_automation_control_type());
        assert_eq!("Calendar", peer.get_class_name());
    }

    #[test]
    fn can_select_multiple_reflects_selection_mode() {
        for (selection_mode, can_select_multiple) in [
            (CalendarSelectionMode::SingleDate, false),
            (CalendarSelectionMode::SingleRange, true),
            (CalendarSelectionMode::MultipleRange, true),
            (CalendarSelectionMode::None, false),
        ] {
            let _scope = test_scope();
            let target = Calendar::new();
            target.set_selection_mode(selection_mode);
            let peer = create_peer(&target).get_provider::<dyn ISelectionProvider>().unwrap();

            assert_eq!(can_select_multiple, peer.can_select_multiple());
            assert!(!peer.is_selection_required());
        }
    }

    #[test]
    fn selection_events_include_selection_and_value_properties() {
        let _scope = test_scope();
        let target = Calendar::new();
        target.set_selection_mode(CalendarSelectionMode::SingleDate);
        let peer = create_peer(&target).cast::<CalendarAutomationPeer>().unwrap();

        let selection_changed: Rc<RefCell<Option<AutomationPropertyChangedEventArgs>>> = Rc::new(RefCell::new(None));
        let value_changed: Rc<RefCell<Option<AutomationPropertyChangedEventArgs>>> = Rc::new(RefCell::new(None));
        let (selection_sink, value_sink) = (selection_changed.clone(), value_changed.clone());
        peer.property_changed(move |e| {
            if e.property() == SelectionPatternIdentifiers::selection_property() {
                *selection_sink.borrow_mut() = Some(e.clone());
            } else if e.property() == ValuePatternIdentifiers::value_property() {
                *value_sink.borrow_mut() = Some(e.clone());
            }
        });

        target.set_selected_date(Some(DateTime::new(2010, 1, 1)));

        let selection_changed = selection_changed.borrow();
        let value_changed = value_changed.borrow();
        assert!(selection_changed.is_some());
        assert!(value_changed.is_some());
        assert!(selection_changed.as_ref().unwrap().property() == SelectionPatternIdentifiers::selection_property());
        assert!(value_changed.as_ref().unwrap().property() == ValuePatternIdentifiers::value_property());
    }

    /// Restores the current culture and the locator when dropped (the
    /// `finally` block of the reference test).
    struct CultureScope {
        previous_culture: CultureInfo,
        locator: Rc<dyn IDisposable>,
    }

    impl Drop for CultureScope {
        fn drop(&mut self) {
            CultureInfo::set_current_culture(self.previous_culture.clone());
            self.locator.dispose();
        }
    }

    #[test]
    fn value_joins_selected_dates_with_current_culture() {
        let _scope = test_scope();
        let _previous_culture =
            CultureScope { previous_culture: CultureInfo::current_culture(), locator: FerroLocator::enter_scope() };
        // The culture data of en-GB comes from the culture data provider of the tests.
        TestCultureDataProvider::register();
        CultureInfo::set_current_culture(CultureInfo::get_culture_info("en-GB"));

        let selected_dates = [DateTime::new(2026, 5, 7), DateTime::new(2026, 5, 8)];
        let target = Calendar::new();
        target.set_selection_mode(CalendarSelectionMode::MultipleRange);
        let peer = create_peer(&target).cast::<CalendarAutomationPeer>().unwrap();

        for date in selected_dates {
            target.selected_dates().add(date);
        }

        let culture = CultureInfo::current_culture();
        assert_eq!(
            Some(
                selected_dates
                    .iter()
                    .map(|x| x.to_string_provider(&culture))
                    .collect::<Vec<_>>()
                    .join(culture.text_info().list_separator())
            ),
            peer.value()
        );
    }

    #[test]
    fn set_value_throws_not_supported() {
        let _scope = test_scope();
        let target = Calendar::new();
        let peer = create_peer(&target).get_provider::<dyn IValueProvider>().unwrap();

        assert!(catch_unwind(AssertUnwindSafe(|| peer.set_value(Some("2026-01-01")))).is_err());
    }

    #[test]
    fn value_is_read_only() {
        let _scope = test_scope();
        let target = Calendar::new();
        let peer = create_peer(&target).get_provider::<dyn IValueProvider>().unwrap();

        assert!(peer.is_read_only());
    }

    #[test]
    fn get_selection_returns_empty_when_day_button_not_realized() {
        let _scope = test_scope();
        let target = Calendar::new();
        target.set_selection_mode(CalendarSelectionMode::SingleDate);
        target.set_selected_date(Some(DateTime::new(2026, 5, 7)));
        let peer = create_peer(&target).cast::<CalendarAutomationPeer>().unwrap();

        let selection = peer.get_selection();

        assert!(selection.is_empty());
    }

    #[test]
    fn get_selection_returns_realized_day_button_peers() {
        let _scope = test_scope();
        let selected_date = DateTime::new(2026, 5, 7);
        let target = Calendar::new();
        target.set_selection_mode(CalendarSelectionMode::SingleDate);
        target.set_display_date(DateTime::new(2026, 5, 1));
        target.set_selected_date(Some(selected_date));
        let month_view = Grid::new();
        let calendar_item = CalendarItem::new();
        calendar_item.set_owner(Some(&target));
        calendar_item.set_month_view(Some(month_view.clone()));
        let root = Panel::new();
        root.children().add(calendar_item.clone());
        target.set_root(Some(root));

        for _ in 0..Calendar::COLUMNS_PER_MONTH {
            month_view.children().add(TextBlock::new());
        }

        for i in 0..Calendar::ROWS_PER_MONTH * Calendar::COLUMNS_PER_MONTH - Calendar::COLUMNS_PER_MONTH {
            let day_button = CalendarDayButton::new();
            day_button.set_owner(Some(&target));
            let date = if i == 0 { selected_date } else { selected_date.add_days(f64::from(i + 1)) };
            day_button.set_data_context(Some(Rc::new(date) as BoxedValue));
            month_view.children().add(day_button);
        }

        let peer = create_peer(&target).cast::<CalendarAutomationPeer>().unwrap();
        let selection = peer.get_selection();

        assert_eq!(1, selection.len());
    }
}
