use super::{IPlatformSettings, PlatformColorValues};
use crate::input::platform::PlatformHotkeyConfiguration;
use crate::input::PointerType;
use crate::reactive::{Disposable, IDisposable};
use crate::utilities::HandlerList;
use crate::{FerroLocator, LocatorExtensions, Size};
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

const TOUCH_TAP_SIZE: f64 = 10.0;
/// The default double tap distance for touch input on Windows.
const TOUCH_DOUBLE_TAP_SIZE: f64 = 50.0;

/// A default implementation of [`IPlatformSettings`] for platforms that do
/// not provide settings of their own, and the base the settings of the
/// platform backends are built on: a backend embeds one, delegates what it
/// does not replace and raises the change events through
/// [`on_color_values_changed`](Self::on_color_values_changed),
/// [`on_preferred_application_language_changed`](Self::on_preferred_application_language_changed) and
/// [`on_accessibility_settings_changed`](Self::on_accessibility_settings_changed).
///
/// Its colour values are the defaults (light, no contrast preference) until
/// [`set_color_values`](Self::set_color_values) simulates others: the platform-neutral setting
/// of the headless platform and of tests (a simulated high contrast, a dark theme, system
/// colours), raised to the subscribers as a platform raises its own.
#[derive(Default)]
pub struct DefaultPlatformSettings {
    color_values: Cell<PlatformColorValues>,
    color_values_changed: Rc<HandlerList<dyn Fn(&PlatformColorValues)>>,
    preferred_application_language_changed: Rc<HandlerList<dyn Fn()>>,
    accessibility_settings_changed: Rc<HandlerList<dyn Fn()>>,
    energy_saver_changed: Rc<HandlerList<dyn Fn()>>,
}

impl DefaultPlatformSettings {
    /// Creates the default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Raises [`IPlatformSettings::color_values_changed`].
    ///
    /// The settings belong to the UI thread: a backend that learns about
    /// the change on another thread marshals to the UI thread first.
    pub fn on_color_values_changed(&self, color_values: PlatformColorValues) {
        if self.color_values_changed.is_empty() {
            return;
        }
        for (_, handler) in self.color_values_changed.snapshot().iter() {
            handler(&color_values);
        }
    }

    /// Simulates the colour values of the platform: [`get_color_values`](IPlatformSettings::get_color_values)
    /// answers `values` from now on and, when they differ from the previous ones,
    /// [`color_values_changed`](IPlatformSettings::color_values_changed) is raised with them.
    ///
    /// For the headless platform and tests (a simulated contrast setting, theme or system
    /// colours); a backend that embeds these settings answers its own colour values and raises
    /// its changes with [`on_color_values_changed`](Self::on_color_values_changed). A FerroUI
    /// addition.
    pub fn set_color_values(&self, values: PlatformColorValues) {
        if self.color_values.replace(values) != values {
            self.on_color_values_changed(values);
        }
    }

    /// Raises [`IPlatformSettings::preferred_application_language_changed`].
    ///
    /// The settings belong to the UI thread: a backend that learns about
    /// the change on another thread marshals to the UI thread first.
    pub fn on_preferred_application_language_changed(&self) {
        if self.preferred_application_language_changed.is_empty() {
            return;
        }
        for (_, handler) in self.preferred_application_language_changed.snapshot().iter() {
            handler();
        }
    }

    /// Raises [`IPlatformSettings::accessibility_settings_changed`] on the UI
    /// thread. Snapshot dispatch permits a handler to unsubscribe itself.
    pub fn on_accessibility_settings_changed(&self) {
        for (_, handler) in self.accessibility_settings_changed.snapshot().iter() {
            handler();
        }
    }

    /// Raises [`IPlatformSettings::energy_saver_changed`] on the UI thread. Snapshot dispatch
    /// permits a handler to unsubscribe itself.
    pub fn on_energy_saver_changed(&self) {
        for (_, handler) in self.energy_saver_changed.snapshot().iter() {
            handler();
        }
    }

    /// The language tag from a POSIX locale name: `en_US.UTF-8` gives
    /// `en-US`. The `C` and `POSIX` locales give the empty (invariant) tag.
    fn language_tag_from_locale(locale: &str) -> String {
        let name = locale.split(['.', '@']).next().unwrap_or("");
        if name.is_empty() || name == "C" || name == "POSIX" {
            return String::new();
        }
        name.replace('_', "-")
    }
}

impl IPlatformSettings for DefaultPlatformSettings {
    fn get_tap_size(&self, type_: PointerType) -> Size {
        match type_ {
            PointerType::Touch | PointerType::Pen => Size::new(TOUCH_TAP_SIZE, TOUCH_TAP_SIZE),
            PointerType::Mouse => Size::new(4.0, 4.0),
        }
    }

    fn get_double_tap_size(&self, type_: PointerType) -> Size {
        match type_ {
            PointerType::Touch | PointerType::Pen => Size::new(TOUCH_DOUBLE_TAP_SIZE, TOUCH_DOUBLE_TAP_SIZE),
            PointerType::Mouse => Size::new(4.0, 4.0),
        }
    }

    fn get_double_tap_time(&self, _type: PointerType) -> Duration {
        Duration::from_millis(500)
    }

    fn hold_wait_duration(&self) -> Duration {
        Duration::from_millis(300)
    }

    /// The hotkey configuration registered with the locator.
    ///
    /// Panics if none is registered.
    fn hotkey_configuration(&self) -> Rc<PlatformHotkeyConfiguration> {
        FerroLocator::current().get_required_service::<PlatformHotkeyConfiguration>()
    }

    /// The language of the user interface the environment asks for
    /// (`LC_ALL`, `LC_MESSAGES`, `LANG`, in this order), or the empty
    /// (invariant) tag when it does not say.
    fn preferred_application_language(&self) -> String {
        ["LC_ALL", "LC_MESSAGES", "LANG"]
            .iter()
            .filter_map(|name| std::env::var(name).ok())
            .find(|value| !value.is_empty())
            .map(|value| Self::language_tag_from_locale(&value))
            .unwrap_or_default()
    }

    fn get_color_values(&self) -> PlatformColorValues {
        self.color_values.get()
    }

    fn color_values_changed(&self, handler: Rc<dyn Fn(&PlatformColorValues)>) -> Rc<dyn IDisposable> {
        let token = self.color_values_changed.add(handler);
        let handlers = self.color_values_changed.clone();
        Disposable::create(move || {
            handlers.remove(token);
        })
    }

    fn preferred_application_language_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        let token = self.preferred_application_language_changed.add(handler);
        let handlers = self.preferred_application_language_changed.clone();
        Disposable::create(move || {
            handlers.remove(token);
        })
    }

    fn accessibility_settings_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        let token = self.accessibility_settings_changed.add(handler);
        let handlers = self.accessibility_settings_changed.clone();
        Disposable::create(move || {
            handlers.remove(token);
        })
    }

    fn energy_saver_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        let token = self.energy_saver_changed.add(handler);
        let handlers = self.energy_saver_changed.clone();
        Disposable::create(move || {
            handlers.remove(token);
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::PlatformThemeVariant;
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn default_color_values_are_light() {
        let settings = DefaultPlatformSettings::new();
        assert_eq!(settings.get_color_values(), PlatformColorValues::new());
        assert_eq!(settings.get_color_values().theme_variant(), PlatformThemeVariant::Light);
    }

    #[test]
    fn simulated_color_values_are_answered_and_raised_once_per_change() {
        use super::super::{ColorContrastPreference, SystemColors};
        let settings = DefaultPlatformSettings::new();
        let raised = Rc::new(RefCell::new(Vec::new()));
        let _subscription = {
            let raised = raised.clone();
            settings
                .color_values_changed(Rc::new(move |values: &PlatformColorValues| raised.borrow_mut().push(*values)))
        };

        let high = PlatformColorValues::new()
            .with_theme_variant(PlatformThemeVariant::Dark)
            .with_contrast_preference(ColorContrastPreference::High)
            .with_system_colors(Some(SystemColors::high_contrast_black()));
        settings.set_color_values(high);
        assert_eq!(settings.get_color_values(), high);
        // The same values again are no change.
        settings.set_color_values(high);
        assert_eq!(*raised.borrow(), [high]);

        settings.set_color_values(PlatformColorValues::new());
        assert_eq!(settings.get_color_values(), PlatformColorValues::new());
        assert_eq!(raised.borrow().len(), 2);
    }

    #[test]
    fn change_events_reach_subscribers_until_disposed() {
        let settings = DefaultPlatformSettings::new();
        let log = Rc::new(RefCell::new(Vec::new()));

        let colors = {
            let log = log.clone();
            settings.color_values_changed(Rc::new(move |values: &PlatformColorValues| {
                log.borrow_mut().push(format!("{:?}", values.theme_variant()))
            }))
        };
        let language = {
            let log = log.clone();
            settings.preferred_application_language_changed(Rc::new(move || log.borrow_mut().push("language".to_string())))
        };

        settings.on_color_values_changed(PlatformColorValues::new().with_theme_variant(PlatformThemeVariant::Dark));
        settings.on_preferred_application_language_changed();
        assert_eq!(*log.borrow(), ["Dark", "language"]);

        colors.dispose();
        language.dispose();
        settings.on_color_values_changed(PlatformColorValues::new());
        settings.on_preferred_application_language_changed();
        assert_eq!(log.borrow().len(), 2);
    }

    #[test]
    fn a_platform_that_does_not_report_the_motion_preference_lets_animations_play() {
        let settings = DefaultPlatformSettings::new();
        assert!(settings.animations_enabled());
        let as_contract: &dyn IPlatformSettings = &settings;
        assert!(as_contract.animations_enabled());
        assert!(settings.transparency_enabled());
        assert!(as_contract.transparency_enabled());
    }

    #[test]
    fn the_energy_saver_is_off_and_its_changes_unsubscribe_cleanly() {
        use std::cell::Cell;
        let settings = DefaultPlatformSettings::new();
        assert!(!IPlatformSettings::energy_saver_enabled(&settings));
        let count = Rc::new(Cell::new(0));
        let subscription = {
            let count = count.clone();
            settings.energy_saver_changed(Rc::new(move || count.set(count.get() + 1)))
        };
        settings.on_energy_saver_changed();
        settings.on_accessibility_settings_changed();
        assert_eq!(count.get(), 1);
        subscription.dispose();
        settings.on_energy_saver_changed();
        assert_eq!(count.get(), 1);
    }

    #[test]
    fn accessibility_changes_are_independent_and_unsubscribe_cleanly() {
        use std::cell::Cell;
        let settings = DefaultPlatformSettings::new();
        let count = Rc::new(Cell::new(0));
        let changes = {
            let count = count.clone();
            settings.accessibility_settings_changed(Rc::new(move || count.set(count.get() + 1)))
        };
        settings.on_accessibility_settings_changed();
        settings.on_color_values_changed(PlatformColorValues::new());
        settings.on_preferred_application_language_changed();
        settings.on_energy_saver_changed();
        assert_eq!(count.get(), 1);
        changes.dispose();
        settings.on_accessibility_settings_changed();
        assert_eq!(count.get(), 1);
    }

    #[test]
    fn accessibility_handler_can_dispose_itself_during_notification() {
        use std::cell::Cell;
        let settings = DefaultPlatformSettings::new();
        let count = Rc::new(Cell::new(0));
        let subscription: Rc<RefCell<Option<Rc<dyn IDisposable>>>> = Rc::new(RefCell::new(None));
        let token = {
            let weak_subscription = Rc::downgrade(&subscription);
            let count = count.clone();
            settings.accessibility_settings_changed(Rc::new(move || {
                count.set(count.get() + 1);
                if let Some(subscription) = weak_subscription.upgrade() {
                    if let Some(token) = subscription.borrow_mut().take() {
                        token.dispose();
                    }
                }
            }))
        };
        *subscription.borrow_mut() = Some(token);
        settings.on_accessibility_settings_changed();
        settings.on_accessibility_settings_changed();
        assert_eq!(count.get(), 1);
    }

    #[test]
    fn language_tag_is_derived_from_the_locale_name() {
        assert_eq!(DefaultPlatformSettings::language_tag_from_locale("en_US.UTF-8"), "en-US");
        assert_eq!(DefaultPlatformSettings::language_tag_from_locale("sr_RS@latin"), "sr-RS");
        assert_eq!(DefaultPlatformSettings::language_tag_from_locale("pl"), "pl");
        assert_eq!(DefaultPlatformSettings::language_tag_from_locale("C"), "");
        assert_eq!(DefaultPlatformSettings::language_tag_from_locale("POSIX"), "");
        assert_eq!(DefaultPlatformSettings::language_tag_from_locale(""), "");
    }
}
