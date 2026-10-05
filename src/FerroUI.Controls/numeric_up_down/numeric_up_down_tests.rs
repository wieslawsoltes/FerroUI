//! Port of the reference `NumericUpDownTests`.

use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
use crate::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use crate::{
    ButtonSpinner, Control, DataValidationErrors, NumericUpDown, SpinDirection, SpinEventArgs, Spinner, TextBox,
    ValidSpinDirections, Window,
};
use ferroui_base::data::converters::IValueConverter;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::data::{BindingError, BindingErrorType, BindingNotification, BindingPriority};
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::media::{Brushes, IBrush};
use ferroui_base::platform::IPlatformRenderInterface;
use ferroui_base::reactive::Observable;
use ferroui_base::threading::Dispatcher;
use ferroui_base::input::{FocusChangedEventArgs, InputElement, Key, KeyEventArgs};
use ferroui_base::utilities::{Decimal, NumberFormatInfo, NumberStyles};
use ferroui_base::{AnyValue, BoxedValue, FerroLocator, LocatorExtensions, Ref};
use std::rc::Rc;

/// A running unit test application, with the text services of the tests
/// registered over the services of the application.
struct AppScope {
    // Dropped in this order: the application first.
    _app: UnitTestApplicationScope,
    _text: TextTestScope,
}

fn start() -> AppScope {
    let text = TextTestScope::new();
    let render_interface = FerroLocator::current()
        .get_service::<dyn IPlatformRenderInterface>()
        .expect("the text services have a render interface");
    let app = UnitTestApplication::start(TestServices::styled_window().with_render_interface(render_interface));
    AppScope { _app: app, _text: text }
}

fn d(text: &str) -> Decimal {
    Decimal::parse(text).unwrap()
}

/// Asserts that the data validation errors of the control are exactly
/// `exception`.
fn assert_errors_are(target: &Control, exception: &BindingError) {
    let errors = DataValidationErrors::get_errors(target).map(|errors| errors.to_vec()).unwrap_or_default();
    assert_eq!(1, errors.len());
    let error = errors[0].clone();
    let error: &dyn AnyValue = &*error;
    assert!(error.downcast_ref::<BindingError>() == Some(exception));
}

#[test]
fn text_validation() {
    run_test(|control, _textbox| {
        let exception = BindingError::message("failed validation");
        let text_observable = Observable::single_value(Rc::new(BindingNotification::with_error(
            exception.clone(),
            BindingErrorType::DataValidationError,
        )) as BoxedValue);
        control.bind_property_untyped(
            NumericUpDown::text_property().as_property(),
            text_observable,
            BindingPriority::LocalValue,
        );
        Dispatcher::ui_thread().run_jobs(None);

        assert!(DataValidationErrors::get_has_errors(control));
        assert_errors_are(control, &exception);
    });
}

#[test]
fn value_validation() {
    run_test(|control, _textbox| {
        let exception = BindingError::message("failed validation");
        let value_observable = Observable::single_value(Rc::new(BindingNotification::with_error(
            exception.clone(),
            BindingErrorType::DataValidationError,
        )) as BoxedValue);
        control.bind_property_untyped(
            NumericUpDown::value_property().as_property(),
            value_observable,
            BindingPriority::LocalValue,
        );
        Dispatcher::ui_thread().run_jobs(None);

        assert!(DataValidationErrors::get_has_errors(control));
        assert_errors_are(control, &exception);
    });
}

#[test]
fn increment_decrement_tests() {
    for (min, max, value, direction, expected) in increment_decrement_test_data() {
        let control = create_control();
        if min > Decimal::MIN_VALUE {
            control.set_minimum(min);
        }
        if max < Decimal::MAX_VALUE {
            control.set_maximum(max);
        }
        control.set_numeric_value(value);

        let spinner = get_spinner(&control);

        spinner.raise_event(&SpinEventArgs::with_event(Some(Spinner::spin_event()), direction));

        assert_eq!(control.value(), expected, "{min} {max} {value:?} {direction:?}");
    }
}

#[test]
fn format_string_is_applied_immediately() {
    run_test(|control, _textbox| {
        let value = d("10.11");
        let current = NumberFormatInfo::current_info();

        // Establish and verify initial conditions.
        control.set_format_string("F0");
        control.set_numeric_value(Some(value));
        assert_eq!(value.to_string_with("F0", Some(&current)).ok(), control.text());
        assert_eq!(Some("10"), control.text().as_deref());

        // Check that the format string is applied.
        control.set_format_string("F2");
        assert_eq!(value.to_string_with("F2", Some(&current)).ok(), control.text());
        assert_eq!(Some("10.11"), control.text().as_deref());
    });
}

#[test]
fn number_format_is_applied_immediately() {
    run_test(|control, _textbox| {
        let value = d("10.11");
        let initial_number_format = Rc::new(NumberFormatInfo::new().with_number_decimal_separator("."));
        let new_number_format = Rc::new(NumberFormatInfo::new().with_number_decimal_separator(";"));

        // Establish and verify initial conditions.
        control.set_number_format(Some(initial_number_format.clone()));
        control.set_numeric_value(Some(value));
        assert_eq!(Some(value.to_string_provider(&initial_number_format)), control.text());
        assert_eq!(Some("10.11"), control.text().as_deref());

        // Check that the number format is applied.
        control.set_number_format(Some(new_number_format.clone()));
        assert_eq!(Some(value.to_string_provider(&new_number_format)), control.text());
        assert_eq!(Some("10;11"), control.text().as_deref());
    });
}

struct TestNumericUpDownValueConverter {
    format: &'static str,
}

impl IValueConverter for TestNumericUpDownValueConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, BindingError> {
        let input = value.map(|value| ValueTypes::to_display_string(Some(value))).unwrap_or_default();
        let zero = || Ok(Some(Rc::new(Decimal::ZERO) as BoxedValue));
        if input.is_empty() {
            return zero();
        }
        // The first run of digits, points and commas.
        let is_number_character = |c: char| c.is_ascii_digit() || c == '.' || c == ',';
        let Some(start) = input.find(is_number_character) else {
            return zero();
        };
        let rest = &input[start..];
        let number = &rest[..rest.find(|c| !is_number_character(c)).unwrap_or(rest.len())];

        match Decimal::parse(number) {
            Ok(value) => Ok(Some(Rc::new(value) as BoxedValue)),
            Err(error) => Err(BindingError::message(&error.to_string())),
        }
    }

    fn convert_back(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, BindingError> {
        let Some(input_number) = value.and_then(|value| value.downcast_ref::<Decimal>()) else {
            return Ok(None);
        };
        let text = input_number
            .to_string_with(self.format, Some(&NumberFormatInfo::invariant_info()))
            .map_err(|error| BindingError::message(&error.to_string()))?;
        Ok(Some(Rc::new(text) as BoxedValue))
    }
}

#[test]
fn text_converter_is_applied_immediately() {
    run_test(|control, _textbox| {
        let value = d("10.11");
        let initial_converter: Rc<dyn IValueConverter> = Rc::new(TestNumericUpDownValueConverter { format: "C2" });
        let new_converter: Rc<dyn IValueConverter> = Rc::new(TestNumericUpDownValueConverter { format: "P2" });

        // Establish and verify initial conditions.
        control.set_numeric_value(Some(value));
        control.set_text_converter(Some(initial_converter));
        let old_text = control.text().unwrap_or_default();
        assert_eq!("\u{00A4}10.11", old_text);

        // Check that the converter is applied.
        control.set_text_converter(Some(new_converter));
        let new_text = control.text().unwrap_or_default();
        assert_eq!("1,011.00 %", new_text);
    });
}

type IncrementDecrementRow = (Decimal, Decimal, Option<Decimal>, SpinDirection, Option<Decimal>);

fn increment_decrement_test_data() -> Vec<IncrementDecrementRow> {
    vec![
        // If min and max are not defined and the value was null, 0 should
        // be the new value after a spin.
        (Decimal::MIN_VALUE, Decimal::MAX_VALUE, None, SpinDirection::Decrease, Some(d("0"))),
        (Decimal::MIN_VALUE, Decimal::MAX_VALUE, None, SpinDirection::Increase, Some(d("0"))),
        // If no value was defined, but min or max are defined, use these as
        // the new value.
        (d("-400"), d("-200"), None, SpinDirection::Decrease, Some(d("-200"))),
        (d("200"), d("400"), None, SpinDirection::Increase, Some(d("200"))),
        // The value should be clamped to min / max after spinning.
        (d("200"), d("400"), Some(d("5")), SpinDirection::Increase, Some(d("200"))),
        (d("200"), d("400"), Some(d("200")), SpinDirection::Decrease, Some(d("200"))),
    ]
}

fn run_test(test: impl FnOnce(&Ref<NumericUpDown>, &Ref<TextBox>)) {
    let _app = start();

    let control = create_control();
    let text_box = get_text_box(&control);
    let window = Window::new();
    window.set_content(Some(Control::boxed(control.clone())));
    window.apply_styling();
    window.apply_template();
    window.presenter().expect("the window has a presenter").apply_template();
    Dispatcher::ui_thread().run_jobs(None);
    test(&control, &text_box);
}

fn create_control() -> Ref<NumericUpDown> {
    let control = NumericUpDown::new();
    control.set_template(Some(create_template()));

    control.apply_template();
    control
}

fn get_text_box(control: &Ref<NumericUpDown>) -> Ref<TextBox> {
    control
        .get_template_descendants()
        .into_iter()
        .filter_map(|descendant| descendant.cast::<ButtonSpinner>())
        .filter_map(|spinner| spinner.content())
        .filter_map(|content| Control::from_boxed(&content).and_then(|control| control.cast::<TextBox>()))
        .next()
        .expect("the template has a text box")
}

fn get_spinner(control: &Ref<NumericUpDown>) -> Ref<ButtonSpinner> {
    control
        .get_template_descendants()
        .into_iter()
        .find_map(|descendant| descendant.cast::<ButtonSpinner>())
        .expect("the template has a button spinner")
}

fn create_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<NumericUpDown>(|_control, scope| {
        let text_box = TextBox::new();
        text_box.set_name(Some("PART_TextBox".to_string()));
        let text_box = text_box.register_in_name_scope(&**scope);

        let spinner = ButtonSpinner::new();
        spinner.set_name(Some("PART_Spinner".to_string()));
        spinner.set_content(Some(Control::boxed(text_box)));
        spinner.register_in_name_scope(&**scope).upcast()
    })
}

#[test]
fn tab_index_should_be_synchronized_with_inner_text_box() {
    run_test(|control, textbox| {
        // Set the tab index on the control.
        control.set_tab_index(5);

        // The inner text box should have the same tab index.
        assert_eq!(5, textbox.tab_index());

        // Change the tab index and verify it gets synchronized.
        control.set_tab_index(10);
        assert_eq!(10, textbox.tab_index());
    });
}

#[test]
fn placeholder_foreground_can_be_set() {
    let _app = start();

    let control = create_control();
    control.set_placeholder_text(Some("Enter value"));
    let red: Rc<dyn IBrush> = Brushes::red();
    control.set_placeholder_foreground(Some(red.clone()));

    assert!(control.placeholder_foreground() == Some(red));
}

// ---------------------------------------------------------------------------
// Additional tests (not in the reference test class). The expectations are
// those of the reference control, read from its source.
// ---------------------------------------------------------------------------

fn spin(control: &Ref<NumericUpDown>, direction: SpinDirection) -> SpinEventArgs {
    let e = SpinEventArgs::with_event(Some(Spinner::spin_event()), direction);
    get_spinner(control).raise_event(&e);
    e
}

fn press_enter(control: &Ref<NumericUpDown>) -> KeyEventArgs {
    let mut e = KeyEventArgs::new();
    e.set_routed_event(Some(InputElement::key_down_event()));
    e.key = Key::Enter;
    control.raise_event(&e);
    e
}

fn lose_focus(control: &Ref<NumericUpDown>) {
    control.raise_event(&FocusChangedEventArgs::new(InputElement::lost_focus_event()));
}

#[test]
fn additional_spin_adds_and_subtracts_the_increment_and_clamps() {
    run_test(|control, textbox| {
        control.set_minimum(d("0"));
        control.set_maximum(d("6"));
        control.set_increment(d("2.5"));
        control.set_numeric_value(Some(d("1")));
        assert_eq!(Some("1"), control.text().as_deref());

        let e = spin(control, SpinDirection::Increase);
        assert!(e.handled());
        assert_eq!(Some(d("3.5")), control.value());
        assert_eq!(Some("3.5"), control.text().as_deref());
        assert_eq!(Some("3.5"), textbox.text().as_deref());

        // 3.5 + 2.5 is the maximum (the sum has the scale of the operands).
        spin(control, SpinDirection::Increase);
        assert_eq!(Some("6.0"), control.text().as_deref());
        assert_eq!(ValidSpinDirections::DECREASE, get_spinner(control).valid_spin_direction());

        // At the maximum a spin up does nothing (the direction is not valid).
        spin(control, SpinDirection::Increase);
        assert_eq!(Some("6.0"), control.text().as_deref());

        // A sum beyond the maximum is clamped to it.
        control.set_numeric_value(Some(d("4")));
        spin(control, SpinDirection::Increase);
        assert_eq!(Some("6"), control.text().as_deref());

        spin(control, SpinDirection::Decrease);
        assert_eq!(Some("3.5"), control.text().as_deref());
        assert_eq!(
            ValidSpinDirections::INCREASE | ValidSpinDirections::DECREASE,
            get_spinner(control).valid_spin_direction()
        );
        spin(control, SpinDirection::Decrease);
        assert_eq!(Some("1.0"), control.text().as_deref());
        // 1.0 - 2.5 is clamped to the minimum.
        spin(control, SpinDirection::Decrease);
        assert_eq!(Some("0"), control.text().as_deref());
        assert_eq!(ValidSpinDirections::INCREASE, get_spinner(control).valid_spin_direction());
        spin(control, SpinDirection::Decrease);
        assert_eq!(Some(d("0")), control.value());
    });
}

#[test]
fn additional_spin_at_the_end_of_the_decimal_range_stops_at_the_maximum() {
    // Port difference: the reference adds unchecked, which throws here.
    run_test(|control, _textbox| {
        control.set_numeric_value(Some(Decimal::MAX_VALUE - d("0.5")));
        spin(control, SpinDirection::Increase);
        assert_eq!(Some(Decimal::MAX_VALUE), control.value());

        control.set_numeric_value(Some(Decimal::MIN_VALUE + d("0.5")));
        spin(control, SpinDirection::Decrease);
        assert_eq!(Some(Decimal::MIN_VALUE), control.value());
    });
}

#[test]
fn additional_spin_is_ignored_when_not_allowed_read_only_or_from_the_wheel_without_focus() {
    run_test(|control, _textbox| {
        control.set_numeric_value(Some(d("1")));
        let spinned = Rc::new(std::cell::Cell::new(0));
        let _subscription = control.spinned({
            let spinned = spinned.clone();
            move |_| spinned.set(spinned.get() + 1)
        });

        control.set_allow_spin(false);
        let e = spin(control, SpinDirection::Increase);
        assert!(!e.handled());
        assert_eq!(Some(d("1")), control.value());
        control.set_allow_spin(true);

        control.set_is_read_only(true);
        assert_eq!(ValidSpinDirections::NONE, get_spinner(control).valid_spin_direction());
        let e = spin(control, SpinDirection::Increase);
        assert!(!e.handled());
        assert_eq!(Some(d("1")), control.value());
        control.set_is_read_only(false);

        // The mouse wheel spins only while the text box has the focus.
        let e = SpinEventArgs::with_event_and_mouse_wheel(Some(Spinner::spin_event()), SpinDirection::Increase, true);
        get_spinner(control).raise_event(&e);
        assert!(!e.handled());
        assert_eq!(Some(d("1")), control.value());
        assert_eq!(0, spinned.get());

        let e = spin(control, SpinDirection::Increase);
        assert!(e.handled());
        assert_eq!(Some(d("2")), control.value());
        assert_eq!(1, spinned.get());

        // A zero increment prevents the spin.
        control.set_increment(d("0"));
        assert_eq!(ValidSpinDirections::NONE, get_spinner(control).valid_spin_direction());
        spin(control, SpinDirection::Increase);
        assert_eq!(Some(d("2")), control.value());
        // The event is raised all the same.
        assert_eq!(2, spinned.get());
    });
}

#[test]
fn additional_valid_text_updates_the_value_and_is_committed_on_enter_and_lost_focus() {
    run_test(|control, textbox| {
        let changes = Rc::new(std::cell::RefCell::new(Vec::new()));
        control.value_changed({
            let changes = changes.clone();
            move |_, e| changes.borrow_mut().push((e.old_value(), e.new_value()))
        });

        // Text typed into the text box updates the value at once and is not
        // touched.
        textbox.set_text(Some("42.50"));
        assert_eq!(Some(d("42.5")), control.value());
        assert_eq!(Some("42.50"), control.text().as_deref());
        assert_eq!(vec![(None, Some(d("42.5")))], *changes.borrow());

        // A successful commit leaves the key to others.
        let e = press_enter(control);
        assert!(!e.handled());
        assert_eq!(Some("42.50"), control.text().as_deref());

        // Text that is the same number does not change the value.
        textbox.set_text(Some("42.5"));
        assert_eq!(1, changes.borrow().len());

        // Losing the focus commits and forces the text of the value.
        textbox.set_text(Some(" 1,000 "));
        assert_eq!(Some(d("1000")), control.value());
        assert_eq!(Some(" 1,000 "), control.text().as_deref());
        lose_focus(control);
        assert_eq!(Some("1000"), control.text().as_deref());
        assert_eq!(Some("1000"), textbox.text().as_deref());

        // Empty text is no value.
        textbox.set_text(Some(""));
        assert_eq!(None, control.value());
        assert_eq!(
            ValidSpinDirections::INCREASE | ValidSpinDirections::DECREASE,
            get_spinner(control).valid_spin_direction()
        );
    });
}

#[test]
fn additional_invalid_text_keeps_the_value_and_is_replaced_on_lost_focus() {
    run_test(|control, textbox| {
        control.set_numeric_value(Some(d("5")));

        textbox.set_text(Some("abc"));
        assert_eq!(Some(d("5")), control.value());
        assert_eq!(Some("abc"), control.text().as_deref());
        // Invalid text typed by the user disables the spinner.
        assert_eq!(ValidSpinDirections::NONE, get_spinner(control).valid_spin_direction());

        // A failed commit handles the key and keeps the text.
        let e = press_enter(control);
        assert!(e.handled());
        assert_eq!(Some("abc"), control.text().as_deref());
        assert_eq!(Some(d("5")), control.value());

        // Losing the focus puts the text of the value back.
        lose_focus(control);
        assert_eq!(Some("5"), control.text().as_deref());
        assert_eq!(Some("5"), textbox.text().as_deref());
        assert_eq!(Some(d("5")), control.value());
        assert_eq!(
            ValidSpinDirections::INCREASE | ValidSpinDirections::DECREASE,
            get_spinner(control).valid_spin_direction()
        );
    });
}

#[test]
fn additional_text_out_of_range_is_invalid_unless_the_value_is_clipped() {
    run_test(|control, textbox| {
        control.set_minimum(d("0"));
        control.set_maximum(d("10"));
        control.set_numeric_value(Some(d("5")));

        textbox.set_text(Some("20"));
        assert_eq!(Some(d("5")), control.value());
        assert!(press_enter(control).handled());
        lose_focus(control);
        assert_eq!(Some("5"), control.text().as_deref());

        control.set_clip_value_to_min_max(true);
        textbox.set_text(Some("20"));
        assert_eq!(Some(d("10")), control.value());
        assert_eq!(Some("20"), control.text().as_deref());
        assert!(!press_enter(control).handled());
        lose_focus(control);
        assert_eq!(Some("10"), control.text().as_deref());

        textbox.set_text(Some("-3"));
        assert_eq!(Some(d("0")), control.value());
    });
}

#[test]
fn additional_parsing_number_style_is_used() {
    run_test(|control, textbox| {
        control.set_numeric_value(Some(d("5")));

        // The default styles permit an exponent, parentheses and a currency
        // symbol.
        textbox.set_text(Some("1e2"));
        assert_eq!(Some(d("100")), control.value());
        textbox.set_text(Some("(7)"));
        assert_eq!(Some(d("-7")), control.value());

        control.set_parsing_number_style(NumberStyles::INTEGER);
        textbox.set_text(Some("1.5"));
        assert_eq!(Some(d("-7")), control.value());
        assert_eq!(ValidSpinDirections::NONE, get_spinner(control).valid_spin_direction());
        textbox.set_text(Some(" -12 "));
        assert_eq!(Some(d("-12")), control.value());

        // Styles that decimals do not support make every text invalid.
        control.set_parsing_number_style(NumberStyles::HEX_NUMBER);
        textbox.set_text(Some("1F"));
        assert_eq!(Some(d("-12")), control.value());
        assert!(press_enter(control).handled());
    });
}

#[test]
fn additional_format_strings_are_displayed_and_parsed_back() {
    run_test(|control, textbox| {
        // A composite format string.
        control.set_format_string("{0:N2} \u{00B0}");
        control.set_numeric_value(Some(d("1234.5")));
        assert_eq!(Some("1,234.50 \u{00B0}"), control.text().as_deref());
        assert_eq!(Some("1,234.50 \u{00B0}"), textbox.text().as_deref());

        // The text of the value is the value, although it does not parse.
        lose_focus(control);
        assert_eq!(Some(d("1234.5")), control.value());

        // Text without every special character of the current text (here
        // the separators) is not valid.
        textbox.set_text(Some("45 \u{00B0}"));
        assert_eq!(Some(d("1234.5")), control.value());
        lose_focus(control);
        assert_eq!(Some("1,234.50 \u{00B0}"), control.text().as_deref());

        // Formatting rounds midpoints away from zero.
        control.set_format_string("{0:0} \u{00B0}");
        assert_eq!(Some("1235 \u{00B0}"), control.text().as_deref());
        control.set_numeric_value(Some(d("90")));
        assert_eq!(Some("90 \u{00B0}"), control.text().as_deref());

        // Text with the special characters of the current text is parsed
        // without them (every character that is not a digit goes).
        textbox.set_text(Some("45 \u{00B0}"));
        assert_eq!(Some(d("45")), control.value());
        lose_focus(control);
        assert_eq!(Some("45 \u{00B0}"), control.text().as_deref());
        textbox.set_text(Some("-4.6 \u{00B0}"));
        assert_eq!(Some(d("46")), control.value());

        textbox.set_text(Some("47 deg"));
        assert_eq!(Some(d("46")), control.value());

        // A digit of another script is not a special character: it stays in
        // the text, which then is not a number.
        textbox.set_text(Some("4\u{0663}7 \u{00B0}"));
        assert_eq!(Some(d("46")), control.value());
        assert_eq!(ValidSpinDirections::NONE, get_spinner(control).valid_spin_direction());
        lose_focus(control);

        // A custom format string.
        control.set_format_string("0.000");
        assert_eq!(Some("46.000"), control.text().as_deref());

        // A percent format string: the text is a percentage.
        control.set_format_string("P0");
        control.set_numeric_value(Some(d("0.5")));
        assert_eq!(Some("50 %"), control.text().as_deref());
        textbox.set_text(Some("75 %"));
        assert_eq!(Some(d("0.75")), control.value());
        textbox.set_text(Some("20"));
        assert_eq!(Some(d("0.2")), control.value());

        // A "P" between quotes is text.
        control.set_format_string("0' P'");
        control.set_numeric_value(Some(d("3")));
        assert_eq!(Some("3 P"), control.text().as_deref());

        // No value has no text; in a composite format string it has the
        // text around the item.
        control.set_format_string("F1");
        control.set_numeric_value(None);
        assert_eq!(None, control.text());
        control.set_format_string("{0:F1} m");
        assert_eq!(Some(" m"), control.text().as_deref());
    });
}

#[test]
fn additional_minimum_and_maximum_are_coerced_against_each_other_when_set() {
    let control = create_control();

    // The maximum is not less than the minimum.
    control.set_minimum(d("10"));
    control.set_maximum(d("5"));
    assert_eq!(d("10"), control.maximum());
    assert_eq!(d("10"), control.minimum());

    // The minimum is not greater than the maximum.
    let control = create_control();
    control.set_maximum(d("5"));
    control.set_minimum(d("10"));
    assert_eq!(d("5"), control.minimum());
    assert_eq!(d("5"), control.maximum());

    // The other bound is not coerced again when a bound changes.
    control.set_maximum(d("20"));
    assert_eq!(d("5"), control.minimum());
    assert_eq!(d("20"), control.maximum());

    // Of two equal bounds the reference keeps the given maximum and the
    // current maximum (the first argument of the larger-of, the second of
    // the smaller-of).
    let control = create_control();
    control.set_minimum(d("5"));
    control.set_maximum(d("5.00"));
    assert_eq!("5.00", control.maximum().to_string());
    let control = create_control();
    control.set_maximum(d("5"));
    control.set_minimum(d("5.00"));
    assert_eq!("5", control.minimum().to_string());

    // The value is not coerced into the range.
    let control = create_control();
    control.set_maximum(d("10"));
    control.set_numeric_value(Some(d("20")));
    assert_eq!(Some(d("20")), control.value());
    control.set_minimum(d("30"));
    assert_eq!(d("10"), control.minimum());
    assert_eq!(Some(d("20")), control.value());
}

#[test]
fn additional_clip_value_to_min_max_clamps_the_value_when_a_bound_changes() {
    let control = create_control();
    control.set_numeric_value(Some(d("20")));

    // Not clipped: the bounds leave the value alone.
    control.set_maximum(d("15"));
    assert_eq!(Some(d("20")), control.value());

    // Turning the clipping on does not clamp by itself.
    control.set_clip_value_to_min_max(true);
    assert_eq!(Some(d("20")), control.value());

    control.set_maximum(d("10"));
    assert_eq!(Some(d("10")), control.value());

    control.set_minimum(d("3"));
    assert_eq!(Some(d("10")), control.value());
    control.set_numeric_value(Some(d("1")));
    // Setting the value does not clamp it.
    assert_eq!(Some(d("1")), control.value());
    control.set_minimum(d("4"));
    assert_eq!(Some(d("4")), control.value());

    // No value stays no value.
    control.set_numeric_value(None);
    control.set_minimum(d("5"));
    assert_eq!(None, control.value());
}
