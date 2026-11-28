use crate::frn_string::frn_string_to_string;
use crate::helpers::ComResultExt;
use crate::interop::*;
use ferroui_base::input::platform::PlatformHotkeyConfiguration;
use ferroui_base::input::PointerType;
use ferroui_base::media::Color;
use ferroui_base::platform::{
    ColorContrastPreference, DefaultPlatformSettings, IPlatformSettings, PlatformColorValues, PlatformThemeVariant,
    SystemColors,
};
use ferroui_base::reactive::IDisposable;
use ferroui_base::Size;
use ferroui_microcom::ComPtr;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::time::Duration;

/// Maps the native theme to the theme variant and contrast preference;
/// `None` for a value the toolkit does not know.
fn to_theme(theme: FrnPlatformThemeVariant) -> Option<(PlatformThemeVariant, ColorContrastPreference)> {
    if theme == FrnPlatformThemeVariant::Dark {
        Some((PlatformThemeVariant::Dark, ColorContrastPreference::NoPreference))
    } else if theme == FrnPlatformThemeVariant::Light {
        Some((PlatformThemeVariant::Light, ColorContrastPreference::NoPreference))
    } else if theme == FrnPlatformThemeVariant::HighContrastDark {
        Some((PlatformThemeVariant::Dark, ColorContrastPreference::High))
    } else if theme == FrnPlatformThemeVariant::HighContrastLight {
        Some((PlatformThemeVariant::Light, ColorContrastPreference::High))
    } else {
        None
    }
}

fn to_color_values(theme: FrnPlatformThemeVariant, accent_color: u32) -> PlatformColorValues {
    let Some((theme_variant, contrast_preference)) = to_theme(theme) else {
        panic!("Specified argument was out of the range of valid values: {theme:?}");
    };
    let color_values =
        PlatformColorValues::new().with_theme_variant(theme_variant).with_contrast_preference(contrast_preference);

    if accent_color > 0 {
        color_values.with_accent_color1(Color::from_uint32(accent_color))
    } else {
        color_values
    }
}

/// The system colours of the native contrast extension (#560): the colours of the indices of
/// `IFrnContrastSettings::GetSystemColor` (`windowBackgroundColor`, `labelColor`, `controlColor`,
/// `controlTextColor`, `selectedContentBackgroundColor`, `alternateSelectedControlTextColor`,
/// `linkColor`, `disabledControlTextColor`); `None` when the library answers none of an index.
fn to_system_colors(color: impl Fn(i32) -> u32) -> Option<SystemColors> {
    let mut colors = [Color::default(); 8];
    for (index, slot) in colors.iter_mut().enumerate() {
        let argb = color(index as i32);
        if argb == 0 {
            return None;
        }
        *slot = Color::from_uint32(argb);
    }
    let [window, window_text, button_face, button_text, highlight, highlight_text, hotlight, gray_text] = colors;
    Some(
        SystemColors::new(window, window_text)
            .with_button_face(button_face)
            .with_button_text(button_text)
            .with_highlight(highlight)
            .with_highlight_text(highlight_text)
            .with_hotlight(hotlight)
            .with_gray_text(gray_text),
    )
}

/// The platform settings of the macOS backend.
pub struct NativePlatformSettings {
    base: DefaultPlatformSettings,
    platform_settings: ComPtr<IFrnPlatformSettings>,
    accessibility_settings: Option<ComPtr<IFrnAccessibilitySettings>>,
    /// The optional power extension of the native library: the low power mode (the energy
    /// saver of the material policy, docs/porting/winui-materials.md).
    power_settings: Option<ComPtr<IFrnPowerSettings>>,
    /// The optional contrast extension of the native library: "Increase contrast" and the
    /// system colours (#560).
    contrast_settings: Option<ComPtr<IFrnContrastSettings>>,
    color_values: Cell<Option<PlatformColorValues>>,
    last_language: RefCell<Option<String>>,
}

struct ColorsChangeCallback {
    settings: Weak<NativePlatformSettings>,
}

impl IFrnActionCallbackImpl for ColorsChangeCallback {
    fn run(&self) {
        crate::callback_base::guard((), || {
            if let Some(settings) = self.settings.upgrade() {
                settings.on_color_values_changed();
            }
        })
    }
}

struct LanguageChangeCallback {
    settings: Weak<NativePlatformSettings>,
}

struct AccessibilityChangeCallback {
    settings: Weak<NativePlatformSettings>,
}

struct PowerChangeCallback {
    settings: Weak<NativePlatformSettings>,
}

impl IFrnActionCallbackImpl for PowerChangeCallback {
    fn run(&self) {
        crate::callback_base::guard((), || {
            if let Some(settings) = self.settings.upgrade() {
                settings.base.on_energy_saver_changed();
            }
        })
    }
}

impl IFrnActionCallbackImpl for AccessibilityChangeCallback {
    fn run(&self) {
        crate::callback_base::guard((), || {
            if let Some(settings) = self.settings.upgrade() {
                // Increase Contrast can change the appearance together with
                // motion/transparency, even when the color callback is absent.
                settings.on_color_values_changed();
                settings.base.on_accessibility_settings_changed();
            }
        })
    }
}

impl IFrnActionCallbackImpl for LanguageChangeCallback {
    fn run(&self) {
        crate::callback_base::guard((), || {
            if let Some(settings) = self.settings.upgrade() {
                settings.on_preferred_language_changed();
            }
        })
    }
}

impl NativePlatformSettings {
    pub fn new(platform_settings: ComPtr<IFrnPlatformSettings>) -> Rc<NativePlatformSettings> {
        let accessibility_settings = platform_settings.cast::<IFrnAccessibilitySettings>().ok();
        let power_settings = platform_settings.cast::<IFrnPowerSettings>().ok();
        let contrast_settings = platform_settings.cast::<IFrnContrastSettings>().ok();
        let this = Rc::new(NativePlatformSettings {
            base: DefaultPlatformSettings::new(),
            platform_settings,
            accessibility_settings,
            power_settings,
            contrast_settings,
            color_values: Cell::new(None),
            last_language: RefCell::new(None),
        });
        let colors_change = IFrnActionCallback::from_impl(ColorsChangeCallback { settings: Rc::downgrade(&this) });
        this.platform_settings.register_colors_change(Some(&colors_change));
        let language_change =
            IFrnActionCallback::from_impl(LanguageChangeCallback { settings: Rc::downgrade(&this) });
        this.platform_settings.register_language_change(Some(&language_change));
        if let Some(settings) = &this.accessibility_settings {
            let callback =
                IFrnActionCallback::from_impl(AccessibilityChangeCallback { settings: Rc::downgrade(&this) });
            settings.register_accessibility_change(Some(&callback));
        }
        if let Some(settings) = &this.power_settings {
            let callback = IFrnActionCallback::from_impl(PowerChangeCallback { settings: Rc::downgrade(&this) });
            settings.register_power_change(Some(&callback));
        }
        this
    }

    /// The colour values of the system. High contrast is the high contrast appearances or
    /// "Increase contrast" of the accessibility display options (#560, decision D11 of
    /// docs/porting/winui-calculator-gaps.md); while it is on, the system colours under the
    /// effective appearance come with it.
    fn get_uncached_color_values(&self) -> PlatformColorValues {
        let values =
            to_color_values(self.platform_settings.get_platform_theme(), self.platform_settings.get_accent_color());
        let Some(contrast) = &self.contrast_settings else {
            return values;
        };
        let values = if contrast.get_increase_contrast() {
            values.with_contrast_preference(ColorContrastPreference::High)
        } else {
            values
        };
        if values.is_high_contrast() {
            values.with_system_colors(to_system_colors(|index| contrast.get_system_color(index)))
        } else {
            values
        }
    }

    pub fn on_color_values_changed(&self) {
        let old_color_values = self.color_values.get();
        let color_values = self.get_uncached_color_values();

        if old_color_values != Some(color_values) {
            self.color_values.set(Some(color_values));
            self.base.on_color_values_changed(color_values);
        }
    }

    fn on_preferred_language_changed(&self) {
        let old_language = self.last_language.borrow_mut().take();

        if old_language != Some(self.preferred_application_language()) {
            self.base.on_preferred_application_language_changed();
        }
    }

    fn query_preferred_application_language(&self) -> String {
        let language = self.platform_settings.get_preferred_language().check();
        match language.and_then(|language| frn_string_to_string(&language)) {
            Some(value) if !value.is_empty() => value,
            _ => self.base.preferred_application_language(),
        }
    }
}

impl Drop for NativePlatformSettings {
    fn drop(&mut self) {
        if let Some(settings) = &self.accessibility_settings {
            settings.register_accessibility_change(None);
        }
        if let Some(settings) = &self.power_settings {
            settings.register_power_change(None);
        }
    }
}

impl IPlatformSettings for NativePlatformSettings {
    fn get_tap_size(&self, type_: PointerType) -> Size {
        self.base.get_tap_size(type_)
    }

    fn get_double_tap_size(&self, type_: PointerType) -> Size {
        self.base.get_double_tap_size(type_)
    }

    fn get_double_tap_time(&self, type_: PointerType) -> Duration {
        self.base.get_double_tap_time(type_)
    }

    fn hold_wait_duration(&self) -> Duration {
        self.base.hold_wait_duration()
    }

    fn hotkey_configuration(&self) -> Rc<PlatformHotkeyConfiguration> {
        self.base.hotkey_configuration()
    }

    fn animations_enabled(&self) -> bool {
        self.accessibility_settings.as_ref().is_none_or(|settings| !settings.get_reduce_motion())
    }

    fn transparency_enabled(&self) -> bool {
        self.accessibility_settings.as_ref().is_none_or(|settings| !settings.get_reduce_transparency())
    }

    fn accessibility_settings_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        self.base.accessibility_settings_changed(handler)
    }

    fn energy_saver_enabled(&self) -> bool {
        self.power_settings.as_ref().is_some_and(|settings| settings.get_low_power_mode())
    }

    fn energy_saver_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        self.base.energy_saver_changed(handler)
    }

    fn preferred_application_language(&self) -> String {
        if let Some(language) = self.last_language.borrow().clone() {
            return language;
        }
        let language = self.query_preferred_application_language();
        *self.last_language.borrow_mut() = Some(language.clone());
        language
    }

    fn get_color_values(&self) -> PlatformColorValues {
        match self.color_values.get() {
            Some(color_values) => color_values,
            None => {
                let color_values = self.get_uncached_color_values();
                self.color_values.set(Some(color_values));
                color_values
            }
        }
    }

    fn color_values_changed(&self, handler: Rc<dyn Fn(&PlatformColorValues)>) -> Rc<dyn IDisposable> {
        self.base.color_values_changed(handler)
    }

    fn preferred_application_language_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        self.base.preferred_application_language_changed(handler)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_microcom::{make_com_with, HResult, InterfaceEntry};

    #[derive(Default)]
    struct AccessibilityState {
        reduce_motion: Cell<bool>,
        reduce_transparency: Cell<bool>,
        callback: RefCell<Option<ComPtr<IFrnActionCallback>>>,
        low_power_mode: Cell<bool>,
        power_callback: RefCell<Option<ComPtr<IFrnActionCallback>>>,
        increase_contrast: Cell<bool>,
        system_colors: Cell<[u32; 8]>,
    }

    struct FakePlatformSettings {
        accessibility: Rc<AccessibilityState>,
    }

    impl IFrnPlatformSettingsImpl for FakePlatformSettings {
        fn get_platform_theme(&self) -> FrnPlatformThemeVariant {
            FrnPlatformThemeVariant::Light
        }

        fn get_accent_color(&self) -> u32 {
            0xFF007AFF
        }

        fn register_colors_change(&self, _callback: Option<&IFrnActionCallback>) {}

        fn get_preferred_language(&self) -> Result<Option<ComPtr<IFrnString>>, HResult> {
            Ok(None)
        }

        fn register_language_change(&self, _callback: Option<&IFrnActionCallback>) {}
    }

    impl IFrnPowerSettingsImpl for FakePlatformSettings {
        fn get_low_power_mode(&self) -> bool {
            self.accessibility.low_power_mode.get()
        }

        fn register_power_change(&self, callback: Option<&IFrnActionCallback>) {
            *self.accessibility.power_callback.borrow_mut() = callback.map(ComPtr::from_ref);
        }
    }

    impl IFrnContrastSettingsImpl for FakePlatformSettings {
        fn get_increase_contrast(&self) -> bool {
            self.accessibility.increase_contrast.get()
        }

        fn get_system_color(&self, index: i32) -> u32 {
            let colors = self.accessibility.system_colors.get();
            usize::try_from(index).ok().and_then(|index| colors.get(index).copied()).unwrap_or(0)
        }
    }

    impl IFrnAccessibilitySettingsImpl for FakePlatformSettings {
        fn get_reduce_motion(&self) -> bool {
            self.accessibility.reduce_motion.get()
        }

        fn get_reduce_transparency(&self) -> bool {
            self.accessibility.reduce_transparency.get()
        }

        fn register_accessibility_change(&self, callback: Option<&IFrnActionCallback>) {
            *self.accessibility.callback.borrow_mut() = callback.map(ComPtr::from_ref);
        }
    }

    #[test]
    fn old_native_settings_without_the_extension_preserve_default_behavior() {
        let state = Rc::new(AccessibilityState::default());
        state.reduce_motion.set(true);
        state.reduce_transparency.set(true);
        // The implementation is exposed through only the original interface,
        // exactly as an older native library answers QueryInterface.
        let native = IFrnPlatformSettings::from_impl(FakePlatformSettings { accessibility: state.clone() });
        let settings = NativePlatformSettings::new(native);
        assert!(settings.accessibility_settings.is_none());
        assert!(settings.power_settings.is_none());
        assert!(!settings.energy_saver_enabled());
        assert!(state.power_callback.borrow().is_none());
        assert!(settings.animations_enabled());
        assert!(settings.transparency_enabled());
        assert!(state.callback.borrow().is_none());
    }

    #[test]
    fn accessibility_extension_reads_changes_and_releases_its_subscription() {
        let state = Rc::new(AccessibilityState::default());
        let native = make_com_with::<IFrnPlatformSettings, _, 1>(
            FakePlatformSettings { accessibility: state.clone() },
            [InterfaceEntry::of::<IFrnAccessibilitySettings>()],
        );
        let settings = NativePlatformSettings::new(native);
        assert!(settings.animations_enabled());
        assert!(settings.transparency_enabled());
        let seen = Rc::new(RefCell::new(Vec::new()));
        let subscription = {
            let seen = seen.clone();
            let settings_weak = Rc::downgrade(&settings);
            settings.accessibility_settings_changed(Rc::new(move || {
                if let Some(settings) = settings_weak.upgrade() {
                    seen.borrow_mut().push((settings.animations_enabled(), settings.transparency_enabled()));
                }
            }))
        };
        let callback = state.callback.borrow().clone().expect("native notification callback");
        state.reduce_motion.set(true);
        callback.run();
        state.reduce_motion.set(false);
        state.reduce_transparency.set(true);
        callback.run();
        assert_eq!(*seen.borrow(), [(false, true), (true, false)]);

        subscription.dispose();
        callback.run();
        assert_eq!(seen.borrow().len(), 2);
        drop(settings);
        assert!(state.callback.borrow().is_none(), "drop unregisters the native notification observer");
        // A notification already queued by the platform holds only a weak
        // reference to the disposed Rust adapter.
        callback.run();
        assert_eq!(seen.borrow().len(), 2);
    }

    #[test]
    fn power_extension_reports_the_low_power_mode_and_releases_its_subscription() {
        let state = Rc::new(AccessibilityState::default());
        let native = make_com_with::<IFrnPlatformSettings, _, 2>(
            FakePlatformSettings { accessibility: state.clone() },
            [InterfaceEntry::of::<IFrnAccessibilitySettings>(), InterfaceEntry::of::<IFrnPowerSettings>()],
        );
        let settings = NativePlatformSettings::new(native);
        assert!(!settings.energy_saver_enabled());
        let seen = Rc::new(RefCell::new(Vec::new()));
        let subscription = {
            let seen = seen.clone();
            let settings_weak = Rc::downgrade(&settings);
            settings.energy_saver_changed(Rc::new(move || {
                if let Some(settings) = settings_weak.upgrade() {
                    seen.borrow_mut().push(settings.energy_saver_enabled());
                }
            }))
        };
        let callback = state.power_callback.borrow().clone().expect("native power callback");
        state.low_power_mode.set(true);
        callback.run();
        state.low_power_mode.set(false);
        callback.run();
        assert_eq!(*seen.borrow(), [true, false]);

        subscription.dispose();
        callback.run();
        assert_eq!(seen.borrow().len(), 2);
        drop(settings);
        assert!(state.power_callback.borrow().is_none(), "drop unregisters the native power observer");
    }

    #[test]
    fn increase_contrast_is_high_contrast_with_the_system_colors_and_is_raised_on_change() {
        let state = Rc::new(AccessibilityState::default());
        let scheme = [
            0xFFFFFFFF, 0xFF000000, 0xFFFFFFFF, 0xFF000000, 0xFF0040DD, 0xFFFFFFFF, 0xFF0000EE, 0xFF5E5E5E,
        ];
        state.system_colors.set(scheme);
        let native = make_com_with::<IFrnPlatformSettings, _, 2>(
            FakePlatformSettings { accessibility: state.clone() },
            [InterfaceEntry::of::<IFrnAccessibilitySettings>(), InterfaceEntry::of::<IFrnContrastSettings>()],
        );
        let settings = NativePlatformSettings::new(native);
        // Off: no preference and no system colours.
        assert!(!settings.get_color_values().is_high_contrast());
        assert_eq!(settings.get_color_values().system_colors(), None);

        let seen = Rc::new(RefCell::new(Vec::new()));
        let _subscription = {
            let seen = seen.clone();
            settings.color_values_changed(Rc::new(move |values: &PlatformColorValues| seen.borrow_mut().push(*values)))
        };
        // "Increase contrast" turned on: the accessibility notification raises the colour values.
        state.increase_contrast.set(true);
        state.callback.borrow().clone().expect("native notification callback").run();
        let values = settings.get_color_values();
        assert!(values.is_high_contrast());
        assert_eq!(values.theme_variant(), PlatformThemeVariant::Light);
        let colors = values.system_colors().expect("the system colours");
        assert_eq!(colors.window(), Color::from_uint32(0xFFFFFFFF));
        assert_eq!(colors.window_text(), Color::from_uint32(0xFF000000));
        assert_eq!(colors.highlight(), Color::from_uint32(0xFF0040DD));
        assert_eq!(colors.hotlight(), Color::from_uint32(0xFF0000EE));
        assert_eq!(colors.gray_text(), Color::from_uint32(0xFF5E5E5E));
        assert_eq!(*seen.borrow(), [values]);

        // A colour the library does not answer: no system colours (the theme's schemes apply).
        assert_eq!(to_system_colors(|index| if index == 7 { 0 } else { 0xFF000000 }), None);

        state.increase_contrast.set(false);
        state.callback.borrow().clone().expect("native notification callback").run();
        assert!(!settings.get_color_values().is_high_contrast());
        assert_eq!(seen.borrow().len(), 2);
    }

    #[test]
    fn themes() {
        use ColorContrastPreference::*;
        use PlatformThemeVariant::*;
        assert_eq!(to_theme(FrnPlatformThemeVariant::Light), Some((Light, NoPreference)));
        assert_eq!(to_theme(FrnPlatformThemeVariant::Dark), Some((Dark, NoPreference)));
        assert_eq!(to_theme(FrnPlatformThemeVariant::HighContrastLight), Some((Light, High)));
        assert_eq!(to_theme(FrnPlatformThemeVariant::HighContrastDark), Some((Dark, High)));
        assert_eq!(to_theme(FrnPlatformThemeVariant(9)), None);
    }

    #[test]
    fn accent_color_is_only_taken_when_the_system_reports_one() {
        let values = to_color_values(FrnPlatformThemeVariant::HighContrastDark, 0xFF336699);
        assert_eq!(values.theme_variant(), PlatformThemeVariant::Dark);
        assert_eq!(values.contrast_preference(), ColorContrastPreference::High);
        assert_eq!(values.accent_color1(), Color::from_uint32(0xFF336699));

        let values = to_color_values(FrnPlatformThemeVariant::Light, 0);
        assert_eq!(values, PlatformColorValues::new());
    }

    /// The settings object over the real native settings: none of this
    /// needs a window server.
    #[test]
    fn native_settings_report_the_system_values() {
        let factory = create_ferro_native().expect("factory");
        let Ok(Some(native)) = factory.create_platform_settings() else {
            eprintln!("skipped: the native platform settings are not available");
            return;
        };
        let settings = NativePlatformSettings::new(native);

        // Cached: asking twice gives the same values.
        assert_eq!(settings.get_color_values(), settings.get_color_values());
        assert_eq!(settings.preferred_application_language(), settings.preferred_application_language());
        assert_eq!(settings.get_double_tap_time(PointerType::Mouse), Duration::from_millis(500));
        let accessibility = settings.accessibility_settings.as_ref().expect("current native accessibility interface");
        assert_eq!(settings.animations_enabled(), !accessibility.get_reduce_motion());
        assert_eq!(settings.transparency_enabled(), !accessibility.get_reduce_transparency());
    }
}
