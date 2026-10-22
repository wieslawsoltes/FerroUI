use ferroui_controls::automation::peers::{NumberBoxAutomationPeer};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_controls::automation::provider::ProviderAdapter;
use ferroui_controls::templates::FuncControlTemplate;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{
    evaluate_number_expression, INumberBoxNumberFormatter, NumberBox, NumberBoxSpinButtonPlacementMode, NumberBoxValidationMode,
};
use ferroui_controls::{Button, RepeatButton, TextBox};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn start() -> ferroui_controls::testing::UnitTestApplicationScope {
    ferroui_controls::register_types();
    UnitTestApplication::start(TestServices::mock_threading_interface())
}

#[test]
fn number_box_has_double_domain_and_distinct_default_type() {
    let _app = start();
    let control = NumberBox::new();
    assert!(control.value().is_nan());
    assert_eq!(control.text(), "");
    assert_eq!(control.minimum(), -f64::MAX);
    assert_eq!(control.maximum(), f64::MAX);
    assert_eq!(control.small_change(), 1.0);
    assert_eq!(control.large_change(), 10.0);
    assert_eq!(
        control.spin_button_placement_mode(),
        NumberBoxSpinButtonPlacementMode::Hidden
    );
    control.set_numeric_value(1.0e100);
    assert_eq!(control.value(), 1.0e100);
    assert_eq!(control.text().parse::<f64>().unwrap(), 1.0e100);
    assert_ne!(NumberBox::TYPE, ferroui_controls::NumericUpDown::TYPE);
}

#[test]
fn empty_nan_and_equal_values_do_not_emit_duplicate_notifications() {
    let _app = start();
    let control = NumberBox::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let recorded = changes.clone();
    control.value_changed(move |e| recorded.borrow_mut().push((e.old_value, e.new_value)));
    control.set_numeric_value(f64::NAN);
    control.set_numeric_value(5.0);
    control.set_numeric_value(5.0);
    control.set_text("");
    assert_eq!(changes.borrow().len(), 2);
    assert!(changes.borrow()[0].0.is_nan());
    assert_eq!(changes.borrow()[0].1, 5.0);
    assert!(changes.borrow()[1].1.is_nan());
    assert_eq!(control.text(), "");
}

#[test]
fn bounds_coerce_value_once_and_follow_last_changed_bound() {
    let _app = start();
    let control = NumberBox::new();
    control.set_minimum(0.0);
    control.set_maximum(10.0);
    let changes = Rc::new(Cell::new(0));
    let observed = changes.clone();
    control.value_changed(move |_| observed.set(observed.get() + 1));
    control.set_numeric_value(50.0);
    assert_eq!(control.value(), 10.0);
    assert_eq!(changes.get(), 1);
    control.set_minimum(20.0);
    assert_eq!(control.maximum(), 20.0);
    assert_eq!(control.value(), 20.0);
    control.set_maximum(-5.0);
    assert_eq!(control.minimum(), -5.0);
    assert_eq!(control.value(), -5.0);
}

#[test]
fn disabled_validation_preserves_invalid_text_and_out_of_range_values() {
    let _app = start();
    let control = NumberBox::new();
    control.set_maximum(10.0);
    control.set_numeric_value(5.0);
    control.set_text("invalid");
    assert_eq!(control.text(), "5");
    control.set_validation_mode(NumberBoxValidationMode::Disabled);
    control.set_text("invalid");
    assert_eq!(control.text(), "invalid");
    assert_eq!(control.value(), 5.0);
    assert!(!control.commit_input());
    control.set_numeric_value(25.0);
    assert_eq!(control.value(), 25.0);
    control.set_validation_mode(NumberBoxValidationMode::InvalidInputOverwritten);
    assert_eq!(control.value(), 10.0);
}

#[test]
fn expression_input_uses_upstream_precedence_and_nan_division() {
    let _app = start();
    let control = NumberBox::new();
    control.set_accepts_expression(true);
    control.set_text("2 + 3 * (4 - 1)");
    assert_eq!(control.value(), 11.0);
    assert_eq!(control.text(), "11");
    control.set_text("2 ^ 3 ^ 2");
    assert_eq!(control.value(), 64.0); // WinUI is deliberately left associative.
    control.set_text("1 / 0");
    assert!(control.value().is_nan());
    assert!(control.text().is_empty());
}

#[test]
fn expression_parser_rejects_malformed_and_bounded_inputs_without_recursion() {
    for input in [
        "", "(1", "1)", "1 +", "()", "-(2)", "1 2", "1 + 🍎", "1\t+2",
    ] {
        assert!(
            evaluate_number_expression(input, |s| s.parse().ok()).is_err(),
            "{input}"
        );
    }
    assert_eq!(
        evaluate_number_expression("-2 * -3", |s| s.parse().ok()).unwrap(),
        6.0
    );
    assert!(evaluate_number_expression(&"(".repeat(65_537), |s| s.parse().ok()).is_err());
    assert!(evaluate_number_expression(
        &format!("{}1{}", "(".repeat(4096), ")".repeat(4096)),
        |s| s.parse().ok()
    )
    .is_err());
}

#[test]
fn stepping_commits_drafts_wraps_and_preserves_empty_value() {
    let _app = start();
    let control = NumberBox::new();
    control.set_minimum(0.0);
    control.set_maximum(10.0);
    control.step_value(1.0);
    assert!(control.value().is_nan());
    control.set_numeric_value(9.0);
    control.step_value(3.0);
    assert_eq!(control.value(), 10.0);
    control.set_is_wrap_enabled(true);
    control.step_value(1.0);
    assert_eq!(control.value(), 0.0);
    control.step_value(-1.0);
    assert_eq!(control.value(), 10.0);
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
fn custom_number_formatter_is_shared_by_text_and_expression_parser() {
    let _app = start();
    let control = NumberBox::new();
    control.set_number_formatter(Rc::new(CommaFormatter));
    control.set_accepts_expression(true);
    control.set_text("1,5 + 2,25");
    assert_eq!(control.value(), 3.75);
    assert_eq!(control.text(), "3,75");
}

#[test]
fn handler_can_redirect_value_and_subscription_disposal_is_effective() {
    let _app = start();
    let control = NumberBox::new();
    let weak = control.downgrade();
    let subscription = control.value_changed(move |e| {
        if e.new_value == 5.0 {
            weak.upgrade().unwrap().set_numeric_value(6.0);
        }
    });
    control.set_numeric_value(5.0);
    assert_eq!(control.value(), 6.0);
    assert_eq!(control.text(), "6");
    subscription.dispose();
    control.set_numeric_value(5.0);
    assert_eq!(control.value(), 5.0);
}

#[test]
fn native_text_and_repeat_button_parts_drive_the_numeric_model() {
    let _app = start();
    let control = NumberBox::new();
    control.set_numeric_value(3.0);
    control.set_spin_button_placement_mode(NumberBoxSpinButtonPlacementMode::Inline);
    control.apply_template();
    let descendants = control.get_template_descendants();
    let text = descendants
        .iter()
        .find_map(|c| c.cast::<TextBox>())
        .unwrap();
    text.set_text(Some("7"));
    assert_eq!(control.text(), "7");
    assert_eq!(control.value(), 3.0); // The edit is a draft until committed.
    let up = descendants
        .iter()
        .find(|c| c.name().as_deref() == Some("PART_SpinUp"))
        .unwrap()
        .cast::<RepeatButton>()
        .unwrap();
    up.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert_eq!(control.value(), 8.0);
    assert_eq!(text.text().as_deref(), Some("8"));
}

#[test]
fn detached_template_parts_cannot_change_the_owner() {
    let _app = start();
    let control = NumberBox::new();
    control.set_numeric_value(3.0);
    control.apply_template();
    let old = control
        .get_template_descendants()
        .iter()
        .find_map(|c| c.cast::<TextBox>())
        .unwrap();
    let template = control.template();
    control.set_template(Some(FuncControlTemplate::for_type::<NumberBox>(|_, _| {
        TextBox::new().upcast()
    })));
    control.apply_template();
    control.set_template(template);
    control.apply_template();
    old.set_text(Some("99"));
    assert_eq!(control.value(), 3.0);
    assert_eq!(control.text(), "3");
}

#[test]
fn display_rounding_preserves_full_value_and_rounds_half_toward_positive_infinity() {
    let _app = start();
    let control = NumberBox::new();
    let precise = 0.1_f64 + 0.2_f64;
    control.set_numeric_value(precise);
    assert_eq!(control.value().to_bits(), precise.to_bits());
    assert_eq!(control.text(), "0.3");
    control.set_numeric_value(12_345_678_905.0);
    assert_eq!(control.text(), "12345678910");
    control.set_numeric_value(-12_345_678_905.0);
    assert_eq!(control.text(), "-12345678900");
    for value in [1.0e100, 1.0e-310, f64::from_bits(1)] {
        control.set_numeric_value(value);
        assert_eq!(control.value().to_bits(), value.to_bits());
        assert_eq!(
            control.text().parse::<f64>().unwrap().to_bits(),
            value.to_bits()
        );
    }
}

#[test]
fn automation_exposes_double_range_steps_and_native_writable_contract() {
    let _app = start();
    let control = NumberBox::new();
    control.set_numeric_value(2.5);
    control.set_small_change(0.5);
    control.set_large_change(20.0);
    let provider = ProviderAdapter::as_range_value_provider(NumberBoxAutomationPeer::new(&control));
    assert_eq!(provider.value(), 2.5);
    assert_eq!(provider.small_change(), 0.5);
    assert_eq!(provider.large_change(), 20.0);
    provider.set_value(4.5).unwrap();
    assert_eq!(control.value(), 4.5);
    control.set_is_enabled(false);
    assert!(!provider.is_read_only());
    provider.set_value(7.0).unwrap();
    assert_eq!(control.value(), 7.0);
}

#[test]
fn automation_reports_one_coerced_value_after_application_handlers_and_uses_header_name() {
    use ferroui_controls::automation::{AutomationProperties, RangeValuePatternIdentifiers};
    let _app = start();
    let control = NumberBox::new();
    control.set_header(Some(Rc::new(String::from("Quantity"))));
    control.set_maximum(10.0);
    let peer = ferroui_controls::automation::peers::ControlAutomationPeer::create_peer_for_element(
        &control,
    );
    assert_eq!(peer.get_name(), "Quantity");
    AutomationProperties::set_name(&control, Some("Explicit label"));
    assert_eq!(peer.get_name(), "Explicit label");
    let events = Rc::new(RefCell::new(Vec::new()));
    let recorded = events.clone();
    control.value_changed(move |_| recorded.borrow_mut().push("application"));
    let recorded = events.clone();
    peer.property_changed(move |event| {
        if event.property() == RangeValuePatternIdentifiers::value_property() {
            assert_eq!(
                event.new_value().unwrap().downcast_ref::<f64>(),
                Some(&10.0)
            );
            recorded.borrow_mut().push("automation");
        }
    });
    control.set_numeric_value(50.0);
    assert_eq!(&*events.borrow(), &["application", "automation"]);
}
