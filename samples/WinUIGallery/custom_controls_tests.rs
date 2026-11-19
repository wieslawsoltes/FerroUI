//! The CustomUserControls page: the custom controls of `custom_controls.rs` in their templates.

use crate::custom_controls::{
    convert_to_fahrenheit, CounterControl, CounterMode, ValidatedPasswordBox, MISSING_NUMBER, MISSING_UPPERCASE,
    TOO_SHORT, VALID,
};
use crate::{examples, tests::start};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::Ref;
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::{Button, Control, ContentControl, StackPanel, TextBlock, TextBox, UserControl, Window};

fn page_in_window() -> (Ref<Control>, Ref<Window>) {
    let root = examples::create(examples::find("CustomUserControls").unwrap())
        .unwrap_or_else(|error| panic!("CustomUserControls: {error}"));
    let window = Window::new();
    window.set_content(Some(Control::boxed(root.clone())));
    window.show();
    window.update_layout();
    (root, window)
}

fn click(button: &Button) {
    button.raise_event(&RoutedEventArgs::with_event(&Button::click_event()));
}

#[test]
fn counter_controls_count_up_and_down_in_their_templates() {
    let _application = start();
    let (root, window) = page_in_window();
    let counters = root.get_control::<StackPanel>("CounterHost");
    let counter = |index: usize| counters.children().get(index).cast::<CounterControl>().unwrap();
    for (index, mode, label, step) in [
        (0, CounterMode::Increment, "Increase", 1),
        (1, CounterMode::Decrement, "Decrease", -1),
    ] {
        let counter = counter(index);
        assert_eq!(counter.mode(), mode);
        // OnApplyTemplate found the parts: the button says what it does.
        let button = counter
            .get_visual_descendants()
            .find_map(|visual| visual.cast::<Button>())
            .expect("the ActionButton of the template");
        assert_eq!(
            button.content().and_then(|content| content.downcast_ref::<String>().cloned()).as_deref(),
            Some(label)
        );
        assert_eq!(AutomationProperties::get_name(&button), Some(format!("{label} counter")));
        click(&button);
        click(&button);
        assert_eq!(counter.count(), 2 * step);
        let text = counter
            .get_visual_descendants()
            .find_map(|visual| {
                visual.cast::<TextBlock>().filter(|text| text.name().as_deref() == Some("CountText"))
            })
            .expect("the CountText of the template");
        assert_eq!(text.text(), Some((2 * step).to_string()));
    }
    window.close();
}

#[test]
fn validated_password_box_reports_what_the_password_misses() {
    assert_eq!(
        ValidatedPasswordBox::validate("abc", 8),
        (false, vec![MISSING_UPPERCASE, MISSING_NUMBER, TOO_SHORT])
    );
    assert_eq!(ValidatedPasswordBox::validate("Abcdefg1", 8), (true, vec![VALID]));
    assert_eq!(ValidatedPasswordBox::validate("abcdefg1", 8), (false, vec![MISSING_UPPERCASE]));
    let _application = start();
    let (root, window) = page_in_window();
    let password = root
        .get_control::<StackPanel>("PasswordHost")
        .children()
        .get(0)
        .cast::<ValidatedPasswordBox>()
        .unwrap();
    let submit = root.get_control::<Button>("SubmitButton");
    assert!(!submit.is_enabled());
    let input = password
        .get_visual_descendants()
        .find_map(|visual| visual.cast::<TextBox>())
        .expect("the PasswordInput of the template");
    assert_eq!(input.placeholder_text().as_deref(), Some("Enter password..."));
    // The password box's text is the Password, and IsValid enables Submit.
    input.set_text(Some("short"));
    assert_eq!(password.password(), "short");
    assert!(!password.is_valid() && !submit.is_enabled());
    input.set_text(Some("Longer password 1"));
    assert!(password.is_valid() && submit.is_enabled());
    let lines = password
        .get_visual_descendants()
        .find_map(|visual| {
            visual.cast::<StackPanel>().filter(|panel| panel.name().as_deref() == Some("ValidationLines"))
        })
        .expect("the ValidationLines of the template");
    assert!(lines.is_visible());
    assert_eq!(lines.children().count(), 1);
    input.set_text(Some(""));
    assert!(!lines.is_visible());
    window.close();
}

#[test]
fn temperature_converter_user_control_converts_celsius() {
    assert_eq!(convert_to_fahrenheit("100"), "Fahrenheit: 212.00°F");
    assert_eq!(convert_to_fahrenheit("-40"), "Fahrenheit: -40.00°F");
    assert_eq!(convert_to_fahrenheit("warm"), "Invalid input!");
    let _application = start();
    let (root, window) = page_in_window();
    let converter = root
        .get_control::<ContentControl>("TemperatureHost")
        .content()
        .and_then(|content| Control::from_boxed(&content))
        .and_then(|control| control.cast::<UserControl>())
        .expect("the TemperatureConverterControl");
    let input = converter.get_control::<TextBox>("InputTextBox");
    let button = converter.get_control::<Button>("ConvertButton");
    // HasText(InputTextBox.Text) enables the button.
    assert!(!button.is_enabled());
    input.set_text(Some("37"));
    assert!(button.is_enabled());
    click(&button);
    assert_eq!(
        converter.get_control::<TextBlock>("ResultTextBlock").text().as_deref(),
        Some("Fahrenheit: 98.60°F")
    );
    input.set_text(Some("   "));
    assert!(!button.is_enabled());
    window.close();
}
