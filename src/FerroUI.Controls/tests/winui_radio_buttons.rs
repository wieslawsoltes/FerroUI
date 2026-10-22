//! Public RadioButtons contracts exercised through native layout/input services.

use ferroui_base::input::{InputElement, Key, KeyEventArgs, KeyModifiers, NavigationMethod};
use ferroui_base::{BoxedValue, Rect, Ref, Size};
use ferroui_base::layout::ILayoutManager;
use ferroui_controls::automation::{AutomationProperties};
use ferroui_controls::automation::peers::{AutomationControlType, ControlAutomationPeer};
use ferroui_controls::primitives::SelectingItemsControl;
use ferroui_controls::templates::FuncDataTemplate;
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::RadioButtons;
use ferroui_controls::{Control, ItemsSource, RadioButton, TextBlock, Window};
use std::cell::RefCell;
use std::rc::Rc;

fn start() -> UnitTestApplicationScope {
    ferroui_controls::register_types();
    let mut services = TestServices::styled_window();
    let focus = TestServices::real_focus();
    services.keyboard_device = focus.keyboard_device;
    services.keyboard_navigation = focus.keyboard_navigation;
    services.input_manager = focus.input_manager;
    UnitTestApplication::start(services)
}

fn layout(group: &RadioButtons, width: f64) {
    group.measure(Size::new(width, 1000.0));
    group.arrange(Rect::new(0.0, 0.0, width, group.desired_size().height));
}

fn radio(group: &RadioButtons, index: i32) -> Ref<RadioButton> {
    group.container_from_index(index).unwrap().cast::<RadioButton>().unwrap()
}

fn fixed_group(count: usize) -> Ref<RadioButtons> {
    let group = RadioButtons::new();
    for _ in 0..count {
        let item = RadioButton::new();
        item.set_width(40.0);
        item.set_height(20.0);
        group.items().add(Some(Control::boxed(item)));
    }
    group
}

fn press(control: &InputElement, key: Key, modifiers: KeyModifiers) -> KeyEventArgs {
    let mut args = KeyEventArgs::new();
    args.key = key;
    args.key_modifiers = modifiers;
    args.set_routed_event(Some(InputElement::key_down_event()));
    control.raise_event(&args);
    args
}

fn show(group: &RadioButtons) -> Ref<Window> {
    let window = Window::new();
    window.set_content(Some(Control::boxed(group.to_ref())));
    window.show();
    window.layout_manager().execute_layout_pass();
    window
}

#[test]
fn plain_items_become_radios_and_selection_synchronizes_both_directions() {
    let _app = start();
    let group = RadioButtons::new();
    group.items().add(Some(Rc::new("Red".to_string()) as BoxedValue));
    group.items().add(Some(Rc::new("Blue".to_string()) as BoxedValue));
    assert_eq!(group.selected_index(), -1);
    layout(&group, 300.0);
    assert_eq!(radio(&group, 0).content().unwrap().downcast_ref::<String>().unwrap(), "Red");
    group.set_selected_index(0);
    assert_eq!(radio(&group, 0).is_checked(), Some(true));
    assert_eq!(radio(&group, 1).is_checked(), Some(false));
    radio(&group, 1).on_click();
    assert_eq!(group.selected_index(), 1);
    assert_eq!(group.selected_item().unwrap().downcast_ref::<String>().unwrap(), "Blue");
    assert_eq!(radio(&group, 0).is_checked(), Some(false));
    radio(&group, 1).on_click();
    assert_eq!(group.selected_index(), 1);
    radio(&group, 1).set_is_checked(Some(false));
    assert_eq!(group.selected_index(), -1);
}

#[test]
fn supplied_checked_radio_is_preserved_and_each_group_is_independent() {
    let _app = start();
    let first = fixed_group(3);
    let supplied = first.items_view().get_at(1).unwrap();
    let supplied = Control::from_boxed(&supplied).unwrap().cast::<RadioButton>().unwrap();
    supplied.set_is_checked(Some(true));
    layout(&first, 300.0);
    assert_eq!(radio(&first, 1), supplied);
    assert_eq!(first.selected_index(), 1);
    let other = fixed_group(2);
    layout(&other, 300.0);
    radio(&other, 0).on_click();
    assert_eq!(first.selected_index(), 1);
    assert_eq!(other.selected_index(), 0);
}

#[test]
fn selection_changed_reports_actual_items_once_and_containers_are_already_checked() {
    let _app = start();
    let group = fixed_group(2);
    layout(&group, 300.0);
    let changes = Rc::new(RefCell::new(Vec::new()));
    let observed = changes.clone();
    let weak = group.downgrade();
    group.selection_changed(move |_, args| {
        let group = weak.upgrade().unwrap();
        let selected = group.selected_index();
        if selected >= 0 { assert_eq!(radio(&group, selected).is_checked(), Some(true)); }
        observed.borrow_mut().push((args.removed_items().len(), args.added_items().len(), selected));
    });
    group.set_selected_index(0);
    group.set_selected_index(0);
    radio(&group, 1).on_click();
    group.set_selected_index(-1);
    assert_eq!(&*changes.borrow(), &[(0, 1, 0), (1, 1, 1), (1, 0, -1)]);
}

#[test]
fn column_major_layout_balances_columns_and_shrinks_to_available_width() {
    let _app = start();
    let group = fixed_group(5);
    group.set_max_columns(3);
    layout(&group, 200.0);
    assert_eq!(radio(&group, 0).bounds(), Rect::new(0.0, 0.0, 40.0, 20.0));
    assert_eq!(radio(&group, 1).bounds(), Rect::new(0.0, 28.0, 40.0, 20.0));
    assert_eq!(radio(&group, 2).bounds(), Rect::new(47.0, 0.0, 40.0, 20.0));
    assert_eq!(radio(&group, 3).bounds(), Rect::new(47.0, 28.0, 40.0, 20.0));
    assert_eq!(radio(&group, 4).bounds(), Rect::new(94.0, 0.0, 40.0, 20.0));
    layout(&group, 70.0);
    assert_eq!(radio(&group, 4).bounds().x, 0.0);
    assert_eq!(radio(&group, 4).bounds().y, 112.0);
    group.set_max_columns(1);
    layout(&group, 200.0);
    assert_eq!(radio(&group, 4).bounds().x, 0.0);
}

#[test]
fn keyboard_selection_skips_disabled_items_does_not_wrap_and_respects_control_modifier() {
    let _app = start();
    let group = fixed_group(4);
    let window = show(&group);
    let first = radio(&group, 0);
    let last = radio(&group, 3);
    group.set_selected_index(0);
    assert!(first.focus());
    radio(&group, 1).set_is_enabled(false);
    assert!(press(&first, Key::Down, KeyModifiers::NONE).handled());
    assert_eq!(group.selected_index(), 2);
    assert!(radio(&group, 2).is_focused());
    press(&radio(&group, 2), Key::Down, KeyModifiers::CONTROL);
    assert!(last.is_focused());
    assert_eq!(group.selected_index(), 2);
    assert!(press(&last, Key::Down, KeyModifiers::NONE).handled());
    assert!(last.is_focused());
    assert_eq!(group.selected_index(), 2);
    press(&last, Key::Up, KeyModifiers::NONE);
    assert_eq!(group.selected_index(), 2);
    press(&radio(&group, 2), Key::Up, KeyModifiers::NONE);
    assert_eq!(group.selected_index(), 0);
    window.close();
}

#[test]
fn keyboard_entry_restores_selected_container_and_horizontal_navigation_stays_in_group() {
    let _app = start();
    let group = fixed_group(5);
    group.set_max_columns(3);
    group.set_selected_index(3);
    let window = show(&group);
    layout(&group, 200.0);
    let first = radio(&group, 0);
    first.focus_with(NavigationMethod::Tab, KeyModifiers::NONE);
    assert!(radio(&group, 3).is_focused());
    press(&radio(&group, 3), Key::Right, KeyModifiers::NONE);
    assert!(radio(&group, 4).is_focused());
    assert_eq!(group.selected_index(), 4);
    press(&radio(&group, 4), Key::Right, KeyModifiers::NONE);
    assert!(radio(&group, 4).is_focused());
    window.close();
}

#[test]
fn collection_edits_keep_item_identity_and_update_accessibility_positions() {
    let _app = start();
    let group = fixed_group(3);
    layout(&group, 300.0);
    group.set_selected_index(1);
    let selected = radio(&group, 1);
    group.items().insert(0, Some(Control::boxed(RadioButton::new())));
    layout(&group, 300.0);
    assert_eq!(group.selected_index(), 2);
    assert_eq!(radio(&group, 2), selected);
    assert_eq!(selected.is_checked(), Some(true));
    for index in 0..4 {
        assert_eq!(AutomationProperties::get_position_in_set(&radio(&group, index)), index + 1);
        assert_eq!(AutomationProperties::get_size_of_set(&radio(&group, index)), 4);
    }
    group.items().remove_at(2);
    layout(&group, 300.0);
    assert_eq!(group.selected_index(), -1);
    assert_eq!(group.item_count(), 3);
}

#[test]
fn item_source_item_template_and_header_template_are_native_live_properties() {
    let _app = start();
    let group = RadioButtons::new();
    group.set_items_source(Some(ItemsSource::from_strs(["One", "Two"])));
    let template = FuncDataTemplate::for_type::<String>(|value, _| {
        let text = TextBlock::new();
        text.set_text(Some(&format!("Option {value}")));
        Some(text.upcast())
    }, false);
    group.set_item_template(Some(template.clone()));
    group.set_header(Some(Rc::new("Choices".to_string()) as BoxedValue));
    group.set_header_template(Some(template));
    layout(&group, 300.0);
    assert!(radio(&group, 0).content_template().is_some());
    let header = group.header_presenter().unwrap();
    assert_eq!(header.content().unwrap().downcast_ref::<String>().unwrap(), "Choices");
    assert!(header.content_template().is_some());
    group.set_selected_item(group.items_view().get_at(1));
    assert_eq!(group.selected_index(), 1);
    group.set_header(Some(Rc::new("Changed".to_string()) as BoxedValue));
    assert_eq!(header.content().unwrap().downcast_ref::<String>().unwrap(), "Changed");
    group.set_items_source(Some(ItemsSource::from_strs(["Replacement"])));
    layout(&group, 300.0);
    assert_eq!(group.item_count(), 1);
    assert_eq!(group.selected_index(), -1);
}

#[test]
fn group_exposes_native_selection_automation_and_preserves_other_control_defaults() {
    let _app = start();
    let group = fixed_group(2);
    let window = show(&group);
    group.set_selected_index(1);
    let peer = ControlAutomationPeer::create_peer_for_element(&group);
    assert_eq!(peer.get_class_name(), "RadioButtons");
    assert_eq!(peer.get_automation_control_type(), AutomationControlType::Group);
    let provider = peer.get_provider::<dyn ferroui_controls::automation::provider::ISelectionProvider>().unwrap();
    assert!(!provider.can_select_multiple());
    assert_eq!(provider.get_selection().len(), 1);
    assert_eq!(SelectingItemsControl::new().selected_index(), -1);
    assert_eq!(RadioButton::new().is_checked(), Some(false));
    window.close();
}
