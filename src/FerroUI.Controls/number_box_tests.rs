//! Tests of `NumberBox` (WinUI; docs/porting/winui-controls.md, Input), adapted from the tests of
//! #286 (`tests/winui_number_box.rs`) to the behaviour of WinUI's `NumberBox.cpp`.

use crate::automation::peers::{AutomationControlType, ControlAutomationPeer, NumberBoxAutomationPeer};
use crate::automation::provider::ProviderAdapter;
use crate::automation::{AutomationProperties, RangeValuePatternIdentifiers};
use crate::input_test_support::{number_box_template, part, start_with_focus};
use crate::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use crate::{
    Button, Control, INumberBoxNumberFormatter, NumberBox, NumberBoxSpinButtonPlacementMode, NumberBoxValidationMode,
    NumericUpDown, RepeatButton, TextBox, Window,
};
use ferroui_base::input::{InputElement, Key, KeyEventArgs};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::layout::ILayoutManager;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn start() -> UnitTestApplicationScope {
    crate::register_types();
    UnitTestApplication::start(TestServices::mock_threading_interface())
}

fn templated() -> ferroui_base::Ref<NumberBox> {
    let control = NumberBox::new();
    control.set_template(Some(number_box_template()));
    control
}

#[test]
fn the_value_is_a_double_that_is_nan_while_empty() {
    let _app = start();
    let control = NumberBox::new();
    assert!(control.value().is_nan());
    assert_eq!("", control.text());
    assert_eq!(-f64::MAX, control.minimum());
    assert_eq!(f64::MAX, control.maximum());
    assert_eq!(1.0, control.small_change());
    assert_eq!(10.0, control.large_change());
    assert_eq!(NumberBoxSpinButtonPlacementMode::Hidden, control.spin_button_placement_mode());
    assert_eq!(NumberBoxValidationMode::InvalidInputOverwritten, control.validation_mode());
    assert!(!control.accepts_expression());
    assert!(!control.is_wrap_enabled());
    control.set_numeric_value(1.0e100);
    assert_eq!(1.0e100, control.value());
    assert_eq!(1.0e100, control.text().parse::<f64>().unwrap());
    assert!(!std::ptr::eq(NumberBox::TYPE, NumericUpDown::TYPE));
}

#[test]
fn nan_and_equal_values_raise_no_change() {
    let _app = start();
    let control = NumberBox::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let recorded = changes.clone();
    control.value_changed(move |e| recorded.borrow_mut().push((e.old_value, e.new_value)));
    control.set_numeric_value(f64::NAN);
    control.set_numeric_value(5.0);
    control.set_numeric_value(5.0);
    control.set_text("");
    let changes = changes.borrow();
    assert_eq!(2, changes.len());
    assert!(changes[0].0.is_nan());
    assert_eq!(5.0, changes[0].1);
    assert_eq!(5.0, changes[1].0);
    assert!(changes[1].1.is_nan());
    assert_eq!("", control.text());
}

#[test]
fn the_bounds_coerce_the_value_and_each_other() {
    let _app = start();
    let control = NumberBox::new();
    control.set_minimum(0.0);
    control.set_maximum(10.0);
    let changes = Rc::new(Cell::new(0));
    let observed = changes.clone();
    control.value_changed(move |_| observed.set(observed.get() + 1));
    control.set_numeric_value(50.0);
    assert_eq!(10.0, control.value());
    assert_eq!(1, changes.get());
    // CoerceMaximum, then CoerceValue.
    control.set_minimum(20.0);
    assert_eq!(20.0, control.maximum());
    assert_eq!(20.0, control.value());
    // CoerceMinimum, then CoerceValue.
    control.set_maximum(-5.0);
    assert_eq!(-5.0, control.minimum());
    assert_eq!(-5.0, control.value());
}

#[test]
fn disabled_validation_keeps_invalid_text_and_values_out_of_the_range() {
    let _app = start();
    let control = NumberBox::new();
    control.set_maximum(10.0);
    control.set_numeric_value(5.0);
    control.set_text("invalid");
    assert_eq!("5", control.text());
    control.set_validation_mode(NumberBoxValidationMode::Disabled);
    control.set_text("invalid");
    assert_eq!("invalid", control.text());
    assert_eq!(5.0, control.value());
    assert!(!control.validate_input());
    control.set_numeric_value(25.0);
    assert_eq!(25.0, control.value());
    // As in WinUI, a change of the mode validates the text but does not coerce the value; the next
    // change of the range does.
    control.set_validation_mode(NumberBoxValidationMode::InvalidInputOverwritten);
    assert_eq!(25.0, control.value());
    control.set_maximum(12.0);
    assert_eq!(12.0, control.value());
}

#[test]
fn expressions_follow_the_precedence_of_number_box_and_a_division_by_zero_is_nan() {
    let _app = start();
    let control = NumberBox::new();
    control.set_text("2 + 3");
    assert!(control.value().is_nan(), "not a number without AcceptsExpression");
    control.set_accepts_expression(true);
    control.set_text("2 + 3 * (4 - 1)");
    assert_eq!(11.0, control.value());
    assert_eq!("11", control.text());
    control.set_text("2 ^ 3 ^ 2");
    assert_eq!(64.0, control.value());
    control.set_text("1 / 0");
    assert!(control.value().is_nan());
    assert_eq!("", control.text());
}

#[test]
fn a_step_commits_the_text_wraps_and_leaves_an_empty_box_empty() {
    let _app = start();
    let control = NumberBox::new();
    control.set_minimum(0.0);
    control.set_maximum(10.0);
    control.step_value(1.0);
    assert!(control.value().is_nan());
    control.set_numeric_value(9.0);
    control.step_value(3.0);
    assert_eq!(10.0, control.value());
    control.set_is_wrap_enabled(true);
    control.step_value(1.0);
    assert_eq!(0.0, control.value());
    control.step_value(-1.0);
    assert_eq!(10.0, control.value());
}

struct CommaFormatter;

impl INumberBoxNumberFormatter for CommaFormatter {
    fn format_double(&self, value: f64) -> String {
        value.to_string().replace('.', ",")
    }

    fn parse_double(&self, text: &str) -> Option<f64> {
        text.replace(',', ".").parse().ok()
    }
}

#[test]
fn the_number_formatter_formats_the_value_and_reads_the_numbers_of_expressions() {
    let _app = start();
    let control = NumberBox::new();
    control.set_number_formatter(Rc::new(CommaFormatter));
    control.set_accepts_expression(true);
    control.set_text("1,5 + 2,25");
    assert_eq!(3.75, control.value());
    assert_eq!("3,75", control.text());
}

#[test]
fn a_handler_can_change_the_value_and_disposing_it_removes_it() {
    let _app = start();
    let control = NumberBox::new();
    let weak = control.downgrade();
    let subscription = control.value_changed(move |e| {
        if e.new_value == 5.0 {
            weak.upgrade().unwrap().set_numeric_value(6.0);
        }
    });
    control.set_numeric_value(5.0);
    assert_eq!(6.0, control.value());
    assert_eq!("6", control.text());
    subscription.dispose();
    control.set_numeric_value(5.0);
    assert_eq!(5.0, control.value());
}

#[test]
fn the_text_box_is_committed_by_the_spin_buttons_and_the_states_follow_the_value() {
    let _app = start();
    let control = templated();
    control.set_numeric_value(3.0);
    control.set_maximum(8.0);
    control.set_spin_button_placement_mode(NumberBoxSpinButtonPlacementMode::Inline);
    control.apply_template();
    assert!(control.classes().contains(":spinbuttonsvisible"));
    assert!(!control.classes().contains(":spinbuttonspopup"));
    let text_box = part::<TextBox>(&control, "InputBox");
    assert_eq!(Some("3"), text_box.text().as_deref());
    text_box.set_text(Some("7"));
    // The text of the box is input until it is committed: Text is the text of the value.
    assert_eq!("3", control.text());
    assert_eq!(3.0, control.value());
    let up = part::<RepeatButton>(&control, "UpSpinButton");
    assert_eq!(Some("Increase"), AutomationProperties::get_name(&up).as_deref());
    up.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert_eq!(8.0, control.value());
    assert_eq!(Some("8"), text_box.text().as_deref());
    assert_eq!("8", control.text());
    // UpSpinButtonDisabled at the maximum.
    assert!(control.classes().contains(":upspinbuttondisabled"));
    assert!(!control.classes().contains(":downspinbuttondisabled"));
    control.set_spin_button_placement_mode(NumberBoxSpinButtonPlacementMode::Compact);
    assert!(!control.classes().contains(":spinbuttonsvisible"));
    assert!(control.classes().contains(":spinbuttonspopup"));
    control.set_numeric_value(f64::NAN);
    assert!(control.classes().contains(":upspinbuttondisabled"));
    assert!(control.classes().contains(":downspinbuttondisabled"));
}

#[test]
fn the_parts_of_a_replaced_template_no_longer_change_the_box() {
    let _app = start();
    let control = templated();
    control.set_numeric_value(3.0);
    control.apply_template();
    let old_up = part::<RepeatButton>(&control, "UpSpinButton");
    control.set_template(Some(number_box_template()));
    control.apply_template();
    old_up.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert_eq!(3.0, control.value());
    let up = part::<RepeatButton>(&control, "UpSpinButton");
    up.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert_eq!(4.0, control.value());
}

#[test]
fn text_set_before_the_template_is_the_value_when_the_value_is_not_set() {
    let _app = start();
    let control = templated();
    control.set_text("12");
    control.apply_template();
    assert_eq!(12.0, control.value());
    assert_eq!(Some("12"), part::<TextBox>(&control, "InputBox").text().as_deref());
}

#[test]
fn the_header_presenter_is_shown_for_a_text_header_or_a_header_template() {
    let _app = start();
    let control = templated();
    control.apply_template();
    let header = part::<crate::presenters::ContentPresenter>(&control, "HeaderContentPresenter");
    assert!(!header.is_visible());
    control.set_header(Some(Rc::new(String::new())));
    assert!(!header.is_visible());
    control.set_header(Some(Rc::new("Quantity".to_string())));
    assert!(header.is_visible());
    // The text box is named by the header, and by the range.
    let text_box = part::<TextBox>(&control, "InputBox");
    assert_eq!(Some("Quantity"), AutomationProperties::get_name(&text_box).as_deref());
    control.set_minimum(0.0);
    assert_eq!(Some("Quantity Minimum0"), AutomationProperties::get_name(&text_box).as_deref());
    control.set_header(None);
    assert!(!header.is_visible());
}

#[test]
fn the_value_is_rounded_to_ten_digits_for_display_only() {
    let _app = start();
    let control = NumberBox::new();
    let precise = 0.1_f64 + 0.2_f64;
    control.set_numeric_value(precise);
    assert_eq!(precise.to_bits(), control.value().to_bits());
    assert_eq!("0.3", control.text());
    // RoundHalfUp: a tie goes toward positive infinity.
    control.set_numeric_value(12_345_678_905.0);
    assert_eq!("12345678910", control.text());
    control.set_numeric_value(-12_345_678_905.0);
    assert_eq!("-12345678900", control.text());
    for value in [1.0e100, 1.0e-310, f64::from_bits(1)] {
        control.set_numeric_value(value);
        assert_eq!(value.to_bits(), control.value().to_bits());
        assert_eq!(value.to_bits(), control.text().parse::<f64>().unwrap().to_bits());
    }
}

#[test]
fn the_keys_step_the_value_while_the_text_box_has_the_focus() {
    let _app = start_with_focus();
    let control = templated();
    control.set_numeric_value(5.0);
    let window = Window::new();
    window.set_content(Some(Control::boxed(control.clone())));
    window.show();
    window.layout_manager().execute_layout_pass();
    let text_box = part::<TextBox>(&control, "InputBox");
    assert!(text_box.focus());
    let press = |key| {
        let mut args = KeyEventArgs::new();
        args.key = key;
        args.set_routed_event(Some(InputElement::key_down_event()));
        text_box.raise_event(&args);
        args
    };
    assert!(press(Key::Up).handled());
    assert_eq!(6.0, control.value());
    press(Key::PageUp);
    assert_eq!(16.0, control.value());
    press(Key::PageDown);
    press(Key::Down);
    assert_eq!(5.0, control.value());
    assert!(!press(Key::A).handled());
    window.close();
}

#[test]
fn the_peer_is_a_writable_spinner_with_the_steps_of_the_box() {
    let _app = start();
    let control = NumberBox::new();
    control.set_numeric_value(2.5);
    control.set_small_change(0.5);
    control.set_large_change(20.0);
    let provider = ProviderAdapter::as_range_value_provider(NumberBoxAutomationPeer::new(&control));
    assert_eq!(2.5, provider.value());
    assert_eq!(0.5, provider.small_change());
    assert_eq!(20.0, provider.large_change());
    provider.set_value(4.5).unwrap();
    assert_eq!(4.5, control.value());
    control.set_is_enabled(false);
    assert!(!provider.is_read_only());
    provider.set_value(7.0).unwrap();
    assert_eq!(7.0, control.value());
}

#[test]
fn the_peer_reports_the_coerced_value_after_the_handlers_and_the_changes_of_the_steps() {
    let _app = start();
    let control = NumberBox::new();
    control.set_header(Some(Rc::new(String::from("Quantity"))));
    control.set_maximum(10.0);
    let peer = ControlAutomationPeer::create_peer_for_element(&control);
    assert_eq!("Quantity", peer.get_name());
    assert_eq!(AutomationControlType::Spinner, peer.get_automation_control_type());
    assert_eq!("NumberBox", peer.get_class_name());
    AutomationProperties::set_name(&control, Some("Explicit label"));
    assert_eq!("Explicit label", peer.get_name());
    let events = Rc::new(RefCell::new(Vec::new()));
    let recorded = events.clone();
    control.value_changed(move |_| recorded.borrow_mut().push("application".to_string()));
    let recorded = events.clone();
    peer.property_changed(move |event| {
        if std::ptr::eq(event.property(), RangeValuePatternIdentifiers::value_property()) {
            assert_eq!(Some(&10.0), event.new_value().unwrap().downcast_ref::<f64>());
            recorded.borrow_mut().push("automation".to_string());
        } else if std::ptr::eq(event.property(), RangeValuePatternIdentifiers::small_change_property()) {
            assert_eq!(Some(&0.25), event.new_value().unwrap().downcast_ref::<f64>());
            recorded.borrow_mut().push("small change".to_string());
        } else if std::ptr::eq(event.property(), RangeValuePatternIdentifiers::large_change_property()) {
            assert_eq!(Some(&5.0), event.new_value().unwrap().downcast_ref::<f64>());
            recorded.borrow_mut().push("large change".to_string());
        }
    });
    control.set_numeric_value(50.0);
    control.set_small_change(0.25);
    control.set_large_change(5.0);
    assert_eq!(&["application", "automation", "small change", "large change"], &events.borrow()[..]);
}
