//! The settings of the platform: the theme, the accent colours and the
//! contrast of the system, the sizes and times of taps, the language.
//!
//! The reference reads the values from the application context; here the
//! context is behind [`ISettingsContext`], which the Java layer answers
//! with the numbers (`PlatformHelper`), so that what is made of them is a
//! function of values.

use ferroui_base::input::platform::PlatformHotkeyConfiguration;
use ferroui_base::input::PointerType;
use ferroui_base::media::Color;
use ferroui_base::platform::{
    ColorContrastPreference, DefaultPlatformSettings, IPlatformSettings, PlatformColorValues, PlatformThemeVariant,
};
use ferroui_base::reactive::IDisposable;
use ferroui_base::Size;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

/// The accent colours a context has, as ARGB numbers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AccentColors {
    /// The three accent palettes of the system at tone 500 (API 31).
    System(u32, u32, u32),
    /// The accent colour of the theme of the context.
    Theme(u32),
    /// None could be read.
    None,
}

/// What the reference reads of a context for the colour values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ColorInputs {
    /// Whether the night mode of the configuration is "yes".
    pub night: bool,
    /// Whether the high contrast text of the system is on.
    pub high_contrast: bool,
    pub accents: AccentColors,
}

/// What the reference reads of the view configuration.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct InputConfig {
    /// `ViewConfiguration.getLongPressTimeout`, in milliseconds.
    pub long_press_timeout: i32,
    /// `ViewConfiguration.getMultiPressTimeout` (API 31), in milliseconds.
    pub multi_press_timeout: Option<i32>,
    /// The scaled double tap slop and the scaled touch slop of the view
    /// configuration of the context, in pixels; `None` without one.
    pub slops: Option<(i32, i32)>,
    /// The density of the display metrics of the context.
    pub density: f32,
}

/// The application context, as far as the settings read it.
pub(crate) trait ISettingsContext {
    fn color_inputs(&self) -> ColorInputs;
    fn input_config(&self) -> InputConfig;
    /// The language tag of the first locale of the configuration.
    fn preferred_application_language(&self) -> Option<String>;
    /// Registers the receiver of the configuration changes
    /// (`Intent.ACTION_CONFIGURATION_CHANGED`), which calls
    /// [`AndroidPlatformSettings::on_receive`].
    fn register_configuration_changed_receiver(&self);
}

thread_local! {
    static INSTANCE: RefCell<Option<Rc<AndroidPlatformSettings>>> = const { RefCell::new(None) };
}

pub struct AndroidPlatformSettings {
    base: DefaultPlatformSettings,
    context: Option<Box<dyn ISettingsContext>>,
    color_values: Cell<PlatformColorValues>,
    hold_wait_duration: Cell<Duration>,
    double_tap_time: Cell<Duration>,
    double_tap_size: Cell<Size>,
    tap_size: Cell<Size>,
    latest_language: RefCell<Option<String>>,
}

impl AndroidPlatformSettings {
    /// The settings read from `context`; `None` is a process without an
    /// application context, which has the default values.
    pub(crate) fn new(context: Option<Box<dyn ISettingsContext>>) -> Rc<AndroidPlatformSettings> {
        let base = DefaultPlatformSettings::new();
        let this = AndroidPlatformSettings {
            color_values: Cell::new(base.get_color_values()),
            base,
            context,
            hold_wait_duration: Cell::new(Duration::from_millis(300)),
            double_tap_time: Cell::new(Duration::from_millis(500)),
            double_tap_size: Cell::new(Size::new(16.0, 16.0)),
            tap_size: Cell::new(Size::new(10.0, 10.0)),
            latest_language: RefCell::new(None),
        };

        if let Some(context) = &this.context {
            this.update_input_config_values(context.as_ref());
            this.color_values.set(Self::get_color_values_from_context(context.as_ref()));
            *this.latest_language.borrow_mut() = Self::query_preferred_application_language(Some(context.as_ref()));

            context.register_configuration_changed_receiver();
        }

        let this = Rc::new(this);
        // The receiver of the Java layer finds the settings here.
        INSTANCE.with(|instance| *instance.borrow_mut() = Some(this.clone()));
        this
    }

    fn update_color_values(&self, context: &dyn ISettingsContext) {
        let old_color_values = self.color_values.get();
        let color_values = Self::get_color_values_from_context(context);

        if old_color_values != color_values {
            self.color_values.set(color_values);
            self.base.on_color_values_changed(color_values);
        }
    }

    fn update_preferred_application_language(&self, context: &dyn ISettingsContext) {
        let old_language = self.latest_language.borrow().clone();
        let language = Self::query_preferred_application_language(Some(context));

        if old_language != language {
            *self.latest_language.borrow_mut() = language;
            self.base.on_preferred_application_language_changed();
        }
    }

    pub(crate) fn get_color_values_from_context(context: &dyn ISettingsContext) -> PlatformColorValues {
        let inputs = context.color_inputs();
        let system_theme = if inputs.night { PlatformThemeVariant::Dark } else { PlatformThemeVariant::Light };
        let contrast_preference =
            if inputs.high_contrast { ColorContrastPreference::High } else { ColorContrastPreference::NoPreference };

        let values =
            PlatformColorValues::new().with_theme_variant(system_theme).with_contrast_preference(contrast_preference);
        match inputs.accents {
            AccentColors::System(accent1, accent2, accent3) => values
                .with_accent_color1(Color::from_uint32(accent1))
                .with_accent_color2(Color::from_uint32(accent2))
                .with_accent_color3(Color::from_uint32(accent3)),
            AccentColors::Theme(accent) => values.with_accent_color1(Color::from_uint32(accent)),
            AccentColors::None => values,
        }
    }

    fn update_input_config_values(&self, context: &dyn ISettingsContext) {
        let config = context.input_config();
        self.hold_wait_duration.set(Duration::from_millis(u64::try_from(config.long_press_timeout).unwrap_or(0)));

        if let Some(multi_press_timeout) = config.multi_press_timeout {
            self.double_tap_time.set(Duration::from_millis(u64::try_from(multi_press_timeout).unwrap_or(0)));
        }
        let scaling = config.density;
        if let Some((scaled_double_tap_slop, scaled_touch_slop)) = config.slops {
            // The arithmetic of the reference: an integer doubled, divided by the density
            // as a single precision number.
            let size = f64::from((scaled_double_tap_slop * 2) as f32 / scaling);
            self.double_tap_size.set(Size::new(size, size));
            let size = f64::from((scaled_touch_slop * 2) as f32 / scaling);
            self.tap_size.set(Size::new(size, size));
        }
    }

    fn query_preferred_application_language(context: Option<&dyn ISettingsContext>) -> Option<String> {
        context?.preferred_application_language().filter(|tag| !tag.is_empty())
    }

    /// The receiver of the configuration changes was called.
    pub(crate) fn on_receive() {
        let Some(settings) = INSTANCE.with(|instance| instance.borrow().clone()) else {
            return;
        };
        settings.schedule_update();
    }

    #[cfg(target_os = "android")]
    fn schedule_update(self: &Rc<Self>) {
        use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};

        // The context might still have the old values at this point because they haven't been processed yet.
        // Postpone the update 100ms arbitrarily. Not an ideal solution, but sufficient.
        let settings = self.clone();
        DispatcherTimer::run_once(move || settings.update(), Duration::from_millis(100), DispatcherPriority::NORMAL);
    }

    #[cfg(not(target_os = "android"))]
    fn schedule_update(self: &Rc<Self>) {
        // No dispatcher of the platform on another system: the tests call `update`.
        self.update();
    }

    /// Reads the context again and raises what changed.
    pub(crate) fn update(&self) {
        if let Some(context) = &self.context {
            self.update_input_config_values(context.as_ref());
            self.update_color_values(context.as_ref());
            self.update_preferred_application_language(context.as_ref());
        }
    }
}

impl IPlatformSettings for AndroidPlatformSettings {
    fn get_tap_size(&self, type_: PointerType) -> Size {
        if type_ == PointerType::Mouse {
            self.base.get_tap_size(type_)
        } else {
            self.tap_size.get()
        }
    }

    fn get_double_tap_size(&self, type_: PointerType) -> Size {
        if type_ == PointerType::Mouse {
            self.base.get_double_tap_size(type_)
        } else {
            self.double_tap_size.get()
        }
    }

    fn get_double_tap_time(&self, type_: PointerType) -> Duration {
        if type_ == PointerType::Mouse {
            self.base.get_double_tap_time(type_)
        } else {
            self.double_tap_time.get()
        }
    }

    fn hold_wait_duration(&self) -> Duration {
        self.hold_wait_duration.get()
    }

    fn hotkey_configuration(&self) -> Rc<PlatformHotkeyConfiguration> {
        self.base.hotkey_configuration()
    }

    fn preferred_application_language(&self) -> String {
        match &*self.latest_language.borrow() {
            Some(language) => language.clone(),
            None => self.base.preferred_application_language(),
        }
    }

    fn get_color_values(&self) -> PlatformColorValues {
        self.color_values.get()
    }

    fn color_values_changed(&self, handler: Rc<dyn Fn(&PlatformColorValues)>) -> Rc<dyn IDisposable> {
        self.base.color_values_changed(handler)
    }

    fn preferred_application_language_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        self.base.preferred_application_language_changed(handler)
    }
}

/// The application context of the system.
#[cfg(target_os = "android")]
pub(crate) struct ApplicationSettingsContext {
    context: crate::interop::java::JavaObject,
}

#[cfg(target_os = "android")]
impl ApplicationSettingsContext {
    pub fn new(context: crate::interop::java::JavaObject) -> Self {
        Self { context }
    }

    fn helper() -> crate::interop::java::JavaClass {
        crate::interop::java::JavaClass::find(crate::interop::natives::PLATFORM_HELPER)
    }
}

#[cfg(target_os = "android")]
impl ISettingsContext for ApplicationSettingsContext {
    fn color_inputs(&self) -> ColorInputs {
        use crate::interop::java::{call_static_object, int_array_of, JavaValue};

        let values = call_static_object(
            &Self::helper(),
            "getColorValues",
            "(Landroid/content/Context;)[I",
            &[JavaValue::Object(Some(&self.context))],
        )
        .map(|values| int_array_of(&values))
        .unwrap_or_default();
        color_inputs_from_values(&values)
    }

    fn input_config(&self) -> InputConfig {
        use crate::interop::java::{call_static_object, float_array_of, JavaValue};

        let values = call_static_object(
            &Self::helper(),
            "getInputConfigValues",
            "(Landroid/content/Context;)[F",
            &[JavaValue::Object(Some(&self.context))],
        )
        .map(|values| float_array_of(&values))
        .unwrap_or_default();
        input_config_from_values(&values)
    }

    fn preferred_application_language(&self) -> Option<String> {
        use crate::interop::java::{call_static_object, string_of, JavaValue};

        call_static_object(
            &Self::helper(),
            "getPreferredApplicationLanguage",
            "(Landroid/content/Context;)Ljava/lang/String;",
            &[JavaValue::Object(Some(&self.context))],
        )
        .map(|tag| string_of(&tag))
    }

    fn register_configuration_changed_receiver(&self) {
        use crate::interop::java::{call_static_void, JavaClass, JavaValue};

        call_static_void(
            &JavaClass::find(crate::interop::natives::CONFIGURATION_CHANGED_RECEIVER),
            "register",
            "(Landroid/content/Context;)V",
            &[JavaValue::Object(Some(&self.context))],
        );
    }
}

/// Reads what `PlatformHelper.getColorValues` answers: whether the night
/// mode is on, whether the high contrast text is on, the number of accent
/// colours (three of the system, one of the theme, or none) and the
/// colours.
pub(crate) fn color_inputs_from_values(values: &[i32]) -> ColorInputs {
    let at = |index: usize| values.get(index).copied().unwrap_or(0);
    let color = |index: usize| at(index) as u32;
    ColorInputs {
        night: at(0) != 0,
        high_contrast: at(1) != 0,
        accents: match at(2) {
            3 => AccentColors::System(color(3), color(4), color(5)),
            1 => AccentColors::Theme(color(3)),
            _ => AccentColors::None,
        },
    }
}

/// Reads what `PlatformHelper.getInputConfigValues` answers: the long press
/// timeout, the multi press timeout or a negative number, whether the
/// context has a view configuration, its scaled double tap slop and scaled
/// touch slop, and the density of the display metrics.
pub(crate) fn input_config_from_values(values: &[f32]) -> InputConfig {
    let at = |index: usize| values.get(index).copied().unwrap_or(0.0);
    InputConfig {
        long_press_timeout: at(0) as i32,
        multi_press_timeout: (at(1) >= 0.0).then_some(at(1) as i32),
        slops: (at(2) != 0.0).then_some((at(3) as i32, at(4) as i32)),
        density: if values.len() > 5 { at(5) } else { 1.0 },
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of the settings.
    use super::*;

    struct TestContext {
        colors: Rc<Cell<ColorInputs>>,
        input: Rc<Cell<InputConfig>>,
        language: Rc<RefCell<Option<String>>>,
        registered: Rc<Cell<u32>>,
    }

    struct Fixture {
        colors: Rc<Cell<ColorInputs>>,
        input: Rc<Cell<InputConfig>>,
        language: Rc<RefCell<Option<String>>>,
        registered: Rc<Cell<u32>>,
        settings: Rc<AndroidPlatformSettings>,
    }

    impl ISettingsContext for TestContext {
        fn color_inputs(&self) -> ColorInputs {
            self.colors.get()
        }

        fn input_config(&self) -> InputConfig {
            self.input.get()
        }

        fn preferred_application_language(&self) -> Option<String> {
            self.language.borrow().clone()
        }

        fn register_configuration_changed_receiver(&self) {
            self.registered.set(self.registered.get() + 1);
        }
    }

    fn fixture() -> Fixture {
        let colors = Rc::new(Cell::new(ColorInputs {
            night: false,
            high_contrast: false,
            accents: AccentColors::System(0xff11_2233, 0xff44_5566, 0xff77_8899),
        }));
        let input = Rc::new(Cell::new(InputConfig {
            long_press_timeout: 400,
            multi_press_timeout: Some(300),
            slops: Some((262, 21)),
            density: 2.625,
        }));
        let language = Rc::new(RefCell::new(Some("pl-PL".to_string())));
        let registered = Rc::new(Cell::new(0));
        let settings = AndroidPlatformSettings::new(Some(Box::new(TestContext {
            colors: colors.clone(),
            input: input.clone(),
            language: language.clone(),
            registered: registered.clone(),
        })));
        Fixture { colors, input, language, registered, settings }
    }

    #[test]
    fn without_a_context_the_settings_are_the_defaults() {
        let settings = AndroidPlatformSettings::new(None);
        let base = DefaultPlatformSettings::new();
        assert_eq!(settings.get_color_values(), base.get_color_values());
        assert_eq!(settings.hold_wait_duration(), Duration::from_millis(300));
        assert_eq!(settings.get_double_tap_time(PointerType::Touch), Duration::from_millis(500));
        assert_eq!(settings.get_double_tap_size(PointerType::Touch), Size::new(16.0, 16.0));
        assert_eq!(settings.get_tap_size(PointerType::Pen), Size::new(10.0, 10.0));
        assert_eq!(settings.preferred_application_language(), base.preferred_application_language());
    }

    #[test]
    fn the_values_of_the_context_are_read_and_the_receiver_is_registered() {
        let f = fixture();
        assert_eq!(f.registered.get(), 1);

        let colors = f.settings.get_color_values();
        assert_eq!(colors.theme_variant(), PlatformThemeVariant::Light);
        assert_eq!(colors.contrast_preference(), ColorContrastPreference::NoPreference);
        assert_eq!(colors.accent_color1(), Color::from_argb(0xff, 0x11, 0x22, 0x33));
        assert_eq!(colors.accent_color2(), Color::from_argb(0xff, 0x44, 0x55, 0x66));
        assert_eq!(colors.accent_color3(), Color::from_argb(0xff, 0x77, 0x88, 0x99));

        assert_eq!(f.settings.hold_wait_duration(), Duration::from_millis(400));
        assert_eq!(f.settings.get_double_tap_time(PointerType::Touch), Duration::from_millis(300));
        let double_tap = f64::from(524.0f32 / 2.625f32);
        assert_eq!(f.settings.get_double_tap_size(PointerType::Touch), Size::new(double_tap, double_tap));
        let tap = f64::from(42.0f32 / 2.625f32);
        assert_eq!(f.settings.get_tap_size(PointerType::Touch), Size::new(tap, tap));
        assert_eq!(f.settings.preferred_application_language(), "pl-PL");
    }

    #[test]
    fn a_mouse_has_the_values_of_the_framework() {
        let f = fixture();
        let base = DefaultPlatformSettings::new();
        assert_eq!(f.settings.get_tap_size(PointerType::Mouse), base.get_tap_size(PointerType::Mouse));
        assert_eq!(f.settings.get_double_tap_size(PointerType::Mouse), base.get_double_tap_size(PointerType::Mouse));
        assert_eq!(f.settings.get_double_tap_time(PointerType::Mouse), base.get_double_tap_time(PointerType::Mouse));
    }

    #[test]
    fn a_configuration_change_raises_what_changed_and_only_that() {
        let f = fixture();
        let color_changes = Rc::new(RefCell::new(Vec::new()));
        let language_changes = Rc::new(Cell::new(0));
        let _colors = f.settings.color_values_changed(Rc::new({
            let color_changes = color_changes.clone();
            move |values: &PlatformColorValues| color_changes.borrow_mut().push(*values)
        }));
        let _language = f.settings.preferred_application_language_changed(Rc::new({
            let language_changes = language_changes.clone();
            move || language_changes.set(language_changes.get() + 1)
        }));

        // Nothing changed.
        AndroidPlatformSettings::on_receive();
        assert!(color_changes.borrow().is_empty());
        assert_eq!(language_changes.get(), 0);

        // Night mode and high contrast.
        f.colors.set(ColorInputs { night: true, high_contrast: true, accents: AccentColors::Theme(0xff00_ff00) });
        AndroidPlatformSettings::on_receive();
        assert_eq!(color_changes.borrow().len(), 1);
        let colors = f.settings.get_color_values();
        assert_eq!(colors, color_changes.borrow()[0]);
        assert_eq!(colors.theme_variant(), PlatformThemeVariant::Dark);
        assert_eq!(colors.contrast_preference(), ColorContrastPreference::High);
        assert_eq!(colors.accent_color1(), Color::from_argb(0xff, 0, 0xff, 0));
        // Without a second and a third accent they follow the first.
        assert_eq!(colors.accent_color2(), colors.accent_color1());
        assert_eq!(language_changes.get(), 0);

        // The language, and the density (a display change).
        *f.language.borrow_mut() = Some("de-DE".to_string());
        f.input.set(InputConfig { long_press_timeout: 500, multi_press_timeout: None, slops: None, density: 1.0 });
        AndroidPlatformSettings::on_receive();
        assert_eq!(language_changes.get(), 1);
        assert_eq!(f.settings.preferred_application_language(), "de-DE");
        assert_eq!(f.settings.hold_wait_duration(), Duration::from_millis(500));
        // No multi press timeout below API 31 and no view configuration: the values stay.
        assert_eq!(f.settings.get_double_tap_time(PointerType::Touch), Duration::from_millis(300));
        assert_eq!(color_changes.borrow().len(), 1);

        // An empty language tag is no language: the language of the framework.
        *f.language.borrow_mut() = Some(String::new());
        AndroidPlatformSettings::on_receive();
        assert_eq!(language_changes.get(), 2);
        assert_eq!(
            f.settings.preferred_application_language(),
            DefaultPlatformSettings::new().preferred_application_language()
        );
    }

    #[test]
    fn the_answers_of_the_java_layer_are_read() {
        assert_eq!(
            color_inputs_from_values(&[1, 0, 3, 0xff11_2233u32 as i32, 0xff44_5566u32 as i32, 0xff77_8899u32 as i32]),
            ColorInputs {
                night: true,
                high_contrast: false,
                accents: AccentColors::System(0xff11_2233, 0xff44_5566, 0xff77_8899),
            }
        );
        assert_eq!(
            color_inputs_from_values(&[0, 1, 1, 0xff00_ff00u32 as i32, 0, 0]),
            ColorInputs { night: false, high_contrast: true, accents: AccentColors::Theme(0xff00_ff00) }
        );
        assert_eq!(
            color_inputs_from_values(&[]),
            ColorInputs { night: false, high_contrast: false, accents: AccentColors::None }
        );

        assert_eq!(
            input_config_from_values(&[400.0, 300.0, 1.0, 262.0, 21.0, 2.625]),
            InputConfig { long_press_timeout: 400, multi_press_timeout: Some(300), slops: Some((262, 21)), density: 2.625 }
        );
        assert_eq!(
            input_config_from_values(&[400.0, -1.0, 0.0, 0.0, 0.0, 1.5]),
            InputConfig { long_press_timeout: 400, multi_press_timeout: None, slops: None, density: 1.5 }
        );
    }
}
