//! The input method of Fcitx (`DBusIme/Fcitx/` of the reference).

pub mod dbus;
pub mod fcitx_enums;
pub mod fcitx_ic_wrapper;
pub mod fcitx_x11_text_input_method;

pub use fcitx_enums::{FcitxCapabilityFlags, FcitxKeyEventType, FcitxKeyState};
pub use fcitx_ic_wrapper::FcitxICWrapper;
pub use fcitx_x11_text_input_method::FcitxX11TextInputMethod;
