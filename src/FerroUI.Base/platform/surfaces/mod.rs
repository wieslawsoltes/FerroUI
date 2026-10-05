//! Platform render surfaces: what a top-level hands to the render backend
//! to draw into.

mod i_framebuffer_platform_surface;
mod i_platform_render_surface;

pub use i_framebuffer_platform_surface::{
    FramebufferLockProperties, FuncFramebufferRenderTarget, IFramebufferPlatformSurface, IFramebufferRenderTarget,
};
pub use i_platform_render_surface::{IPlatformRenderSurface, IPlatformRenderSurfaceRenderTarget};
