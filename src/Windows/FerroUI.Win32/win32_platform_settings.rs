//! The settings of the system the toolkit reads: the sizes and times of
//! mouse gestures, the colours (the theme variant, the accent colour and its
//! shades, high contrast) and the preferred language of the user.

use ferroui_base::media::Color;
use ferroui_base::platform::{AccentColorShades, ColorContrastPreference, PlatformColorValues, PlatformThemeVariant};

/// The shades of the accent colour from the colours `UISettings` reports
/// for `UIColorType` `AccentDark3` to `AccentLight3`, in the order of the
/// enumeration (FerroUI addition: the reference reads only the accent).
pub(crate) fn accent_shades_from_ui_colors(colors: [Color; 6]) -> AccentColorShades {
    let [dark3, dark2, dark1, light1, light2, light3] = colors;
    AccentColorShades { dark1, dark2, dark3, light1, light2, light3 }
}

/// The colour values of a system in a high contrast mode, from the name of
/// its high contrast scheme and its accent colour.
pub(crate) fn high_contrast_color_values(high_contrast_scheme: Option<&str>, accent: Color) -> PlatformColorValues {
    // Windows 11 has 4 different high contrast schemes:
    // - Aquatic - High Contrast Black
    // - Desert - High Contrast White
    // - Dusk - High Contrast #1
    // - Night sky - High Contrast #2
    // Only "Desert" one can be considered a "light" preference.
    PlatformColorValues::new()
        .with_theme_variant(if high_contrast_scheme.is_some_and(|scheme| scheme.contains("White")) {
            PlatformThemeVariant::Light
        } else {
            PlatformThemeVariant::Dark
        })
        .with_contrast_preference(ColorContrastPreference::High)
        // Windows provides more than one accent color for the HighContrast themes, but with no API for that (at least not in the WinRT)
        .with_accent_color1(accent)
}

/// The colour values of a system that is not in a high contrast mode, from
/// its background and accent colours: a background that is nearer to black
/// than to white is the dark theme.
pub(crate) fn color_values_from_background(background: Color, accent: Color) -> PlatformColorValues {
    let (r, g, b) = (i32::from(background.r), i32::from(background.g), i32::from(background.b));
    PlatformColorValues::new()
        .with_theme_variant(if r + g + b < (255 * 3 - r - g - b) {
            PlatformThemeVariant::Dark
        } else {
            PlatformThemeVariant::Light
        })
        .with_contrast_preference(ColorContrastPreference::NoPreference)
        .with_accent_color1(accent)
}

/// What a change of a setting of the system (`WM_SETTINGCHANGE`) means for
/// the platform settings, by the name of the setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SettingChange {
    /// The colours: dark or light mode, or high contrast.
    ColorValues,
    /// The language or the locale.
    Language,
}

/// The change the name of a changed setting stands for, if any.
pub(crate) fn setting_change(changed_setting: Option<&str>) -> Option<SettingChange> {
    match changed_setting {
        // dark/light mode
        Some("ImmersiveColorSet")
        // high contrast mode
        | Some("WindowsThemeElement") => Some(SettingChange::ColorValues),
        // language/locale change
        Some("intl") => Some(SettingChange::Language),
        _ => None,
    }
}

#[cfg(windows)]
pub use imp::Win32PlatformSettings;

#[cfg(windows)]
mod imp {
    use super::*;
    use crate::interop::unmanaged_methods::{get_double_click_time, get_system_metrics, SystemMetric};
    use crate::win_rt::{
        HStringInterop, IAccessibilitySettings, IGlobalizationPreferencesStatics, IUISettings, IUISettings3,
        NativeWinRTMethods, UIColorType, WinRTApiInformation,
    };
    use ferroui_base::input::platform::PlatformHotkeyConfiguration;
    use ferroui_base::input::PointerType;
    use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
    use ferroui_base::platform::{DefaultPlatformSettings, IPlatformSettings};
    use ferroui_base::reactive::IDisposable;
    use ferroui_base::Size;
    use ferroui_microcom::{ComPtr, HResult, Interface};
    use std::cell::{Cell, OnceCell, RefCell};
    use std::rc::Rc;
    use std::time::Duration;

    ferroui_base::ferro_thread_local! {
        static UI_SETTINGS_SUPPORTED: OnceCell<bool> = const { OnceCell::new() };
        static GLOBALIZATION_SUPPORTED: OnceCell<bool> = const { OnceCell::new() };
    }

    fn ui_settings_supported() -> bool {
        UI_SETTINGS_SUPPORTED.with(|supported| {
            *supported.get_or_init(|| {
                WinRTApiInformation::is_type_present("Windows.UI.ViewManagement.UISettings")
                    && WinRTApiInformation::is_type_present("Windows.UI.ViewManagement.AccessibilitySettings")
            })
        })
    }

    fn globalization_supported() -> bool {
        GLOBALIZATION_SUPPORTED.with(|supported| {
            *supported
                .get_or_init(|| WinRTApiInformation::is_type_present("Windows.System.UserProfile.GlobalizationPreferences"))
        })
    }

    /// The class of the Windows Runtime that reports the settings of the user interface. It is
    /// the one source of every setting the platform settings read from `UISettings`, each
    /// through the interface that declares it in `winrt.idl`: the colours (`IUISettings3`) and
    /// the motion preference (`IUISettings`). A further setting (the advanced effects of
    /// `IUISettings4`, which WinUI's material policy reads) is read the same way: its interface
    /// declared once in `winrt.idl`, read with `ui_settings` and `read_ui_setting`.
    const UI_SETTINGS: &str = "Windows.UI.ViewManagement.UISettings";

    /// A new `UISettings`, through its interface `I`.
    fn ui_settings<I: Interface>() -> Result<ComPtr<I>, HResult> {
        NativeWinRTMethods::create_instance::<I>(UI_SETTINGS)
    }

    /// A setting read through `UISettings` by `read`: `fallback()`, the answer of a system
    /// without the class, when the class is not present or the read fails. A failure is an
    /// exception of the Windows Runtime, which nothing catches in WinUI or the reference; here
    /// it is logged with `message` (whose `{0}` is the error) and the fallback is the answer.
    fn read_ui_setting<T>(
        message: &'static str,
        read: impl FnOnce() -> Result<T, HResult>,
        fallback: impl FnOnce() -> T,
    ) -> T {
        if !ui_settings_supported() {
            return fallback();
        }
        match read() {
            Ok(value) => value,
            Err(error) => {
                if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::WIN32_PLATFORM) {
                    logger.log_with_values(None, message, &[&error]);
                }
                fallback()
            }
        }
    }

    /// The platform settings of the Windows backend.
    ///
    /// The gesture metrics of the mouse come from the system metrics. The
    /// colour values and the preferred language are read through the
    /// Windows Runtime (`UISettings`, `AccessibilitySettings`,
    /// `GlobalizationPreferences`) and kept until the platform tells the
    /// settings that the system changed them (`WM_SETTINGCHANGE` of the
    /// message window). The motion preference (`UISettings`) is read each
    /// time it is asked for.
    pub struct Win32PlatformSettings {
        base: DefaultPlatformSettings,
        color_values: Cell<Option<PlatformColorValues>>,
        last_language: RefCell<Option<String>>,
    }

    impl Win32PlatformSettings {
        /// Creates the settings.
        pub fn new() -> Rc<Win32PlatformSettings> {
            Rc::new(Win32PlatformSettings {
                base: DefaultPlatformSettings::new(),
                color_values: Cell::new(None),
                last_language: RefCell::new(None),
            })
        }

        fn get_uncached_color_values(&self) -> PlatformColorValues {
            if !ui_settings_supported() {
                return PlatformColorValues::new().with_theme_variant(PlatformThemeVariant::Light);
            }

            // A call of the Windows Runtime that fails is an exception of
            // the reference, which nothing between the call and the caller
            // of the settings catches. Here it is logged and the values are
            // the ones of a system without these types.
            match Self::read_color_values() {
                Ok(color_values) => color_values,
                Err(error) => {
                    if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::WIN32_PLATFORM) {
                        logger.log_with_values(None, "Unable to read the colour values of the system: {0}", &[&error]);
                    }
                    PlatformColorValues::new().with_theme_variant(PlatformThemeVariant::Light)
                }
            }
        }

        fn read_color_values() -> Result<PlatformColorValues, HResult> {
            let ui_settings = ui_settings::<IUISettings3>()?;
            let accent = ui_settings.get_color_value(UIColorType::Accent)?.to_ferro();
            // The shades of the accent, which WinUI reads the same way
            // (SystemThemingInterop::GetSystemVariantAccentColor). A system
            // that fails to report them keeps the other values; the themes
            // then compute the shades.
            let accent_shades = Self::read_accent_shades(&ui_settings).ok();

            let accessibility_settings = NativeWinRTMethods::create_instance::<IAccessibilitySettings>(
                "Windows.UI.ViewManagement.AccessibilitySettings",
            )?;
            if accessibility_settings.high_contrast()? == 1 {
                let high_contrast_scheme = HStringInterop::from_handle(accessibility_settings.high_contrast_scheme()?, false);
                Ok(high_contrast_color_values(high_contrast_scheme.value().as_deref(), accent)
                    .with_accent_shades(accent_shades))
            } else {
                let background = ui_settings.get_color_value(UIColorType::Background)?.to_ferro();
                Ok(color_values_from_background(background, accent).with_accent_shades(accent_shades))
            }
        }

        fn read_accent_shades(ui_settings: &IUISettings3) -> Result<AccentColorShades, HResult> {
            let mut colors = [Color::default(); 6];
            let types = [
                UIColorType::AccentDark3,
                UIColorType::AccentDark2,
                UIColorType::AccentDark1,
                UIColorType::AccentLight1,
                UIColorType::AccentLight2,
                UIColorType::AccentLight3,
            ];
            for (color, color_type) in colors.iter_mut().zip(types) {
                *color = ui_settings.get_color_value(color_type)?.to_ferro();
            }
            Ok(accent_shades_from_ui_colors(colors))
        }

        /// The system changed its colours: reads them again and raises the
        /// change event if they differ from the ones kept.
        pub(crate) fn on_color_values_changed(&self) {
            let old_color_values = self.color_values.get();
            let color_values = self.get_uncached_color_values();

            if old_color_values != Some(color_values) {
                self.color_values.set(Some(color_values));
                self.base.on_color_values_changed(color_values);
            }
        }

        /// The system changed its language: reads it again and raises the
        /// change event if it differs from the one kept.
        pub(crate) fn on_language_changed(&self) {
            let old_language = self.last_language.borrow_mut().take();
            let new_language = self.preferred_application_language();

            if old_language.as_deref() != Some(new_language.as_str()) {
                self.base.on_preferred_application_language_changed();
            }
        }

        fn query_preferred_application_language(&self) -> String {
            // `GetUserPreferredUILanguages`win32 API doesn't seem to respect Win11 "Preferred Languages" setting.
            // While GlobalizationPreferences works fine.
            if globalization_supported() {
                match Self::read_first_language() {
                    Ok(Some(language)) => return language,
                    Ok(None) => {}
                    // As for the colour values: logged, and the answer of
                    // a system without the type.
                    Err(error) => {
                        if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::WIN32_PLATFORM) {
                            logger.log_with_values(None, "Unable to read the languages of the user: {0}", &[&error]);
                        }
                    }
                }
            }

            self.base.preferred_application_language()
        }

        fn read_first_language() -> Result<Option<String>, HResult> {
            let globalization_preferences = NativeWinRTMethods::create_activation_factory::<IGlobalizationPreferencesStatics>(
                "Windows.System.UserProfile.GlobalizationPreferences",
            )?;
            let Some(languages) = globalization_preferences.languages()? else {
                return Ok(None);
            };
            if languages.size()? > 0 {
                let language_h_string = HStringInterop::from_handle(languages.get_at(0)?, false);
                if let Some(language) = language_h_string.value() {
                    return Ok(Some(language));
                }
            }
            Ok(None)
        }
    }

    impl IPlatformSettings for Win32PlatformSettings {
        fn get_tap_size(&self, type_: PointerType) -> Size {
            match type_ {
                PointerType::Mouse => Size::new(
                    f64::from(get_system_metrics(SystemMetric::SM_CXDRAG)),
                    f64::from(get_system_metrics(SystemMetric::SM_CYDRAG)),
                ),
                _ => self.base.get_tap_size(type_),
            }
        }

        fn get_double_tap_size(&self, type_: PointerType) -> Size {
            match type_ {
                PointerType::Mouse => Size::new(
                    f64::from(get_system_metrics(SystemMetric::SM_CXDOUBLECLK)),
                    f64::from(get_system_metrics(SystemMetric::SM_CYDOUBLECLK)),
                ),
                _ => self.base.get_double_tap_size(type_),
            }
        }

        fn get_double_tap_time(&self, _type: PointerType) -> Duration {
            Duration::from_millis(u64::from(get_double_click_time()))
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

        fn animations_enabled(&self) -> bool {
            // Read for each animation and not kept, as WinUI's `SharedHelpers::IsAnimationsEnabled`
            // does: a change of the setting applies to the next animation. `AnimationsEnabled`
            // is a WinRT boolean, one byte.
            read_ui_setting(
                "Unable to read the animation preference of the system: {0}",
                || Ok(ui_settings::<IUISettings>()?.animations_enabled()? != 0),
                || self.base.animations_enabled(),
            )
        }

        fn get_color_values(&self) -> PlatformColorValues {
            if let Some(color_values) = self.color_values.get() {
                return color_values;
            }
            let color_values = self.get_uncached_color_values();
            self.color_values.set(Some(color_values));
            color_values
        }

        fn color_values_changed(&self, handler: Rc<dyn Fn(&PlatformColorValues)>) -> Rc<dyn IDisposable> {
            self.base.color_values_changed(handler)
        }

        fn preferred_application_language_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
            self.base.preferred_application_language_changed(handler)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ACCENT: Color = Color::new(255, 0, 120, 215);

    #[test]
    fn a_dark_background_is_the_dark_theme() {
        let dark = color_values_from_background(Color::new(255, 0, 0, 0), ACCENT);
        assert_eq!(dark.theme_variant(), PlatformThemeVariant::Dark);
        assert_eq!(dark.contrast_preference(), ColorContrastPreference::NoPreference);
        assert_eq!(dark.accent_color1(), ACCENT);

        let light = color_values_from_background(Color::new(255, 255, 255, 255), ACCENT);
        assert_eq!(light.theme_variant(), PlatformThemeVariant::Light);

        // The sum of the channels against the sum of what is left to
        // white: 382 of 765 is still dark, 383 is light.
        assert_eq!(color_values_from_background(Color::new(255, 127, 127, 128), ACCENT).theme_variant(), PlatformThemeVariant::Dark);
        assert_eq!(color_values_from_background(Color::new(255, 127, 128, 128), ACCENT).theme_variant(), PlatformThemeVariant::Light);
    }

    #[test]
    fn only_the_white_high_contrast_scheme_is_light() {
        let white = high_contrast_color_values(Some("High Contrast White"), ACCENT);
        assert_eq!(white.theme_variant(), PlatformThemeVariant::Light);
        assert_eq!(white.contrast_preference(), ColorContrastPreference::High);
        assert_eq!(white.accent_color1(), ACCENT);

        for scheme in [Some("High Contrast Black"), Some("High Contrast #1"), Some("High Contrast #2"), Some(""), None] {
            assert_eq!(high_contrast_color_values(scheme, ACCENT).theme_variant(), PlatformThemeVariant::Dark, "{scheme:?}");
        }
    }

    #[test]
    fn the_accent_shades_follow_the_order_of_the_ui_color_types() {
        // AccentDark3, AccentDark2, AccentDark1, AccentLight1, AccentLight2,
        // AccentLight3 (shades of the kind Windows 11 reports for its blue).
        let colors = [
            Color::new(255, 0, 36, 82),
            Color::new(255, 0, 62, 146),
            Color::new(255, 0, 103, 192),
            Color::new(255, 0, 145, 248),
            Color::new(255, 76, 194, 255),
            Color::new(255, 153, 235, 255),
        ];
        let shades = accent_shades_from_ui_colors(colors);
        assert_eq!(shades.dark3, colors[0]);
        assert_eq!(shades.dark2, colors[1]);
        assert_eq!(shades.dark1, colors[2]);
        assert_eq!(shades.light1, colors[3]);
        assert_eq!(shades.light2, colors[4]);
        assert_eq!(shades.light3, colors[5]);

        // The values of either mode carry the shades they are given.
        let values = color_values_from_background(Color::new(255, 255, 255, 255), ACCENT).with_accent_shades(Some(shades));
        assert_eq!(values.accent_shades(), Some(shades));
        assert_eq!(values.accent_color1(), ACCENT);
        let values = high_contrast_color_values(Some("High Contrast Black"), ACCENT).with_accent_shades(Some(shades));
        assert_eq!(values.accent_shades(), Some(shades));
        // Without shades, none are reported.
        assert_eq!(color_values_from_background(Color::new(255, 0, 0, 0), ACCENT).accent_shades(), None);
    }

    #[cfg(windows)]
    #[test]
    fn the_platform_settings_report_the_accent_shades_of_the_system() {
        use ferroui_base::platform::IPlatformSettings;

        let settings = Win32PlatformSettings::new();
        let values = settings.get_color_values();
        println!("color values {values:?}");
        if crate::win_rt::WinRTApiInformation::is_type_present("Windows.UI.ViewManagement.UISettings") {
            let shades = values.accent_shades().expect("the shades of the accent");
            let lightness = |color: Color| color.to_hsl().l;
            assert!(lightness(shades.dark3) <= lightness(shades.dark1));
            assert!(lightness(shades.dark1) <= lightness(values.accent_color1()));
            assert!(lightness(values.accent_color1()) <= lightness(shades.light1));
            assert!(lightness(shades.light1) <= lightness(shades.light3));
        } else {
            assert_eq!(values.accent_shades(), None);
        }
    }

    #[test]
    fn the_settings_the_platform_listens_for() {
        assert_eq!(setting_change(Some("ImmersiveColorSet")), Some(SettingChange::ColorValues));
        assert_eq!(setting_change(Some("WindowsThemeElement")), Some(SettingChange::ColorValues));
        assert_eq!(setting_change(Some("intl")), Some(SettingChange::Language));
        assert_eq!(setting_change(Some("Policy")), None);
        assert_eq!(setting_change(None), None);
    }
}
