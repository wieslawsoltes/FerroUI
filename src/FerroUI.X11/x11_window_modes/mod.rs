//! The modes of a window: how it is shown, activated and placed, as a
//! top-level of the window manager or embedded in another window.

pub mod default_window_mode;
pub mod input_proxy_window_mode;
pub mod window_mode;

pub use default_window_mode::DefaultTopLevelWindowMode;
pub use input_proxy_window_mode::InputProxyWindowMode;
pub use window_mode::X11WindowMode;
