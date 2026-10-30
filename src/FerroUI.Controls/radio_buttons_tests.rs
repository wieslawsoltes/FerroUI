//! Tests of `RadioButtons` and `RadioButtonsPanel` (WinUI; docs/porting/winui-controls.md, Input),
//! adapted from the tests of #286 (`tests/winui_radio_buttons.rs`).

use crate::automation::peers::{AutomationControlType, ControlAutomationPeer};
use crate::automation::provider::ISelectionProvider;
use crate::automation::AutomationProperties;
use crate::input_test_support::{radio_buttons_panel, radio_buttons_template, start_with_focus};
use crate::primitives::SelectingItemsControl;
use crate::templates::FuncDataTemplate;
use crate::{Control, ItemsSource, RadioButton, RadioButtons, TextBlock, Window};
use ferroui_base::input::{InputElement, Key, KeyEventArgs, KeyModifiers, NavigationMethod};
use ferroui_base::layout::ILayoutManager;
use ferroui_base::{BoxedValue, Rect, Ref, Size};
use std::cell::RefCell;
use std::rc::Rc;

fn group() -> Ref<RadioButtons> {
    crate::register_types();
    let group = RadioButtons::new();
    group.set_template(Some(radio_buttons_template()));
    group.set_items_panel(radio_buttons_panel());
    group
}

fn layout(group: &RadioButtons, width: f64) {
    group.measure(Size::new(width, 1000.0));
    group.arrange(Rect::new(0.0, 0.0, width, group.desired_size().height));
}

fn radio(group: &RadioButtons, index: i32) -> Ref<RadioButton> {
    group.container_from_index(index).unwrap().cast::<RadioButton>().unwrap()
}

fn fixed_group(count: usize) -> Ref<RadioButtons> {
    let group = group();
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
fn data_items_become_radio_buttons_and_the_selection_follows_the_checked_state() {
    let _app = start_with_focus();
    let group = group();
    group.items().add(Some(Rc::new("Red".to_string()) as BoxedValue));
    group.items().add(Some(Rc::new("Blue".to_string()) as BoxedValue));
    assert_eq!(-1, group.selected_index());
    layout(&group, 300.0);
    assert_eq!("Red", radio(&group, 0).content().unwrap().downcast_ref::<String>().unwrap());
    group.set_selected_index(0);
    assert_eq!(Some(true), radio(&group, 0).is_checked());
    assert_eq!(Some(false), radio(&group, 1).is_checked());
    radio(&group, 1).on_click();
    assert_eq!(1, group.selected_index());
    assert_eq!("Blue", group.selected_item().unwrap().downcast_ref::<String>().unwrap());
    // The radio button group manager unchecked the other container.
    assert_eq!(Some(false), radio(&group, 0).is_checked());
    radio(&group, 1).on_click();
    assert_eq!(1, group.selected_index());
    radio(&group, 1).set_is_checked(Some(false));
    assert_eq!(-1, group.selected_index());
    // A cleared selection unchecks its container.
    group.set_selected_index(0);
    group.set_selected_index(-1);
    assert_eq!(Some(false), radio(&group, 0).is_checked());
}

#[test]
fn a_checked_radio_button_item_is_the_selection_and_each_group_is_its_own() {
    let _app = start_with_focus();
    let first = fixed_group(3);
    let supplied = first.items_view().get_at(1).unwrap();
    let supplied = Control::from_boxed(&supplied).unwrap().cast::<RadioButton>().unwrap();
    supplied.set_is_checked(Some(true));
    layout(&first, 300.0);
    assert_eq!(supplied, radio(&first, 1));
    assert_eq!(1, first.selected_index());
    let other = fixed_group(2);
    layout(&other, 300.0);
    radio(&other, 0).on_click();
    assert_eq!(1, first.selected_index());
    assert_eq!(Some(true), radio(&first, 1).is_checked());
    assert_eq!(0, other.selected_index());
}

#[test]
fn selection_changed_reports_the_items_once_with_the_containers_already_checked() {
    let _app = start_with_focus();
    let group = fixed_group(2);
    layout(&group, 300.0);
    let changes = Rc::new(RefCell::new(Vec::new()));
    let observed = changes.clone();
    let weak = group.downgrade();
    group.selection_changed(move |_, args| {
        let group = weak.upgrade().unwrap();
        let selected = group.selected_index();
        if selected >= 0 {
            assert_eq!(Some(true), radio(&group, selected).is_checked());
        }
        observed.borrow_mut().push((args.removed_items().len(), args.added_items().len(), selected));
    });
    group.set_selected_index(0);
    group.set_selected_index(0);
    radio(&group, 1).on_click();
    group.set_selected_index(-1);
    assert_eq!(&[(0, 1, 0), (1, 1, 1), (1, 0, -1)], &changes.borrow()[..]);
}

#[test]
fn the_panel_fills_columns_of_the_largest_item_with_the_spacing_of_the_theme() {
    let _app = start_with_focus();
    let group = fixed_group(5);
    group.set_max_columns(3);
    layout(&group, 200.0);
    // RadioButtonsColumnSpacing 7 and RadioButtonsRowSpacing 8.
    assert_eq!(Rect::new(0.0, 0.0, 40.0, 20.0), radio(&group, 0).bounds());
    assert_eq!(Rect::new(0.0, 28.0, 40.0, 20.0), radio(&group, 1).bounds());
    assert_eq!(Rect::new(47.0, 0.0, 40.0, 20.0), radio(&group, 2).bounds());
    assert_eq!(Rect::new(47.0, 28.0, 40.0, 20.0), radio(&group, 3).bounds());
    assert_eq!(Rect::new(94.0, 0.0, 40.0, 20.0), radio(&group, 4).bounds());
    // Too narrow for a second column.
    layout(&group, 70.0);
    assert_eq!(0.0, radio(&group, 4).bounds().x);
    assert_eq!(112.0, radio(&group, 4).bounds().y);
    group.set_max_columns(1);
    layout(&group, 200.0);
    assert_eq!(0.0, radio(&group, 4).bounds().x);
}

#[test]
fn max_columns_less_than_one_is_one() {
    let _app = start_with_focus();
    let group = group();
    group.set_max_columns(0);
    assert_eq!(1, group.max_columns());
    group.set_max_columns(-3);
    assert_eq!(1, group.max_columns());
}

#[test]
fn the_arrow_keys_skip_disabled_buttons_stop_at_the_ends_and_control_moves_the_focus_only() {
    let _app = start_with_focus();
    let group = fixed_group(4);
    let window = show(&group);
    let first = radio(&group, 0);
    let last = radio(&group, 3);
    group.set_selected_index(0);
    assert!(first.focus());
    radio(&group, 1).set_is_enabled(false);
    assert!(press(&first, Key::Down, KeyModifiers::NONE).handled());
    assert_eq!(2, group.selected_index());
    assert!(radio(&group, 2).is_focused());
    press(&radio(&group, 2), Key::Down, KeyModifiers::CONTROL);
    assert!(last.is_focused());
    assert_eq!(2, group.selected_index());
    assert!(press(&last, Key::Down, KeyModifiers::NONE).handled());
    assert!(last.is_focused());
    assert_eq!(2, group.selected_index());
    press(&last, Key::Up, KeyModifiers::NONE);
    assert_eq!(2, group.selected_index());
    press(&radio(&group, 2), Key::Up, KeyModifiers::NONE);
    assert_eq!(0, group.selected_index());
    window.close();
}

#[test]
fn tab_enters_at_the_selected_button_and_the_horizontal_keys_stay_in_the_group() {
    let _app = start_with_focus();
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
    assert_eq!(4, group.selected_index());
    press(&radio(&group, 4), Key::Right, KeyModifiers::NONE);
    assert!(radio(&group, 4).is_focused());
    window.close();
}

#[test]
fn collection_edits_keep_the_items_and_update_the_positions_for_automation() {
    let _app = start_with_focus();
    let group = fixed_group(3);
    layout(&group, 300.0);
    group.set_selected_index(1);
    let selected = radio(&group, 1);
    group.items().insert(0, Some(Control::boxed(RadioButton::new())));
    layout(&group, 300.0);
    assert_eq!(2, group.selected_index());
    assert_eq!(selected, radio(&group, 2));
    assert_eq!(Some(true), selected.is_checked());
    for index in 0..4 {
        assert_eq!(index + 1, AutomationProperties::get_position_in_set(&radio(&group, index)));
        assert_eq!(4, AutomationProperties::get_size_of_set(&radio(&group, index)));
    }
    group.items().remove_at(2);
    layout(&group, 300.0);
    assert_eq!(-1, group.selected_index());
    assert_eq!(3, group.item_count());
}

#[test]
fn the_items_source_item_template_and_header_template_are_live() {
    let _app = start_with_focus();
    let group = group();
    group.set_items_source(Some(ItemsSource::from_strs(["One", "Two"])));
    let template = FuncDataTemplate::for_type::<String>(
        |value, _| {
            let text = TextBlock::new();
            text.set_text(Some(&format!("Option {value}")));
            Some(text.upcast())
        },
        false,
    );
    group.set_item_template(Some(template.clone()));
    group.set_header(Some(Rc::new("Choices".to_string()) as BoxedValue));
    group.set_header_template(Some(template));
    layout(&group, 300.0);
    assert!(radio(&group, 0).content_template().is_some());
    let header = group.header_presenter().unwrap();
    assert_eq!("Choices", header.content().unwrap().downcast_ref::<String>().unwrap());
    assert!(header.content_template().is_some());
    group.set_selected_item(group.items_view().get_at(1));
    assert_eq!(1, group.selected_index());
    group.set_header(Some(Rc::new("Changed".to_string()) as BoxedValue));
    assert_eq!("Changed", header.content().unwrap().downcast_ref::<String>().unwrap());
    group.set_items_source(Some(ItemsSource::from_strs(["Replacement"])));
    layout(&group, 300.0);
    assert_eq!(1, group.item_count());
    assert_eq!(-1, group.selected_index());
}

#[test]
fn the_peer_is_a_group_named_by_its_header_with_the_selection_of_the_items() {
    let _app = start_with_focus();
    let group = fixed_group(2);
    group.set_header(Some(Rc::new("Colours".to_string()) as BoxedValue));
    let window = show(&group);
    group.set_selected_index(1);
    let peer = ControlAutomationPeer::create_peer_for_element(&group);
    assert_eq!("RadioButtons", peer.get_class_name());
    assert_eq!(AutomationControlType::Group, peer.get_automation_control_type());
    assert_eq!("Colours", peer.get_name());
    let provider = peer.get_provider::<dyn ISelectionProvider>().unwrap();
    assert!(!provider.can_select_multiple());
    assert_eq!(1, provider.get_selection().len());
    // The defaults of the upstream controls do not change.
    assert_eq!(-1, SelectingItemsControl::new().selected_index());
    assert_eq!(Some(false), RadioButton::new().is_checked());
    window.close();
}
