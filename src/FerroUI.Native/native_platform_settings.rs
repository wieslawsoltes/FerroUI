use crate::frn_string::frn_string_to_string;
use crate::helpers::ComResultExt;
use crate::interop::*;
use ferroui_base::input::platform::PlatformHotkeyConfiguration;
use ferroui_base::input::PointerType;
use ferroui_base::media::Color;
use ferroui_base::platform::{
    ColorContrastPreference, DefaultPlatformSettings, IPlatformSettings, PlatformColorValues, PlatformThemeVariant,
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

/// The platform settings of the macOS backend.
pub struct NativePlatformSettings {
    base: DefaultPlatformSettings,
    platform_settings: ComPtr<IFrnPlatformSettings>,
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
        let this = Rc::new(NativePlatformSettings {
            base: DefaultPlatformSettings::new(),
            platform_settings,
            color_values: Cell::new(None),
            last_language: RefCell::new(None),
        });
        let colors_change = IFrnActionCallback::from_impl(ColorsChangeCallback { settings: Rc::downgrade(&this) });
        this.platform_settings.register_colors_change(Some(&colors_change));
        let language_change =
            IFrnActionCallback::from_impl(LanguageChangeCallback { settings: Rc::downgrade(&this) });
        this.platform_settings.register_language_change(Some(&language_change));
        this
    }

    fn get_uncached_color_values(&self) -> PlatformColorValues {
        to_color_values(self.platform_settings.get_platform_theme(), self.platform_settings.get_accent_color())
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
    }
}
