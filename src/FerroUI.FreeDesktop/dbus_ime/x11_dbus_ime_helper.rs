//! Which input method over D-Bus the session is configured for (the port
//! of `X11DBusImeHelper.cs`).

use super::dbus_text_input_method_base::DBusInputMethodFactory;
use super::fcitx::FcitxX11TextInputMethod;
use super::ibus::IBusX11TextInputMethod;
use crate::dbus_helper::DBusHelper;
use crate::ix11_input_method::IX11InputMethodFactory;
use ferroui_base::FerroLocator;
use std::rc::Rc;
use zbus::Connection;

/// The variable of this framework that names the input method module
/// (`AVALONIA_IM_MODULE` of the reference, renamed: docs/porting/DEVIATIONS.md).
pub const IM_MODULE_VARIABLE: &str = "FERROUI_IM_MODULE";

/// The input methods this crate has (`KnownMethods`, as its keys with the
/// class each makes).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KnownMethod {
    /// `fcitx` and `fcitx5`.
    Fcitx,
    /// `ibus`.
    IBus,
}

impl KnownMethod {
    fn from_name(name: &str) -> Option<KnownMethod> {
        match name {
            "fcitx" | "fcitx5" => Some(KnownMethod::Fcitx),
            "ibus" => Some(KnownMethod::IBus),
            _ => None,
        }
    }

    /// The factory of the input method over `conn`.
    pub fn create_factory(self, conn: Connection) -> Rc<dyn IX11InputMethodFactory> {
        match self {
            KnownMethod::Fcitx => {
                Rc::new(DBusInputMethodFactory::new(move |_| FcitxX11TextInputMethod::new(conn.clone())))
            }
            KnownMethod::IBus => {
                Rc::new(DBusInputMethodFactory::new(move |_| IBusX11TextInputMethod::new(conn.clone())))
            }
        }
    }
}

pub struct X11DBusImeHelper;

impl X11DBusImeHelper {
    /// The input method the environment names (`DetectInputMethod`), from
    /// the values of the module variable of this framework, of
    /// `GTK_IM_MODULE` and `QT_IM_MODULE` in that order, and then of
    /// `XMODIFIERS`.
    pub fn detect_input_method_from(im_modules: [Option<&str>; 3], x_modifiers: Option<&str>) -> Option<KnownMethod> {
        for value in im_modules {
            if value == Some("none") {
                return None;
            }

            if let Some(method) = value.and_then(KnownMethod::from_name) {
                return Some(method);
            }
        }

        if let Some(modifiers) = x_modifiers {
            if let Some(index) = modifiers.find("@im=") {
                let im_name_start = index + "@im=".len();
                let im_name = match modifiers[im_name_start..].find('@') {
                    None => &modifiers[im_name_start..],
                    Some(length) => &modifiers[im_name_start..im_name_start + length],
                };

                if let Some(method) = KnownMethod::from_name(im_name) {
                    return Some(method);
                }
            }
        }

        None
    }

    fn detect_input_method() -> Option<KnownMethod> {
        let values = [IM_MODULE_VARIABLE, "GTK_IM_MODULE", "QT_IM_MODULE"].map(|name| std::env::var(name).ok());
        Self::detect_input_method_from(
            [values[0].as_deref(), values[1].as_deref(), values[2].as_deref()],
            std::env::var("XMODIFIERS").ok().as_deref(),
        )
    }

    /// Registers the factory of the configured input method when there is
    /// one and the session bus can be reached (`DetectAndRegister`).
    pub fn detect_and_register() -> bool {
        if let Some(method) = Self::detect_input_method() {
            if let Some(conn) = DBusHelper::default_connection() {
                FerroLocator::current_mutable()
                    .bind::<dyn IX11InputMethodFactory>()
                    .to_constant(method.create_factory(conn));
                return true;
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of this class.
    use super::*;

    fn detect(own: Option<&str>, gtk: Option<&str>, qt: Option<&str>, x_modifiers: Option<&str>) -> Option<KnownMethod> {
        X11DBusImeHelper::detect_input_method_from([own, gtk, qt], x_modifiers)
    }

    #[test]
    fn the_module_variables_are_asked_in_order() {
        assert_eq!(detect(Some("ibus"), Some("fcitx"), None, None), Some(KnownMethod::IBus));
        assert_eq!(detect(None, Some("fcitx"), Some("ibus"), None), Some(KnownMethod::Fcitx));
        assert_eq!(detect(None, None, Some("fcitx5"), None), Some(KnownMethod::Fcitx));
        // A module this crate does not know is passed over.
        assert_eq!(detect(Some("xim"), Some("gtk-im-context-simple"), Some("ibus"), None), Some(KnownMethod::IBus));
        assert_eq!(detect(None, None, None, None), None);
    }

    #[test]
    fn none_ends_the_search() {
        assert_eq!(detect(Some("none"), Some("ibus"), None, Some("@im=ibus")), None);
        // Only when it is reached: an earlier variable wins.
        assert_eq!(detect(Some("ibus"), Some("none"), None, None), Some(KnownMethod::IBus));
        assert_eq!(detect(None, Some("none"), Some("fcitx"), Some("@im=fcitx")), None);
    }

    #[test]
    fn the_modifiers_of_the_locale_name_the_input_method_last() {
        assert_eq!(detect(None, None, None, Some("@im=ibus")), Some(KnownMethod::IBus));
        assert_eq!(detect(None, None, None, Some("@im=fcitx@other=1")), Some(KnownMethod::Fcitx));
        assert_eq!(detect(None, None, None, Some("@other=1@im=fcitx5")), Some(KnownMethod::Fcitx));
        assert_eq!(detect(None, None, None, Some("@im=none")), None);
        assert_eq!(detect(None, None, None, Some("@im=")), None);
        assert_eq!(detect(None, None, None, Some("ibus")), None);
        // The module variables come first.
        assert_eq!(detect(None, Some("ibus"), None, Some("@im=fcitx")), Some(KnownMethod::IBus));
    }
}
