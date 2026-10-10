//! OpenGL on Windows: OpenGL ES through ANGLE (`angle`), and the OpenGL of
//! the system through WGL.

pub mod angle;
pub mod wgl_consts;

#[cfg(windows)]
mod wgl_context;
mod wgl_display;
#[cfg(windows)]
mod wgl_gdi_resource_manager;
#[cfg(windows)]
mod wgl_gl_platform_surface;
mod wgl_platform_open_gl_interface;
mod wgl_restore_context;

#[cfg(windows)]
pub use wgl_context::WglContext;
#[cfg(windows)]
pub use wgl_gl_platform_surface::WglGlPlatformSurface;
pub use wgl_platform_open_gl_interface::WglPlatformOpenGlInterface;
