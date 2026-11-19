//! The CustomUserControls page: the custom controls of `custom_controls.rs`.

use crate::custom_controls::{
    convert_to_fahrenheit, CounterControl, CounterMode, ValidatedPasswordBox, MISSING_NUMBER, MISSING_UPPERCASE,
    TOO_SHORT, VALID,
};
use crate::{examples, tests::start};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::Ref;
use ferroui_controls::{Button, ContentControl, Control, StackPanel, TextBlock, TextBox, UserControl};

fn page() -> Ref<Control> {
    examples::create(examples::find("CustomUserControls").unwrap())
        .unwrap_or_else(|error| panic!("CustomUserControls: {error}"))
}

fn click(button: &Button) {
    button.raise_event(&RoutedEventArgs::with_event(&Button::click_event()));
}

#[test]
fn validated_password_box_rules_are_the_sources() {
    assert_eq!(
        ValidatedPasswordBox::validate("abc", 8),
        (false, vec![MISSING_UPPERCASE, MISSING_NUMBER, TOO_SHORT])
    );
    assert_eq!(ValidatedPasswordBox::validate("Abcdefg1", 8), (true, vec![VALID]));
    assert_eq!(ValidatedPasswordBox::validate("abcdefg1", 8), (false, vec![MISSING_UPPERCASE]));
    assert_eq!(convert_to_fahrenheit("100"), "Fahrenheit: 212.00°F");
    assert_eq!(convert_to_fahrenheit("-40"), "Fahrenheit: -40.00°F");
    assert_eq!(convert_to_fahrenheit("warm"), "Invalid input!");
}

#[test]
fn custom_user_controls_page_creates_the_controls_of_the_three_examples() {
    let _application = start();
    let root = page();
    let counters = root.get_control::<StackPanel>("CounterHost");
    let modes: Vec<CounterMode> = counters
        .children()
        .to_vec()
        .into_iter()
        .filter_map(|child| child.cast::<CounterControl>())
        .map(|counter| counter.mode())
        .collect();
    assert_eq!(modes, [CounterMode::Increment, CounterMode::Decrement]);
    // The validated password box above the Submit button, whose IsEnabled follows IsValid.
    let password = root
        .get_control::<StackPanel>("PasswordHost")
        .children()
        .get(0)
        .cast::<ValidatedPasswordBox>()
        .unwrap();
    assert_eq!(password.min_length(), 8);
    assert_eq!(password.header().as_deref(), Some("Password"));
    assert_eq!(password.placeholder_text().as_deref(), Some("Enter password..."));
    let submit = root.get_control::<Button>("SubmitButton");
    assert!(!password.is_valid() && !submit.is_enabled());
    password.set_password("short".to_string());
    assert!(!password.is_valid() && !submit.is_enabled());
    password.set_password("Longer password 1".to_string());
    assert!(password.is_valid() && submit.is_enabled());
    password.set_password(String::new());
    assert!(!submit.is_enabled());
}

#[test]
fn temperature_converter_user_control_converts_celsius() {
    let _application = start();
    let root = page();
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
}

/// A counter with the page's control theme, alone in a window with the WinUI theme: its template
/// gives the parts that OnApplyTemplate finds.
#[cfg(feature = "theme")]
#[test]
fn counter_controls_count_up_and_down_in_their_templates() {
    use ferroui_base::controls::ResourceKey;
    use ferroui_base::styling::ControlTheme;
    use ferroui_controls::automation::AutomationProperties;
    use ferroui_controls::Window;
    let _application = crate::tests::start_winui();
    let root = page();
    let theme = root
        .try_find_resource(&ResourceKey::from("CounterControlTheme"), None)
        .flatten()
        .and_then(|value| value.downcast_ref::<Ref<ControlTheme>>().cloned())
        .expect("the page's CounterControl theme");
    for (mode, label, step) in [
        (CounterMode::Increment, "Increase", 1),
        (CounterMode::Decrement, "Decrease", -1),
    ] {
        let counter = CounterControl::new();
        counter.set_mode(mode);
        counter.set_theme(Some(theme.clone()));
        let window = Window::new();
        window.set_content(Some(Control::boxed(counter.clone())));
        window.show();
        window.update_layout();
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
        window.close();
    }
}

/// The validated password box alone in a window with the WinUI theme: the TextBox of its Rust
/// template gets the theme's template and the validation lines follow the password.
#[cfg(feature = "theme")]
#[test]
fn validated_password_box_lays_out_its_template_in_a_window() {
    use ferroui_controls::Window;
    let _application = crate::tests::start_winui();
    let password = ValidatedPasswordBox::new();
    password.set_header(Some("Password".to_string()));
    password.set_template(Some(
        crate::custom_controls::validated_password_box_template()
            as std::rc::Rc<dyn ferroui_controls::templates::IControlTemplate>,
    ));
    let window = Window::new();
    window.set_content(Some(Control::boxed(password.clone())));
    window.show();
    window.update_layout();
    let input = password
        .get_visual_descendants()
        .find_map(|visual| visual.cast::<TextBox>())
        .expect("the PasswordInput of the template");
    input.set_text(Some("abc"));
    assert_eq!(password.password(), "abc");
    let lines = password
        .get_visual_descendants()
        .find_map(|visual| {
            visual.cast::<StackPanel>().filter(|panel| panel.name().as_deref() == Some("ValidationLines"))
        })
        .expect("the ValidationLines of the template");
    assert!(lines.is_visible());
    assert_eq!(lines.children().count(), 3);
    window.close();
}

/// The TemperatureConverterControl alone in a window with the WinUI theme.
#[cfg(feature = "theme")]
#[test]
fn temperature_converter_lays_out_in_a_window() {
    use ferroui_controls::Window;
    let _application = crate::tests::start_winui();
    let converter = crate::custom_controls::temperature_converter().unwrap();
    let window = Window::new();
    window.set_content(Some(Control::boxed(converter.clone())));
    window.show();
    window.update_layout();
    assert!(converter.get_control::<TextBox>("InputTextBox").is_visible());
    window.close();
}
