//! OpenGL ES through ANGLE on Direct3D.

mod angle_egl_interface;
mod angle_win32_egl_display;
mod angle_win32_platform_graphics_factory;
mod d3d11_angle_win32_platform_graphics;
mod d3d9_angle_win32_platform_graphics;

pub use angle_egl_interface::{is_angle_built_in, Win32AngleEglInterface};
pub use angle_win32_egl_display::{AngleWin32EglDisplay, D3D11Adapter};
pub use angle_win32_platform_graphics_factory::{AnglePlatformGraphics, AngleWin32PlatformGraphicsFactory};
pub use d3d11_angle_win32_platform_graphics::D3D11AngleWin32PlatformGraphics;
pub use d3d9_angle_win32_platform_graphics::D3D9AngleWin32PlatformGraphics;
