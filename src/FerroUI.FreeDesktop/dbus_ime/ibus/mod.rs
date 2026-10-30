//! The input method of IBus (`DBusIme/IBus/` of the reference).

pub mod dbus;
pub mod ibus_enums;
mod ibus_input_options;
pub mod ibus_x11_text_input_method;

pub use ibus_enums::{IBusCapability, IBusInputHints, IBusInputPurpose, IBusModifierMask};
pub use ibus_x11_text_input_method::IBusX11TextInputMethod;
