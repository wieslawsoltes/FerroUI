//! The settings of the system the toolkit reads: the sizes and times of
//! mouse gestures, the colours (the theme variant, the accent colour and its
//! shades, high contrast and its system colours) and the preferred language of the user.

use ferroui_base::media::Color;
use ferroui_base::platform::{
    AccentColorShades, ColorContrastPreference, PlatformColorValues, PlatformThemeVariant, SystemColors,
};

/// `SPI_SETHIGHCONTRAST`: the `wParam` of the `WM_SETTINGCHANGE` that announces a change of the
/// high contrast setting (`SystemParametersInfo`).
pub(crate) const SPI_SETHIGHCONTRAST: usize = 0x0043;

/// The indices of `GetSysColor` of the system colours the toolkit reports, in the order of
/// [`system_colors_from_sys_colors`]: `COLOR_WINDOW`, `COLOR_WINDOWTEXT`, `COLOR_BTNFACE`,
/// `COLOR_BTNTEXT`, `COLOR_HIGHLIGHT`, `COLOR_HIGHLIGHTTEXT`, `COLOR_HOTLIGHT`, `COLOR_GRAYTEXT`
/// (the eight `SystemColor*Color` resources the WinUI theme names, #560).
pub(crate) const SYS_COLOR_INDICES: [i32; 8] = [5, 8, 15, 18, 13, 14, 26, 17];

/// A colour of `GetSysColor` (a `COLORREF`, `0x00BBGGRR`), opaque as WinUI makes it
/// (`SystemThemingInterop::GetSystemColor`).
pub(crate) fn color_from_colorref(colorref: u32) -> Color {
    let [r, g, b, _] = colorref.to_le_bytes();
    Color::from_rgb(r, g, b)
}

/// The system colours from the `COLORREF`s of [`SYS_COLOR_INDICES`].
pub(crate) fn system_colors_from_sys_colors(colors: [u32; 8]) -> SystemColors {
    let [window, window_text, button_face, button_text, highlight, highlight_text, hotlight, gray_text] =
        colors.map(color_from_colorref);
    SystemColors::new(window, window_text)
        .with_button_face(button_face)
        .with_button_text(button_text)
        .with_highlight(highlight)
        .with_highlight_text(highlight_text)
        .with_hotlight(hotlight)
        .with_gray_text(gray_text)
}

/// Whether the battery saver is on, from the flag of the power status
/// (`SYSTEM_POWER_STATUS.SystemStatusFlag`: 1 while the battery saver is on), which is what
/// `PowerManager.EnergySaverStatus` of WinUI's material policy reports.
pub(crate) fn energy_saver_from(system_status_flag: u8) -> bool {
    system_status_flag == 1
}

/// The shades of the accent colour from the colours `UISettings` reports
/// for `UIColorType` `AccentDark3` to `AccentLight3`, in the order of the
/// enumeration (FerroUI addition: the reference reads only the accent).
pub(crate) fn accent_shades_from_ui_colors(colors: [Color; 6]) -> AccentColorShades {
    let [dark3, dark2, dark1, light1, light2, light3] = colors;
    AccentColorShades { dark1, dark2, dark3, light1, light2, light3 }
}

/// Serializes the tests that read the colours of the system through `UISettings`. On a Windows
/// runner `GetColorValue(Accent)` of a new `UISettings` reported opaque black, with valid shades,
/// in up to half of the runs while another thread of the process read the same settings, and
/// never in a run alone.
///
/// Only tests read them on several threads, each with a platform of its own. In an application
/// this file is the only reader, on one thread: `Win32PlatformSettings` is not `Send`, it is made
/// by `Win32Platform::initialize`, which a process can call once, and the messages that refresh it
/// arrive on the thread of its message window. Windows share that thread and the render thread
/// does not read it. Code outside the framework reading `UISettings` is not serialized by this.
#[cfg(test)]
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn ui_settings_test_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The colour values of a system in a high contrast mode. The window colour
/// determines the variant for stock, localized, and edited contrast themes.
pub(crate) fn high_contrast_color_values(system_colors: SystemColors, accent: Color) -> PlatformColorValues {
    PlatformColorValues::new()
        .with_theme_variant(system_colors.theme_variant())
        .with_contrast_preference(ColorContrastPreference::High)
        .with_accent_color1(accent)
        .with_system_colors(Some(system_colors))
}

fn fallback_color_values(high_contrast: Option<bool>, system_colors: impl FnOnce() -> SystemColors) -> PlatformColorValues {
    let values = PlatformColorValues::new();
    if high_contrast == Some(true) {
        high_contrast_color_values(system_colors(), values.accent_color1())
    } else {
        values.with_theme_variant(PlatformThemeVariant::Light)
    }
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

/// The change a `WM_SETTINGCHANGE` stands for, by its `wParam` (the action of
/// `SystemParametersInfo`, `SPI_SETHIGHCONTRAST` for high contrast, which WinUI listens for)
/// and the name of the changed setting.
pub(crate) fn setting_change_of(action: usize, changed_setting: Option<&str>) -> Option<SettingChange> {
    if action == SPI_SETHIGHCONTRAST {
        return Some(SettingChange::ColorValues);
    }
    setting_change(changed_setting)
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
        IUISettings4, NativeWinRTMethods, UIColorType, WinRTApiInformation,
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
    /// through the interface that declares it in `winrt.idl`: the colours (`IUISettings3`), the
    /// motion preference (`IUISettings`) and the transparency effects (the advanced effects of
    /// `IUISettings4`, which WinUI's material policy reads), read with `ui_settings` and
    /// `read_ui_setting`.
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
    /// contrast flag and palette come from Win32, with the Windows Runtime
    /// supplying the accent, normal theme and preferred language. The values
    /// are kept until the platform tells the
    /// settings that the system changed them (`WM_SETTINGCHANGE` of the
    /// message window). The motion preference and the transparency effects
    /// (`UISettings`) and the energy saver (the power status) are read each
    /// time they are asked for.
    pub struct Win32PlatformSettings {
        base: DefaultPlatformSettings,
        color_values: Cell<Option<PlatformColorValues>>,
        last_language: RefCell<Option<String>>,
        /// The state of the energy saver last reported, to raise its change once.
        energy_saver: Cell<Option<bool>>,
    }

    impl Win32PlatformSettings {
        /// Creates the settings.
        pub fn new() -> Rc<Win32PlatformSettings> {
            Rc::new(Win32PlatformSettings {
                base: DefaultPlatformSettings::new(),
                color_values: Cell::new(None),
                last_language: RefCell::new(None),
                energy_saver: Cell::new(None),
            })
        }

        fn read_energy_saver() -> bool {
            use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
            // SAFETY: the structure is written by the call.
            let mut power: SYSTEM_POWER_STATUS = unsafe { std::mem::zeroed() };
            let flag = if unsafe { GetSystemPowerStatus(&mut power) } != 0 { power.SystemStatusFlag } else { 0 };
            energy_saver_from(flag)
        }

        /// The power status may have changed (`WM_POWERBROADCAST`): raises
        /// [`IPlatformSettings::energy_saver_changed`] when the energy saver was turned on or off.
        pub(crate) fn on_power_status_changed(&self) {
            let energy_saver = Self::read_energy_saver();
            if self.energy_saver.replace(Some(energy_saver)) != Some(energy_saver) {
                self.base.on_energy_saver_changed();
            }
        }

        /// The transparency effects setting may have changed (it is announced with the colours,
        /// `ImmersiveColorSet`): raises [`IPlatformSettings::accessibility_settings_changed`],
        /// whose handlers read the current values.
        pub(crate) fn on_accessibility_settings_changed(&self) {
            self.base.on_accessibility_settings_changed();
        }

        fn get_uncached_color_values(&self) -> PlatformColorValues {
            let system_high_contrast = Self::read_system_high_contrast();
            let fallback = || fallback_color_values(system_high_contrast, Self::read_system_colors);
            if !ui_settings_supported() {
                return fallback();
            }

            // A call of the Windows Runtime that fails is an exception of
            // the reference, which nothing between the call and the caller
            // of the settings catches. Here it is logged and the native
            // contrast palette stays available.
            match Self::read_color_values(system_high_contrast) {
                Ok(color_values) => color_values,
                Err(error) => {
                    if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::WIN32_PLATFORM) {
                        logger.log_with_values(None, "Unable to read the colour values of the system: {0}", &[&error]);
                    }
                    fallback()
                }
            }
        }

        /// Whether high contrast is on, as WinUI asks (`SystemParametersInfo(SPI_GETHIGHCONTRAST)`
        /// with `HCF_HIGHCONTRASTON`, `SystemThemingInterop::GetSystemHighContrastTheme`); `None`
        /// when the call fails.
        fn read_system_high_contrast() -> Option<bool> {
            use windows_sys::Win32::UI::WindowsAndMessaging::{SystemParametersInfoW, SPI_GETHIGHCONTRAST};
            /// `HIGHCONTRASTW` of `Win32_UI_Accessibility`, declared here.
            #[repr(C)]
            struct HighContrast {
                cb_size: u32,
                dw_flags: u32,
                lpsz_default_scheme: *mut u16,
            }
            const HCF_HIGHCONTRASTON: u32 = 0x0000_0001;
            let mut high_contrast = HighContrast {
                cb_size: std::mem::size_of::<HighContrast>() as u32,
                dw_flags: 0,
                lpsz_default_scheme: std::ptr::null_mut(),
            };
            // SAFETY: the structure is the one the action fills, with its size set.
            let ok = unsafe {
                SystemParametersInfoW(
                    SPI_GETHIGHCONTRAST,
                    high_contrast.cb_size,
                    (&mut high_contrast as *mut HighContrast).cast(),
                    0,
                )
            };
            (ok != 0).then_some(high_contrast.dw_flags & HCF_HIGHCONTRASTON != 0)
        }

        /// The system colours of the active contrast theme (`GetSysColor`).
        fn read_system_colors() -> SystemColors {
            use windows_sys::Win32::Graphics::Gdi::GetSysColor;
            // SAFETY: `GetSysColor` takes any index and reads no memory of the caller.
            system_colors_from_sys_colors(SYS_COLOR_INDICES.map(|index| unsafe { GetSysColor(index) }))
        }

        fn read_color_values(system_high_contrast: Option<bool>) -> Result<PlatformColorValues, HResult> {
            let ui_settings = ui_settings::<IUISettings3>()?;
            let accent = ui_settings.get_color_value(UIColorType::Accent)?.to_ferro();
            // The shades of the accent, which WinUI reads the same way
            // (SystemThemingInterop::GetSystemVariantAccentColor). A system
            // that fails to report them keeps the other values; the themes
            // then compute the shades.
            let accent_shades = Self::read_accent_shades(&ui_settings).ok();

            // High contrast as WinUI reads it, `AccessibilitySettings` where that fails (#560).
            let high_contrast = match system_high_contrast {
                Some(high_contrast) => high_contrast,
                None => NativeWinRTMethods::create_instance::<IAccessibilitySettings>(
                    "Windows.UI.ViewManagement.AccessibilitySettings",
                )
                .and_then(|settings| settings.high_contrast())
                .is_ok_and(|value| value == 1),
            };
            if high_contrast {
                Ok(high_contrast_color_values(Self::read_system_colors(), accent).with_accent_shades(accent_shades))
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

        fn transparency_enabled(&self) -> bool {
            // `UISettings.AdvancedEffectsEnabled`, the transparency effects setting that WinUI's
            // material policy reads; read each time, as the motion preference is.
            read_ui_setting(
                "Unable to read the transparency effects setting of the system: {0}",
                || Ok(ui_settings::<IUISettings4>()?.advanced_effects_enabled()? != 0),
                || self.base.transparency_enabled(),
            )
        }

        fn accessibility_settings_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
            self.base.accessibility_settings_changed(handler)
        }

        fn energy_saver_enabled(&self) -> bool {
            Self::read_energy_saver()
        }

        fn energy_saver_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
            self.base.energy_saver_changed(handler)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ACCENT: Color = Color::new(255, 0, 120, 215);

    #[test]
    fn the_energy_saver_follows_the_battery_saver_flag_of_the_power_status() {
        assert!(!energy_saver_from(0));
        assert!(energy_saver_from(1));
        // 255: the status is unknown.
        assert!(!energy_saver_from(255));
    }

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
    fn high_contrast_variant_follows_the_window_colour() {
        let dark = SystemColors::high_contrast_black();
        let light = SystemColors::new(Color::from_rgb(255, 250, 239), Color::from_rgb(61, 61, 61));
        for (colors, variant) in [(dark, PlatformThemeVariant::Dark), (light, PlatformThemeVariant::Light)] {
            let values = high_contrast_color_values(colors, ACCENT);
            assert_eq!(values.theme_variant(), variant);
            assert_eq!(values.contrast_preference(), ColorContrastPreference::High);
            assert_eq!(values.accent_color1(), ACCENT);
            assert_eq!(values.system_colors(), Some(colors));
        }
    }

    #[test]
    fn winrt_colour_failure_preserves_an_active_win32_contrast_palette() {
        let colors = SystemColors::new(Color::from_rgb(236, 242, 255), Color::from_rgb(15, 27, 61));
        let values = fallback_color_values(Some(true), || colors);
        assert!(values.is_high_contrast());
        assert_eq!(values.system_colors(), Some(colors));
        assert_eq!(values.theme_variant(), PlatformThemeVariant::Light);
        for high_contrast in [Some(false), None] {
            let values = fallback_color_values(high_contrast, || panic!("ordinary theme needs no contrast palette"));
            assert!(!values.is_high_contrast());
            assert_eq!(values.system_colors(), None);
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
        let values = high_contrast_color_values(SystemColors::high_contrast_black(), ACCENT).with_accent_shades(Some(shades));
        assert_eq!(values.accent_shades(), Some(shades));
        // Without shades, none are reported.
        assert_eq!(color_values_from_background(Color::new(255, 0, 0, 0), ACCENT).accent_shades(), None);
    }

    #[cfg(windows)]
    #[test]
    fn the_platform_settings_report_the_accent_shades_of_the_system() {
        use ferroui_base::platform::IPlatformSettings;

        let _ui_settings = ui_settings_test_lock();
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

        // The high contrast action of SystemParametersInfo, with or without a name.
        assert_eq!(setting_change_of(SPI_SETHIGHCONTRAST, None), Some(SettingChange::ColorValues));
        let named = setting_change_of(SPI_SETHIGHCONTRAST, Some("WindowsThemeElement"));
        assert_eq!(named, Some(SettingChange::ColorValues));
        assert_eq!(setting_change_of(0, Some("intl")), Some(SettingChange::Language));
        assert_eq!(setting_change_of(0x002F, None), None);
    }

    #[test]
    fn the_system_colors_are_read_from_the_colorrefs_of_get_sys_color() {
        // A COLORREF is 0x00BBGGRR; the colour is made opaque.
        assert_eq!(color_from_colorref(0x00FF_0000), Color::from_rgb(0, 0, 255));
        assert_eq!(color_from_colorref(0x0000_00FF), Color::from_rgb(255, 0, 0));
        assert_eq!(color_from_colorref(0xFF00_FF00), Color::from_rgb(0, 255, 0));

        // "High Contrast Black" of Windows 10, in the order of SYS_COLOR_INDICES.
        let black = system_colors_from_sys_colors([
            0x0000_0000, 0x00FF_FFFF, 0x0000_0000, 0x00FF_FFFF, 0x00FF_EB1A, 0x0000_0000, 0x0000_FFFF, 0x003F_F23F,
        ]);
        assert_eq!(black, SystemColors::high_contrast_black());
        assert_eq!(black.theme_variant(), PlatformThemeVariant::Dark);
        // The window colour of "Desert" of Windows 11 (#FFFAEF) is light.
        let desert = system_colors_from_sys_colors([0x00EF_FAFF, 0x003D_3D3D, 0, 0, 0, 0, 0, 0]);
        assert_eq!(desert.window(), Color::from_rgb(0xFF, 0xFA, 0xEF));
        assert_eq!(desert.theme_variant(), PlatformThemeVariant::Light);

        // The high contrast values carry them; an unchanged scheme compares equal.
        let values = high_contrast_color_values(black, ACCENT);
        assert_eq!(values.system_colors(), Some(SystemColors::high_contrast_black()));
        let again = high_contrast_color_values(black, ACCENT);
        assert_eq!(values, again);
        assert_eq!(SYS_COLOR_INDICES.len(), 8);
    }
}
