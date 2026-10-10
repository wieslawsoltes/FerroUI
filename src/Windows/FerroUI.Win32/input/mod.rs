//! Input of the Windows backend: the key tables, the devices, the input
//! method of the system and its input pane.

mod imm32_caret_manager;
mod imm32_input_method;
mod key_interop;
mod windows_input_pane;
mod windows_keyboard_device;
#[cfg(windows)]
mod windows_mouse_device;

pub use key_interop::KeyInterop;
pub use windows_keyboard_device::WindowsKeyboardDevice;
#[cfg(windows)]
pub use windows_mouse_device::{WindowsMouseDevice, WindowsMousePointer};

pub(crate) use imm32_input_method::{Imm32InputMethod, Imm32Parent};
#[cfg(windows)]
pub(crate) use windows_input_pane::WindowsInputPane;
