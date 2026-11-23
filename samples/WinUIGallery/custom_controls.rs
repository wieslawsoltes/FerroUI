//! The custom controls of the pinned CustomUserControls page in Rust: `CounterControl.cs` and
//! `ValidatedPasswordBox.cs`, templated controls whose default styles are the control themes of
//! the page (`CounterControl.xaml` and `ValidatedPasswordBox.xaml`), and the UserControl
//! `TemperatureConverterControl` with the code behind of `TemperatureConverterControl.xaml.cs`.
//!
//! WinUI's `DefaultStyleKey` is the control theme the page gives the control (a native page has
//! no Generic.xaml of its own), `GetTemplateChild` is the name scope of the applied template, and
//! a `DependencyProperty` is a styled property. FerroUI has no PasswordBox: the password box of
//! the template is a TextBox with a PasswordChar.

use ferroui_base::controls::ResourceKey;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{Color, IBrush};
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_markup_enum, ferro_properties, instantiate,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledElementImpl,
    StyledProperty, VisualImpl,
};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::primitives::{
    TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt,
};
use ferroui_base::styling::ControlTheme;
use ferroui_controls::{
    Button, ContentControl, Control, ControlImpl, FontIcon, StackPanel, TextBlock, TextBox, UserControl,
};
use ferroui_markup_xaml::XamlLoadError;
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use std::cell::RefCell;
use std::rc::Rc;

// --- CounterControl -----------------------------------------------------------------------------

/// `CounterMode`: whether the counter's button increments or decrements its count.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum CounterMode {
    #[default]
    Increment = 0,
    Decrement = 1,
}

ferro_markup_enum!(CounterMode { Increment, Decrement });

/// `CounterControl`: a count and a button that increments or decrements it, its template parts
/// `ActionButton` and `CountText`.
#[repr(C)]
pub struct CounterControl {
    base: TemplatedControl,
    action_button: RefCell<Option<Ref<Button>>>,
    count_text: RefCell<Option<Ref<TextBlock>>>,
}

ferro_class!(CounterControl: TemplatedControl);
ferro_class_info!(CounterControl { new: CounterControl::new });
ferro_impl_classes!(
    CounterControl: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

ferro_properties! {
    impl CounterControl {
        pub fn count_property() -> StyledProperty<i32> {
            FerroProperty::register::<CounterControl, _>("Count", 0)
        }

        pub fn mode_property() -> StyledProperty<CounterMode> {
            FerroProperty::register::<CounterControl, _>("Mode", CounterMode::Increment)
        }
    }
}

impl TemplatedControlImpl for CounterControl {
    /// `OnApplyTemplate`: the template's button changes the count by the mode, and its content
    /// and automation name follow the mode.
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);
        let button = e.name_scope().find_as::<Button>("ActionButton");
        *this.count_text.borrow_mut() = e.name_scope().find_as::<TextBlock>("CountText");
        *this.action_button.borrow_mut() = button.clone();
        if let Some(button) = button {
            let weak = this.to_ref().downgrade();
            button.click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    let step = if this.mode() == CounterMode::Increment { 1 } else { -1 };
                    this.set_count(this.count().saturating_add(step));
                    this.update_ui();
                }
            });
            this.update_button_text();
        }
        this.update_ui();
    }
}

impl CounterControl {
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            action_button: RefCell::new(None),
            count_text: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn count(&self) -> i32 {
        self.get_value(Self::count_property())
    }

    pub fn set_count(&self, value: i32) {
        self.set_value(Self::count_property(), value)
    }

    pub fn mode(&self) -> CounterMode {
        self.get_value(Self::mode_property())
    }

    pub fn set_mode(&self, value: CounterMode) {
        self.set_value(Self::mode_property(), value)
    }

    /// `UpdateUI`: the count's text. Its live region (`AutomationProperties.LiveSetting` of the
    /// template) announces the change, as the source's `LiveRegionChanged`.
    fn update_ui(&self) {
        if let Some(text) = self.count_text.borrow().clone() {
            text.set_text(Some(&self.count().to_string()));
        }
    }

    /// `UpdateButtonText`: Increase or Decrease, and the automation name of the button.
    fn update_button_text(&self) {
        if let Some(button) = self.action_button.borrow().clone() {
            let text = if self.mode() == CounterMode::Increment { "Increase" } else { "Decrease" };
            button.set_content(Some(Rc::new(text.to_string())));
            AutomationProperties::set_name(&button, Some(&format!("{text} counter")));
        }
    }
}

// --- ValidatedPasswordBox -----------------------------------------------------------------------

/// The glyphs and messages of `UpdateValidationMessages`.
pub(crate) const VALID: (&str, &str) = ("\u{E930}", "Password is valid");
pub(crate) const MISSING_UPPERCASE: (&str, &str) = ("\u{EA39}", "Missing uppercase");
pub(crate) const MISSING_NUMBER: (&str, &str) = ("\u{EA39}", "Missing number");
pub(crate) const TOO_SHORT: (&str, &str) = ("\u{EA39}", "Too short!");

/// `ValidatedPasswordBox`: a password box whose validation lines say what the password misses,
/// its template parts `PasswordInput`, `HeaderText` (WinUI's PasswordBox Header) and
/// `ValidationLines` (the source's `ValidationRichText`; each line is its icon and its message).
#[repr(C)]
pub struct ValidatedPasswordBox {
    base: TemplatedControl,
    password_input: RefCell<Option<Ref<TextBox>>>,
    validation_lines: RefCell<Option<Ref<StackPanel>>>,
}

ferro_class!(ValidatedPasswordBox: TemplatedControl);
ferro_class_info!(ValidatedPasswordBox { new: ValidatedPasswordBox::new });
ferro_impl_classes!(
    ValidatedPasswordBox: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

ferro_properties! {
    impl ValidatedPasswordBox {
        pub fn password_property() -> StyledProperty<String> {
            FerroProperty::register::<ValidatedPasswordBox, _>("Password", String::new())
        }

        pub fn is_valid_property() -> StyledProperty<bool> {
            FerroProperty::register::<ValidatedPasswordBox, _>("IsValid", false)
        }

        pub fn min_length_property() -> StyledProperty<i32> {
            FerroProperty::register::<ValidatedPasswordBox, _>("MinLength", 8)
        }

        pub fn header_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<ValidatedPasswordBox, _>("Header", None)
        }

        pub fn placeholder_text_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<ValidatedPasswordBox, _>("PlaceholderText", None)
        }
    }
}

impl FerroObjectImpl for ValidatedPasswordBox {
    /// `OnPasswordChanged` of the source's properties.
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::password_property().as_property()
            || change.property() == Self::min_length_property().as_property()
            || change.property() == Self::header_property().as_property()
            || change.property() == Self::placeholder_text_property().as_property()
        {
            this.update_validation_messages();
        }
    }
}

impl TemplatedControlImpl for ValidatedPasswordBox {
    /// `OnApplyTemplate`: the header and placeholder of the password box, its changes to
    /// `Password`, and the validation lines, which follow the theme.
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);
        let input = e.name_scope().find_as::<TextBox>("PasswordInput");
        *this.validation_lines.borrow_mut() = e.name_scope().find_as::<StackPanel>("ValidationLines");
        if let Some(header) = e.name_scope().find_as::<TextBlock>("HeaderText") {
            header.set_text(this.header().as_deref());
            header.set_is_visible(this.header().is_some_and(|header| !header.is_empty()));
        }
        if let Some(input) = input.clone() {
            input.set_placeholder_text(this.placeholder_text().as_deref());
            AutomationProperties::set_name(&input, this.header().as_deref());
            let weak = this.to_ref().downgrade();
            let weak_input = input.downgrade();
            input.property_changed(move |change| {
                if change.property() != TextBox::text_property().as_property() {
                    return;
                }
                if let (Some(this), Some(input)) = (weak.upgrade(), weak_input.upgrade()) {
                    this.set_password(input.text().unwrap_or_default());
                }
            });
        }
        *this.password_input.borrow_mut() = input;
        let weak = this.to_ref().downgrade();
        this.actual_theme_variant_changed(move || {
            if let Some(this) = weak.upgrade() {
                this.update_validation_messages();
            }
        })
        .detach();
        this.update_validation_messages();
    }
}

impl ValidatedPasswordBox {
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            password_input: RefCell::new(None),
            validation_lines: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn password(&self) -> String {
        self.get_value(Self::password_property())
    }

    pub fn set_password(&self, value: String) {
        self.set_value(Self::password_property(), value)
    }

    pub fn is_valid(&self) -> bool {
        self.get_value(Self::is_valid_property())
    }

    pub fn min_length(&self) -> i32 {
        self.get_value(Self::min_length_property())
    }

    pub fn set_min_length(&self, value: i32) {
        self.set_value(Self::min_length_property(), value)
    }

    pub fn header(&self) -> Option<String> {
        self.get_value(Self::header_property())
    }

    pub fn set_header(&self, value: Option<String>) {
        self.set_value(Self::header_property(), value)
    }

    pub fn placeholder_text(&self) -> Option<String> {
        self.get_value(Self::placeholder_text_property())
    }

    pub fn set_placeholder_text(&self, value: Option<String>) {
        self.set_value(Self::placeholder_text_property(), value)
    }

    /// The validation lines of `password` (the source's `UpdateValidationMessages`): whether it
    /// is valid, and the icon and message of each line.
    pub(crate) fn validate(password: &str, min_length: i32) -> (bool, Vec<(&'static str, &'static str)>) {
        let has_min_length = i32::try_from(password.chars().count()).unwrap_or(i32::MAX) >= min_length;
        let has_uppercase = password.chars().any(char::is_uppercase);
        let has_number = password.chars().any(char::is_numeric);
        let valid = has_min_length && has_uppercase && has_number;
        let mut lines = Vec::new();
        if valid {
            lines.push(VALID);
        } else {
            if !has_uppercase {
                lines.push(MISSING_UPPERCASE);
            }
            if !has_number {
                lines.push(MISSING_NUMBER);
            }
            if !has_min_length {
                lines.push(TOO_SHORT);
            }
        }
        (valid, lines)
    }

    /// `UpdateValidationMessages`: `IsValid`, and the lines of an entered password in the colors
    /// of the theme (critical or success, light or dark).
    fn update_validation_messages(&self) {
        let password = self.password();
        let (valid, lines) = Self::validate(&password, self.min_length());
        self.set_value(Self::is_valid_property(), valid);
        let Some(panel) = self.validation_lines.borrow().clone() else {
            return;
        };
        panel.set_is_visible(!password.is_empty());
        panel.children().clear();
        if password.is_empty() {
            return;
        }
        let light = self.actual_theme_variant() == Some(ThemeVariant::light());
        for (glyph, message) in lines {
            let critical = (glyph, message) != VALID;
            let color = match (critical, light) {
                (true, true) => Color::from_rgb(196, 43, 28),
                (true, false) => Color::from_rgb(255, 153, 164),
                (false, true) => Color::from_rgb(15, 123, 15),
                (false, false) => Color::from_rgb(108, 203, 95),
            };
            let brush: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::new(color));
            let line = StackPanel::new();
            line.set_orientation(ferroui_base::layout::Orientation::Horizontal);
            line.set_spacing(4.0);
            let icon = FontIcon::new();
            icon.set_glyph(glyph);
            icon.set_font_size(14.0);
            icon.set_foreground(Some(brush.clone()));
            let text = TextBlock::new();
            text.set_text(Some(message));
            text.set_foreground(Some(brush));
            line.children().add(&icon);
            line.children().add(&text);
            panel.children().add(&line);
        }
    }
}

// --- TemperatureConverterControl ----------------------------------------------------------------

/// `Button_Click` of `TemperatureConverterControl.xaml.cs`: the Fahrenheit of a number of degrees
/// Celsius, or the source's message for other input. The number is read as Rust reads a float
/// (the invariant culture), where the source's `double.TryParse` reads the current culture.
pub(crate) fn convert_to_fahrenheit(input: &str) -> String {
    match input.trim().parse::<f64>() {
        Ok(celsius) => format!("Fahrenheit: {:.2}°F", celsius * 9.0 / 5.0 + 32.0),
        Err(_) => "Invalid input!".to_string(),
    }
}

/// `TemperatureConverterControl`, the UserControl of `TemperatureConverterControl.xaml`, with its
/// code behind: `HasText(InputTextBox.Text)` enables the button, and `Button_Click` writes the
/// result.
pub(crate) fn temperature_converter() -> Result<Ref<UserControl>, XamlLoadError> {
    let control = FerroRuntimeXamlLoader::parse_as::<Ref<UserControl>>(
        include_str!("Pages/TemperatureConverterControl.xaml"),
        None,
    )?;
    let input = control.get_control::<TextBox>("InputTextBox");
    let button = control.get_control::<Button>("ConvertButton");
    let weak_input = input.downgrade();
    let weak_button = button.downgrade();
    input.property_changed(move |change| {
        if change.property() != TextBox::text_property().as_property() {
            return;
        }
        if let (Some(input), Some(button)) = (weak_input.upgrade(), weak_button.upgrade()) {
            button.set_is_enabled(!input.text().unwrap_or_default().trim().is_empty());
        }
    });
    let weak_input = input.downgrade();
    let weak_result = control.get_control::<TextBlock>("ResultTextBlock").downgrade();
    button.click(move |_, _| {
        if let (Some(input), Some(result)) = (weak_input.upgrade(), weak_result.upgrade()) {
            result.set_text(Some(&convert_to_fahrenheit(&input.text().unwrap_or_default())));
        }
    });
    Ok(control)
}

// --- The page -----------------------------------------------------------------------------------

pub(crate) fn wire(page_id: &str, root: &Ref<Control>) -> Result<(), XamlLoadError> {
    if page_id != "CustomUserControls" {
        return Ok(());
    }
    let theme = |key: &str| {
        root.try_find_resource(&ResourceKey::from(key), None)
            .flatten()
            .and_then(|value| value.downcast_ref::<Ref<ControlTheme>>().cloned())
    };
    // The first example: an incrementing and a decrementing counter.
    let counters = root.get_control::<StackPanel>("CounterHost");
    for (name, mode) in [
        ("IncrementCounter", CounterMode::Increment),
        ("DecrementCounter", CounterMode::Decrement),
    ] {
        let counter = CounterControl::new();
        counter.set_name(Some(name.to_string()));
        counter.set_mode(mode);
        counter.set_theme(theme("CounterControlTheme"));
        counters.children().add(&counter);
    }
    // The second example: the validated password box above the Submit button, whose IsEnabled
    // follows its IsValid (the source's OneWay x:Bind).
    let host = root.get_control::<StackPanel>("PasswordHost");
    let password = ValidatedPasswordBox::new();
    password.set_name(Some("PasswordInput".to_string()));
    password.set_min_length(8);
    password.set_width(240.0);
    password.set_horizontal_alignment(ferroui_base::layout::HorizontalAlignment::Left);
    password.set_header(Some("Password".to_string()));
    password.set_placeholder_text(Some("Enter password...".to_string()));
    password.set_theme(theme("ValidatedPasswordBoxTheme"));
    host.children().insert(0, &password);
    let weak_password = password.downgrade();
    let weak_submit = root.get_control::<Button>("SubmitButton").downgrade();
    password.property_changed(move |change| {
        if change.property() != ValidatedPasswordBox::is_valid_property().as_property() {
            return;
        }
        if let (Some(password), Some(submit)) = (weak_password.upgrade(), weak_submit.upgrade()) {
            submit.set_is_enabled(password.is_valid());
        }
    });
    // The third example: the UserControl.
    let converter = temperature_converter()?;
    root.get_control::<ContentControl>("TemperatureHost")
        .set_content(Some(Control::boxed(converter)));
    Ok(())
}
