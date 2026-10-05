use super::IPlatformHandle;
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::PixelSize;

/// A native handle that render backends can create a render target for.
pub trait INativePlatformHandleSurface: IPlatformHandle + IPlatformRenderSurface {
    /// The size of the surface in device pixels.
    fn size(&self) -> PixelSize;

    /// The scaling of the surface.
    fn scaling(&self) -> f64;
}
