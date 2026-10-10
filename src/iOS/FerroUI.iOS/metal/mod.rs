//! Metal: the platform graphics, the device, and the render surface of a
//! view over its Metal layer with its render target and drawing session.

mod metal_device;
mod metal_drawing_session;
mod metal_platform_graphics;
mod metal_platform_surface;
mod metal_render_target;

pub use metal_device::MetalDevice;
pub use metal_drawing_session::{FrameCapture, MetalDrawingSession};
pub use metal_platform_graphics::MetalPlatformGraphics;
pub use metal_platform_surface::MetalPlatformSurface;
pub(crate) use metal_platform_surface::{SharedLayer, SurfaceShared};
pub use metal_render_target::MetalRenderTarget;
