//! Event, lifetime, selection and native-part contracts of the suggestion editor.

use ferroui_controls::automation::peers::{AutoSuggestBoxAutomationPeer};
use ferroui_base::collections::FerroList;
use ferroui_base::input::{InputElement, Key, KeyEventArgs};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::threading::Dispatcher;
use ferroui_base::Ref;
use ferroui_controls::automation::peers::{AutomationControlType, ControlAutomationPeer};
use ferroui_controls::automation::provider::ProviderAdapter;
use ferroui_controls::primitives::{Popup, TemplatedControl};
use ferroui_controls::templates::FuncControlTemplate;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{
    AutoSuggestBox, AutoSuggestionBoxTextChangeReason as Reason, Symbol, SymbolIcon,
};
use ferroui_controls::{
    AutoCompleteBox, Button, Control, ItemsSource, ListBox, TextBlock, TextBox,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

fn start() -> ferroui_controls::testing::UnitTestApplicationScope {
    ferroui_controls::register_types();
    UnitTestApplication::start(TestServices::mock_threading_interface())
}

struct Parts {
    editor: Ref<TextBox>,
    list: Ref<ListBox>,
    query: Ref<Button>,
    popup: Ref<Popup>,
}
fn parts(control: &AutoSuggestBox) -> Parts {
    let result = Rc::new(RefCell::new(None));
    let found = result.clone();
    let token = control.template_applied(move |_, args| {
        let scope = args.name_scope();
        *found.borrow_mut() = Some(Parts {
            editor: scope.find_as("PART_TextBox").unwrap(),
            list: scope.find_as("PART_SuggestionsList").unwrap(),
            query: scope.find_as("PART_QueryButton").unwrap(),
            popup: scope.find_as("PART_Popup").unwrap(),
        });
    });
    control.apply_template();
    control.remove_handler(TemplatedControl::template_applied_event(), token);
    let parts = result.borrow_mut().take().unwrap();
    parts
}
fn fire() {
    for timer in Dispatcher::timers_for_unit_tests()
        .into_iter()
        .filter(|timer| timer.interval() == Duration::from_millis(150))
    {
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
fn defaults_and_existing_autocomplete_contract_remain_distinct() {
    let _app = start();
    let existing = AutoCompleteBox::new();
    let filter = existing.filter_mode();
    let control = AutoSuggestBox::new();
    assert_eq!(control.text(), "");
    assert_eq!(control.text_member_path(), "");
    assert!(control.update_text_on_select());
    assert!(!control.is_suggestion_list_open());
    assert_eq!(control.max_suggestion_list_height(), f64::INFINITY);
    assert!(control.query_icon().is_none());
    assert_eq!(Reason::UserInput as i32, 0);
    assert_eq!(Reason::ProgrammaticChange as i32, 1);
    assert_eq!(Reason::SuggestionChosen as i32, 2);
    assert_ne!(AutoSuggestBox::TYPE, AutoCompleteBox::TYPE);
    assert_eq!(AutoCompleteBox::new().filter_mode(), filter);
}

#[test]
fn text_changes_coalesce_for_150ms_and_preserve_user_programmatic_and_choice_reasons() {
    let _app = start();
    let control = AutoSuggestBox::new();
    let parts = parts(&control);
    let events = Rc::new(RefCell::new(Vec::new()));
    let recorded = events.clone();
    control.text_changed(move |args| recorded.borrow_mut().push(args.clone()));
    parts.editor.set_text(Some("a"));
    parts.editor.set_text(Some("ab"));
    assert!(events.borrow().is_empty());
    assert_eq!(control.text(), "ab");
    let timers = Dispatcher::timers_for_unit_tests();
    assert_eq!(
        timers
            .iter()
            .filter(|timer| timer.interval() == Duration::from_millis(150))
            .count(),
        1
    );
    fire();
    assert_eq!(events.borrow().len(), 1);
    assert_eq!(events.borrow()[0].reason(), Reason::UserInput);
    assert!(events.borrow()[0].check_current());
    control.set_text("programmed");
    assert!(!events.borrow()[0].check_current());
    fire();
    assert_eq!(events.borrow()[1].reason(), Reason::ProgrammaticChange);
    control.set_items_source(Some(ItemsSource::from_strs(["selected"])));
    parts.list.set_selected_index(0);
    assert_eq!(control.text(), "selected");
    fire();
    assert_eq!(events.borrow()[2].reason(), Reason::SuggestionChosen);
}

#[test]
fn check_current_and_timer_do_not_keep_the_editor_alive() {
    let _app = start();
    let control = AutoSuggestBox::new();
    let parts = parts(&control);
    let events = Rc::new(RefCell::new(Vec::new()));
    let recorded = events.clone();
    control.text_changed(move |args| recorded.borrow_mut().push(args.clone()));
    parts.editor.set_text(Some("current"));
    fire();
    let args = events.borrow()[0].clone();
    assert!(args.check_current());
    args.set_reason(Reason::ProgrammaticChange);
    assert_eq!(events.borrow()[0].reason(), Reason::ProgrammaticChange);
    parts.editor.set_text(Some("pending"));
    assert!(!args.check_current());
    let weak = control.downgrade();
    drop(control);
    assert!(weak.upgrade().is_none());
    assert!(!args.check_current());
    assert!(Dispatcher::timers_for_unit_tests()
        .iter()
        .all(|timer| timer.interval() != Duration::from_millis(150)));
    parts.editor.set_text(Some("old detached editor"));
}

#[test]
fn suggestion_choice_updates_text_before_handler_and_enter_submits_selected_identity() {
    let _app = start();
    let control = AutoSuggestBox::new();
    let parts = parts(&control);
    control.set_items_source(Some(ItemsSource::from_strs(["Alpha", "Beta"])));
    let events = Rc::new(RefCell::new(Vec::new()));
    let log = events.clone();
    let weak = control.downgrade();
    control.suggestion_chosen(move |args| {
        let owner = weak.upgrade().unwrap();
        assert_eq!(
            owner.text(),
            *args.selected_item.downcast_ref::<String>().unwrap()
        );
        log.borrow_mut().push("chosen".to_string());
    });
    let selected = control.items().get_at(1).unwrap();
    let expected = selected.clone();
    let log = events.clone();
    let weak = control.downgrade();
    control.query_submitted(move |args| {
        assert_eq!(args.query_text, "Beta");
        assert!(Rc::ptr_eq(
            args.chosen_suggestion.as_ref().unwrap(),
            &expected
        ));
        assert!(weak.upgrade().unwrap().is_suggestion_list_open());
        log.borrow_mut().push("query".to_string());
    });
    control.set_is_suggestion_list_open(true);
    parts.list.set_selected_index(1);
    assert!(key(&control, Key::Enter).handled());
    assert_eq!(*events.borrow(), ["chosen", "query"]);
    assert!(!control.is_suggestion_list_open());
    assert_eq!(parts.list.selected_index(), -1);
}

#[test]
fn query_button_ignores_highlighted_suggestion_and_submits_current_text() {
    let _app = start();
    let control = AutoSuggestBox::new();
    let parts = parts(&control);
    control.set_query_icon(Some(SymbolIcon::with_symbol(Symbol::Find).upcast()));
    control.set_items_source(Some(ItemsSource::from_strs(["chosen"])));
    parts.list.set_selected_index(0);
    let calls = Rc::new(Cell::new(0));
    let count = calls.clone();
    control.query_submitted(move |args| {
        assert_eq!(args.query_text, "chosen");
        assert!(args.chosen_suggestion.is_none());
        count.set(count.get() + 1);
    });
    assert!(parts.query.is_visible());
    parts
        .query
        .raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert_eq!(calls.get(), 1);
    assert_eq!(parts.list.selected_index(), -1);
    control.set_query_icon(None);
    assert!(!parts.query.is_visible());
    assert!(parts.query.content().is_none());
}

#[test]
fn update_text_on_select_false_leaves_text_and_still_raises_suggestion_chosen() {
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
    assert_eq!(control.text(), "original");
    assert_eq!(changes.get(), 0);
    assert_eq!(choices.get(), 1);
}

#[test]
fn text_member_path_reads_native_property_metadata_and_preserves_item_identity() {
    let _app = start();
    let control = AutoSuggestBox::new();
    let parts = parts(&control);
    let item = TextBlock::new();
    item.set_text(Some("Metadata name"));
    let item = Control::boxed(item);
    control.set_items_source(Some(ItemsSource::from_items([Some(item.clone())])));
    control.set_text_member_path("Text");
    parts.list.set_selected_index(0);
    assert_eq!(control.text(), "Metadata name");
    assert!(Rc::ptr_eq(
        parts.list.selected_item().as_ref().unwrap(),
        &item
    ));
}

#[test]
fn arrow_navigation_restores_user_text_at_edges_and_escape_or_tab_close_the_list() {
    let _app = start();
    let control = AutoSuggestBox::new();
    let parts = parts(&control);
    parts.editor.set_text(Some("typed"));
    control.set_items_source(Some(ItemsSource::from_strs(["Alpha", "Beta"])));
    control.set_is_suggestion_list_open(true);
    assert!(key(&control, Key::Down).handled());
    assert_eq!(control.text(), "Alpha");
    key(&control, Key::Down);
    assert_eq!(control.text(), "Beta");
    key(&control, Key::Down);
    assert_eq!(control.text(), "typed");
    assert_eq!(parts.list.selected_index(), -1);
    key(&control, Key::Up);
    assert_eq!(control.text(), "Beta");
    assert!(key(&control, Key::Escape).handled());
    assert_eq!(control.text(), "typed");
    assert!(!control.is_suggestion_list_open());
    control.set_is_suggestion_list_open(true);
    key(&control, Key::Down);
    assert!(!key(&control, Key::Tab).handled());
    assert_eq!(control.text(), "typed");
    assert!(!control.is_suggestion_list_open());
}

#[test]
fn a_single_suggestion_does_not_wrap_out_of_selection() {
    let _app = start();
    let control = AutoSuggestBox::new();
    let parts = parts(&control);
    control.set_items_source(Some(ItemsSource::from_strs(["Only"])));
    control.set_is_suggestion_list_open(true);
    key(&control, Key::Down);
    key(&control, Key::Down);
    key(&control, Key::Up);
    assert_eq!(parts.list.selected_index(), 0);
}

#[test]
fn application_suggestions_are_unfiltered_and_observable_collections_remain_live() {
    let _app = start();
    let control = AutoSuggestBox::new();
    let parts = parts(&control);
    let items = Rc::new(FerroList::new());
    items.add("Not a prefix".to_string());
    items.add("Other".to_string());
    control.set_items_source(Some(ItemsSource::from(items.clone())));
    parts.editor.set_text(Some("zzzz"));
    assert_eq!(parts.list.items().count(), 2);
    items.add("New".to_string());
    assert_eq!(parts.list.items().count(), 3);
    control.set_max_suggestion_list_height(120.0);
    assert_eq!(parts.list.max_height(), 120.0);
    assert!(parts.popup.is_light_dismiss_enabled());
    control.set_items_source(Some(ItemsSource::from_strs(["Replacement"])));
    assert_eq!(parts.list.items().count(), 1);
}

#[test]
fn retemplate_removes_old_part_subscriptions() {
    let _app = start();
    let control = AutoSuggestBox::new();
    let old = parts(&control);
    let template = control.template();
    control.set_template(Some(FuncControlTemplate::for_type::<AutoSuggestBox>(
        |_, _| TextBox::new().upcast(),
    )));
    control.apply_template();
    control.set_template(template);
    let current = parts(&control);
    control.set_text("current");
    let calls = Rc::new(Cell::new(0));
    let count = calls.clone();
    control.query_submitted(move |_| count.set(count.get() + 1));
    old.editor.set_text(Some("detached"));
    old.query
        .raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    old.list
        .set_items_source(Some(ItemsSource::from_strs(["stale"])));
    old.list.set_selected_index(0);
    assert_eq!(control.text(), "current");
    assert_eq!(calls.get(), 0);
    current
        .query
        .raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert_eq!(calls.get(), 1);
}

#[test]
fn text_handler_can_replace_text_and_dispose_its_subscription_without_borrow_panics() {
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
    parts.editor.set_text(Some("user"));
    fire();
    assert_eq!(control.text(), "normalized");
    fire();
    assert_eq!(calls.get(), 2);
    subscription.dispose();
    control.set_text("after disposal");
    fire();
    assert_eq!(calls.get(), 2);
}

#[test]
fn automation_invoke_submits_a_query_and_exposes_native_group_identity() {
    let _app = start();
    let control = AutoSuggestBox::new();
    let _parts = parts(&control);
    control.set_text("query");
    let calls = Rc::new(Cell::new(0));
    let count = calls.clone();
    control.query_submitted(move |args| {
        assert_eq!(args.query_text, "query");
        assert!(args.chosen_suggestion.is_none());
        count.set(count.get() + 1);
    });
    let peer = ControlAutomationPeer::create_peer_for_element(&control);
    assert_eq!(
        peer.get_automation_control_type(),
        AutomationControlType::Group
    );
    assert_eq!(peer.get_class_name(), "AutoSuggestBox");
    ProviderAdapter::as_invoke_provider(peer.cast::<AutoSuggestBoxAutomationPeer>().unwrap())
        .invoke()
        .unwrap();
    assert_eq!(calls.get(), 1);
}
