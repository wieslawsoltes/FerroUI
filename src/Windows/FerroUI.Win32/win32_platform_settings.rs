//! The settings of the system the toolkit reads: the sizes and times of
//! mouse gestures.

use crate::interop::unmanaged_methods::{get_double_click_time, get_system_metrics, SystemMetric};
use ferroui_base::input::platform::PlatformHotkeyConfiguration;
use ferroui_base::input::PointerType;
use ferroui_base::platform::{DefaultPlatformSettings, IPlatformSettings, PlatformColorValues};
use ferroui_base::reactive::IDisposable;
use ferroui_base::Size;
use std::rc::Rc;
use std::time::Duration;

/// The platform settings of the Windows backend.
///
/// The gesture metrics of the mouse come from the system. The colour
/// values (the theme variant, the accent colour, high contrast) and the
/// preferred language of the user are read through the Windows Runtime by
/// the reference (`UISettings`, `AccessibilitySettings`,
/// `GlobalizationPreferences`); the Windows Runtime interop is stage 2 of
/// this backend, and until then both are the defaults of the base library,
/// which is what the reference answers on a system without those types: a
/// light theme without an accent colour, and the language of the process.
pub struct Win32PlatformSettings {
    base: DefaultPlatformSettings,
}

impl Win32PlatformSettings {
    /// Creates the settings.
    pub fn new() -> Rc<Win32PlatformSettings> {
        Rc::new(Win32PlatformSettings { base: DefaultPlatformSettings::new() })
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
        self.base.preferred_application_language()
    }

    fn get_color_values(&self) -> PlatformColorValues {
        self.base.get_color_values()
    }

    fn color_values_changed(&self, handler: Rc<dyn Fn(&PlatformColorValues)>) -> Rc<dyn IDisposable> {
        self.base.color_values_changed(handler)
    }

    fn preferred_application_language_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        self.base.preferred_application_language_changed(handler)
    }
}
