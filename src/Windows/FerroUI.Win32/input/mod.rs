//! Input of the Windows backend: the key tables and the devices.

mod key_interop;
mod windows_keyboard_device;
#[cfg(windows)]
mod windows_mouse_device;

pub use key_interop::KeyInterop;
pub use windows_keyboard_device::WindowsKeyboardDevice;
#[cfg(windows)]
pub use windows_mouse_device::{WindowsMouseDevice, WindowsMousePointer};
