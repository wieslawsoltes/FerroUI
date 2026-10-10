//! The input methods that are reached over D-Bus (`DBusIme/` of the
//! reference).

pub mod dbus_text_input_method_base;
pub mod fcitx;
pub mod ibus;
pub mod x11_dbus_ime_helper;

pub use dbus_text_input_method_base::{DBusInputMethodFactory, DBusTextInputMethodBase, DBusTextInputMethodCore};
pub use x11_dbus_ime_helper::{KnownMethod, X11DBusImeHelper, IM_MODULE_VARIABLE};

#[cfg(test)]
mod tests;
