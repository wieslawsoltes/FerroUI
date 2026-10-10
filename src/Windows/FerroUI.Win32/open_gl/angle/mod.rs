//! OpenGL ES through ANGLE on Direct3D.

mod angle_egl_interface;
mod angle_win32_egl_display;

pub use angle_egl_interface::{is_angle_built_in, Win32AngleEglInterface};
pub use angle_win32_egl_display::AngleWin32EglDisplay;
