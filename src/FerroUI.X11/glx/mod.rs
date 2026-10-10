//! Rendering with OpenGL through GLX (the port of the `Glx` directory).

#[allow(clippy::module_inception)]
pub mod glx;
pub mod glx_consts;
pub mod glx_context;
pub mod glx_display;
pub mod glx_gl_platform_surface;
pub mod glx_platform_feature;

pub use glx::GlxInterface;
pub use glx_context::GlxContext;
pub use glx_display::GlxDisplay;
pub use glx_gl_platform_surface::GlxGlPlatformSurface;
pub use glx_platform_feature::GlxPlatformGraphics;
