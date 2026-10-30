//! Tests of `AutoSuggestBox` (WinUI; docs/porting/winui-controls.md, Input), adapted from the
//! tests of #286 (`tests/winui_auto_suggest_box.rs`).

use crate::automation::peers::{AutoSuggestBoxAutomationPeer, AutomationControlType, ControlAutomationPeer};
use crate::automation::provider::ProviderAdapter;
use crate::input_test_support::auto_suggest_box_template;
use crate::primitives::{Popup, TemplatedControl};
use crate::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use crate::{
    AutoCompleteBox, AutoSuggestBox, AutoSuggestionBoxTextChangeReason as Reason, Border, Button, Control, ItemsSource,
    ListBox, Symbol, SymbolIcon, TextBlock, TextBox,
};
use ferroui_base::collections::FerroList;
use ferroui_base::input::{InputElement, Key, KeyEventArgs};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::threading::Dispatcher;
use ferroui_base::Ref;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

fn start() -> UnitTestApplicationScope {
    crate::register_types();
    UnitTestApplication::start(TestServices::mock_threading_interface())
}

struct Parts {
    text_box: Ref<TextBox>,
    list: Ref<ListBox>,
    container: Ref<Border>,
    query: Ref<Button>,
    _popup: Ref<Popup>,
}

/// Applies the test template to `control` and returns its parts.
fn parts(control: &AutoSuggestBox) -> Parts {
    control.set_template(Some(auto_suggest_box_template()));
    let result = Rc::new(RefCell::new(None));
    let found = result.clone();
    let token = control.template_applied(move |_, args| {
        let scope = args.name_scope();
        *found.borrow_mut() = Some(Parts {
            text_box: scope.find_as("TextBox").unwrap(),
            list: scope.find_as("SuggestionsList").unwrap(),
            container: scope.find_as("SuggestionsContainer").unwrap(),
            query: scope.find_as("QueryButton").unwrap(),
            _popup: scope.find_as("SuggestionsPopup").unwrap(),
        });
    });
    control.apply_template();
    control.remove_handler(TemplatedControl::template_applied_event(), token);
    let parts = result.borrow_mut().take().unwrap();
    parts
}

/// Raises the pending text-changed event.
fn fire() {
    for timer in Dispatcher::timers_for_unit_tests().into_iter().filter(|timer| timer.interval() == Duration::from_millis(150)) {
        Dispatcher::force_fire_timer_for_unit_tests(&timer);
    }
}

fn key(control: &AutoSuggestBox, key: Key) -> KeyEventArgs {
    let mut args = KeyEventArgs::new();
    args.key = key;
    args.set_routed_event(Some(InputElement::key_down_event()));
    control.raise_event(&args);
    args
}

#[test]
fn the_defaults_are_winui_and_auto_complete_box_is_another_control() {
    let _app = start();
    let existing = AutoCompleteBox::new();
    let filter = existing.filter_mode();
    let control = AutoSuggestBox::new();
    assert_eq!("", control.text());
    assert_eq!("", control.text_member_path());
    assert!(control.update_text_on_select());
    assert!(!control.is_suggestion_list_open());
    assert_eq!(f64::INFINITY, control.max_suggestion_list_height());
    assert!(control.query_icon().is_none());
    assert!(!control.focusable());
    assert_eq!(0, Reason::UserInput as i32);
    assert_eq!(1, Reason::ProgrammaticChange as i32);
    assert_eq!(2, Reason::SuggestionChosen as i32);
    assert!(!std::ptr::eq(AutoSuggestBox::TYPE, AutoCompleteBox::TYPE));
    assert_eq!(filter, AutoCompleteBox::new().filter_mode());
}

#[test]
fn text_changes_within_150_ms_raise_one_event_with_the_reason_of_the_change() {
    let _app = start();
    let control = AutoSuggestBox::new();
    let parts = parts(&control);
    let events = Rc::new(RefCell::new(Vec::new()));
    let recorded = events.clone();
    control.text_changed(move |args| recorded.borrow_mut().push(args.clone()));
    parts.text_box.set_text(Some("a"));
    parts.text_box.set_text(Some("ab"));
    assert!(events.borrow().is_empty());
    assert_eq!("ab", control.text());
    let timers = Dispatcher::timers_for_unit_tests();
    assert_eq!(1, timers.iter().filter(|timer| timer.interval() == Duration::from_millis(150)).count());
    fire();
    assert_eq!(1, events.borrow().len());
    assert_eq!(Reason::UserInput, events.borrow()[0].reason());
    assert!(events.borrow()[0].check_current());
    control.set_text("programmed");
    assert!(!events.borrow()[0].check_current());
    fire();
    assert_eq!(Reason::ProgrammaticChange, events.borrow()[1].reason());
    control.set_items_source(Some(ItemsSource::from_strs(["selected"])));
    parts.list.set_selected_index(0);
    assert_eq!("selected", control.text());
    fire();
    assert_eq!(Reason::SuggestionChosen, events.borrow()[2].reason());
}

#[test]
fn the_arguments_and_the_timer_do_not_keep_the_box_alive() {
    let _app = start();
    let control = AutoSuggestBox::new();
    let parts = parts(&control);
    let events = Rc::new(RefCell::new(Vec::new()));
    let recorded = events.clone();
    control.text_changed(move |args| recorded.borrow_mut().push(args.clone()));
    parts.text_box.set_text(Some("current"));
    fire();
    let args = events.borrow()[0].clone();
    assert!(args.check_current());
    args.set_reason(Reason::ProgrammaticChange);
    assert_eq!(Reason::ProgrammaticChange, events.borrow()[0].reason());
    parts.text_box.set_text(Some("pending"));
    assert!(!args.check_current());
    let weak = control.downgrade();
    drop(control);
    assert!(weak.upgrade().is_none());
    assert!(!args.check_current());
    assert!(Dispatcher::timers_for_unit_tests().iter().all(|timer| timer.interval() != Duration::from_millis(150)));
    parts.text_box.set_text(Some("old detached text box"));
}

#[test]
fn a_chosen_suggestion_is_the_text_before_its_event_and_enter_submits_it() {
    let _app = start();
    let control = AutoSuggestBox::new();
    let parts = parts(&control);
    control.set_items_source(Some(ItemsSource::from_strs(["Alpha", "Beta"])));
    let events = Rc::new(RefCell::new(Vec::new()));
    let log = events.clone();
    let weak = control.downgrade();
    control.suggestion_chosen(move |args| {
        let owner = weak.upgrade().unwrap();
        assert_eq!(*args.selected_item.downcast_ref::<String>().unwrap(), owner.text());
        log.borrow_mut().push("chosen".to_string());
    });
    let expected = control.items().get_at(1).unwrap();
    let log = events.clone();
    let weak = control.downgrade();
    control.query_submitted(move |args| {
        assert_eq!("Beta", args.query_text);
        assert!(Rc::ptr_eq(args.chosen_suggestion.as_ref().unwrap(), &expected));
        assert!(weak.upgrade().unwrap().is_suggestion_list_open());
        log.borrow_mut().push("query".to_string());
    });
    control.set_is_suggestion_list_open(true);
    parts.list.set_selected_index(1);
    assert!(key(&control, Key::Enter).handled());
    assert_eq!(["chosen", "query"], events.borrow()[..]);
    assert!(!control.is_suggestion_list_open());
    assert_eq!(-1, parts.list.selected_index());
}

#[test]
fn the_query_button_submits_the_text_without_the_selected_suggestion() {
    let _app = start();
    let control = AutoSuggestBox::new();
    let parts = parts(&control);
    assert!(!parts.query.is_visible());
    control.set_query_icon(Some(SymbolIcon::with_symbol(Symbol::Find).upcast()));
    control.set_items_source(Some(ItemsSource::from_strs(["chosen"])));
    parts.list.set_selected_index(0);
    let calls = Rc::new(Cell::new(0));
    let count = calls.clone();
    control.query_submitted(move |args| {
        assert_eq!("chosen", args.query_text);
        assert!(args.chosen_suggestion.is_none());
        count.set(count.get() + 1);
    });
    assert!(parts.query.is_visible());
    assert!(parts.query.content().is_some());
    parts.query.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert_eq!(1, calls.get());
    assert_eq!(-1, parts.list.selected_index());
    control.set_query_icon(None);
    assert!(!parts.query.is_visible());
    assert!(parts.query.content().is_none());
}

#[test]
fn without_update_text_on_select_the_text_stays_and_the_suggestion_is_chosen() {
    let _app = start();
    let control = AutoSuggestBox::new();
    let parts = parts(&control);
    control.set_text("original");
    fire();
    control.set_update_text_on_select(false);
    let changes = Rc::new(Cell::new(0));
    let count = changes.clone();
    control.text_changed(move |_| count.set(count.get() + 1));
    let choices = Rc::new(Cell::new(0));
    let count = choices.clone();
    control.suggestion_chosen(move |_| count.set(count.get() + 1));
    control.set_items_source(Some(ItemsSource::from_strs(["Alpha"])));
    parts.list.set_selected_index(0);
    fire();
    assert_eq!("original", control.text());
    assert_eq!(0, changes.get());
    assert_eq!(1, choices.get());
}

#[test]
fn the_text_member_path_reads_the_text_of_a_suggestion() {
    let _app = start();
    let control = AutoSuggestBox::new();
    let parts = parts(&control);
    let item = TextBlock::new();
    item.set_text(Some("Metadata name"));
    let item = Control::boxed(item);
    control.set_items_source(Some(ItemsSource::from_items([Some(item.clone())])));
    control.set_text_member_path("Text");
    parts.list.set_selected_index(0);
    assert_eq!("Metadata name", control.text());
    assert!(Rc::ptr_eq(parts.list.selected_item().as_ref().unwrap(), &item));
}

#[test]
fn the_arrow_keys_go_back_to_the_typed_text_and_escape_or_tab_close_the_list() {
    let _app = start();
    let control = AutoSuggestBox::new();
    let parts = parts(&control);
    parts.text_box.set_text(Some("typed"));
    control.set_items_source(Some(ItemsSource::from_strs(["Alpha", "Beta"])));
    control.set_is_suggestion_list_open(true);
    assert!(key(&control, Key::Down).handled());
    assert_eq!("Alpha", control.text());
    key(&control, Key::Down);
    assert_eq!("Beta", control.text());
    key(&control, Key::Down);
    assert_eq!("typed", control.text());
    assert_eq!(-1, parts.list.selected_index());
    key(&control, Key::Up);
    assert_eq!("Beta", control.text());
    assert!(key(&control, Key::Escape).handled());
    assert_eq!("typed", control.text());
    assert!(!control.is_suggestion_list_open());
    control.set_is_suggestion_list_open(true);
    key(&control, Key::Down);
    assert!(!key(&control, Key::Tab).handled());
    assert_eq!("typed", control.text());
    assert!(!control.is_suggestion_list_open());
}

#[test]
fn a_single_suggestion_stays_selected() {
    let _app = start();
    let control = AutoSuggestBox::new();
    let parts = parts(&control);
    control.set_items_source(Some(ItemsSource::from_strs(["Only"])));
    control.set_is_suggestion_list_open(true);
    key(&control, Key::Down);
    key(&control, Key::Down);
    key(&control, Key::Up);
    assert_eq!(0, parts.list.selected_index());
}

#[test]
fn the_suggestions_are_not_filtered_and_follow_the_collection() {
    let _app = start();
    let control = AutoSuggestBox::new();
    let parts = parts(&control);
    let items = Rc::new(FerroList::new());
    items.add("Not a prefix".to_string());
    items.add("Other".to_string());
    control.set_items_source(Some(ItemsSource::from(items.clone())));
    parts.text_box.set_text(Some("zzzz"));
    assert_eq!(2, parts.list.items().count());
    items.add("New".to_string());
    assert_eq!(3, parts.list.items().count());
    // UpdateSuggestionListSize: the maximum height is the container's.
    control.set_max_suggestion_list_height(120.0);
    assert_eq!(120.0, parts.container.max_height());
    control.set_max_suggestion_list_height(-1.0);
    assert_eq!(f64::INFINITY, parts.container.max_height());
    control.set_items_source(Some(ItemsSource::from_strs(["Replacement"])));
    assert_eq!(1, parts.list.items().count());
}

#[test]
fn the_parts_of_a_replaced_template_no_longer_change_the_box() {
    let _app = start();
    let control = AutoSuggestBox::new();
    let old = parts(&control);
    let current = parts(&control);
    control.set_text("current");
    let calls = Rc::new(Cell::new(0));
    let count = calls.clone();
    control.query_submitted(move |_| count.set(count.get() + 1));
    old.text_box.set_text(Some("detached"));
    old.query.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    old.list.set_items_source(Some(ItemsSource::from_strs(["stale"])));
    old.list.set_selected_index(0);
    assert_eq!("current", control.text());
    assert_eq!(0, calls.get());
    current.query.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert_eq!(1, calls.get());
}

#[test]
fn a_text_handler_can_replace_the_text_and_dispose_its_subscription() {
    let _app = start();
    let control = AutoSuggestBox::new();
    let parts = parts(&control);
    let calls = Rc::new(Cell::new(0));
    let count = calls.clone();
    let weak = control.downgrade();
    let subscription = control.text_changed(move |args| {
        count.set(count.get() + 1);
        if args.reason() == Reason::UserInput {
            weak.upgrade().unwrap().set_text("normalized");
            assert!(!args.check_current());
        }
    });
    parts.text_box.set_text(Some("user"));
    fire();
    assert_eq!("normalized", control.text());
    fire();
    assert_eq!(2, calls.get());
    subscription.dispose();
    control.set_text("after disposal");
    fire();
    assert_eq!(2, calls.get());
}

#[test]
fn the_peer_is_a_group_whose_invoke_submits_the_query() {
    let _app = start();
    let control = AutoSuggestBox::new();
    let _parts = parts(&control);
    control.set_text("query");
    let calls = Rc::new(Cell::new(0));
    let count = calls.clone();
    control.query_submitted(move |args| {
        assert_eq!("query", args.query_text);
        assert!(args.chosen_suggestion.is_none());
        count.set(count.get() + 1);
    });
    let peer = ControlAutomationPeer::create_peer_for_element(&control);
    assert_eq!(AutomationControlType::Group, peer.get_automation_control_type());
    assert_eq!("AutoSuggestBox", peer.get_class_name());
    ProviderAdapter::as_invoke_provider(peer.cast::<AutoSuggestBoxAutomationPeer>().unwrap()).invoke().unwrap();
    assert_eq!(1, calls.get());
}
