//! OpenGL ES through ANGLE on Direct3D.

mod angle_d3d_texture_feature;
mod angle_egl_interface;
mod angle_external_d3d11_texture2_d;
mod angle_external_objects_feature;
mod angle_win32_egl_display;
mod angle_win32_platform_graphics_factory;
mod d3d11_angle_win32_platform_graphics;
mod d3d9_angle_win32_platform_graphics;
mod swap_chain_gl_surface;

pub use angle_egl_interface::{is_angle_built_in, Win32AngleEglInterface};
pub use angle_win32_egl_display::{AngleWin32EglDisplay, D3D11Adapter};
pub use angle_win32_platform_graphics_factory::{AnglePlatformGraphics, AngleWin32PlatformGraphicsFactory};
#[allow(unused_imports)] // Read by the render target of the DXGI swap chain mode, which is Windows only.
pub(crate) use d3d11_angle_win32_platform_graphics::AngleContextDisplay;
pub use d3d11_angle_win32_platform_graphics::D3D11AngleWin32PlatformGraphics;
pub use d3d9_angle_win32_platform_graphics::D3D9AngleWin32PlatformGraphics;
