use super::{IPlatformSettings, PlatformColorValues, PlatformThemeVariant};
use crate::input::platform::PlatformHotkeyConfiguration;
use crate::input::PointerType;
use crate::reactive::{Disposable, IDisposable};
use crate::utilities::HandlerList;
use crate::{FerroLocator, LocatorExtensions, Size};
use std::rc::Rc;
use std::time::Duration;

const TOUCH_TAP_SIZE: f64 = 10.0;
/// The default double tap distance for touch input on Windows.
const TOUCH_DOUBLE_TAP_SIZE: f64 = 50.0;

/// A default implementation of [`IPlatformSettings`] for platforms that do
/// not provide settings of their own, and the base the settings of the
/// platform backends are built on: a backend embeds one, delegates what it
/// does not replace and raises the change events through
/// [`on_color_values_changed`](Self::on_color_values_changed) and
/// [`on_preferred_application_language_changed`](Self::on_preferred_application_language_changed).
#[derive(Default)]
pub struct DefaultPlatformSettings {
    color_values_changed: Rc<HandlerList<dyn Fn(&PlatformColorValues)>>,
    preferred_application_language_changed: Rc<HandlerList<dyn Fn()>>,
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
        PlatformColorValues::new().with_theme_variant(PlatformThemeVariant::Light)
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn default_color_values_are_light() {
        let settings = DefaultPlatformSettings::new();
        assert_eq!(settings.get_color_values(), PlatformColorValues::new());
        assert_eq!(settings.get_color_values().theme_variant(), PlatformThemeVariant::Light);
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
    fn language_tag_is_derived_from_the_locale_name() {
        assert_eq!(DefaultPlatformSettings::language_tag_from_locale("en_US.UTF-8"), "en-US");
        assert_eq!(DefaultPlatformSettings::language_tag_from_locale("sr_RS@latin"), "sr-RS");
        assert_eq!(DefaultPlatformSettings::language_tag_from_locale("pl"), "pl");
        assert_eq!(DefaultPlatformSettings::language_tag_from_locale("C"), "");
        assert_eq!(DefaultPlatformSettings::language_tag_from_locale("POSIX"), "");
        assert_eq!(DefaultPlatformSettings::language_tag_from_locale(""), "");
    }
}
