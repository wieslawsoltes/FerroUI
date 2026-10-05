//! The surfaces an OpenGL context renders to.

mod i_gl_platform_surface;
mod i_gl_platform_surface_render_target;
mod i_gl_platform_surface_rendering_session;

pub use i_gl_platform_surface::{try_get_gl_surface, IGlPlatformSurface};
pub use i_gl_platform_surface_render_target::IGlPlatformSurfaceRenderTarget;
pub use i_gl_platform_surface_rendering_session::IGlPlatformSurfaceRenderingSession;
