use ferroui_base::platform::surfaces::{IPlatformRenderSurface, IPlatformRenderSurfaceRenderTarget};
use ferroui_base::platform::IPlatformGraphicsContext;
use ferroui_base::PixelSize;
use std::any::TypeId;
use std::ffi::c_void;
use std::rc::Rc;

/// A Metal device and the command queue used to render with it.
///
/// A graphics context that is a Metal device announces it through its
/// optional features: `try_get_feature(TypeId::of::<dyn IMetalDevice>())`
/// returns an `Rc<dyn IMetalDevice>`.
pub trait IMetalDevice: IPlatformGraphicsContext {
    /// The `id<MTLDevice>`.
    fn device(&self) -> *mut c_void;

    /// The `id<MTLCommandQueue>`.
    fn command_queue(&self) -> *mut c_void;
}

/// A surface that is rendered to with Metal.
///
/// A render surface announces that it is a Metal surface through
/// `try_get_surface_kind(TypeId::of::<dyn IMetalPlatformSurface>())`, which
/// returns an `Rc<dyn IMetalPlatformSurface>`; see
/// [`try_get_metal_surface`].
pub trait IMetalPlatformSurface: IPlatformRenderSurface {
    /// Creates the Metal render target of the surface for `device`.
    fn create_metal_render_target(&self, device: Rc<dyn IMetalDevice>) -> Rc<dyn IMetalPlatformSurfaceRenderTarget>;
}

/// The render surface viewed as a Metal surface, when it is one.
pub fn try_get_metal_surface(surface: &dyn IPlatformRenderSurface) -> Option<Rc<dyn IMetalPlatformSurface>> {
    surface
        .try_get_surface_kind(TypeId::of::<dyn IMetalPlatformSurface>())?
        .downcast_ref::<Rc<dyn IMetalPlatformSurface>>()
        .cloned()
}

/// The render target of a Metal surface.
pub trait IMetalPlatformSurfaceRenderTarget: IPlatformRenderSurfaceRenderTarget {
    /// Starts rendering a frame.
    fn begin_rendering(&self) -> Rc<dyn IMetalPlatformSurfaceRenderingSession>;

    /// Releases the render target.
    fn dispose(&self);
}

/// One frame being rendered to a Metal surface. Disposing the session
/// presents the frame.
pub trait IMetalPlatformSurfaceRenderingSession {
    /// The `id<MTLTexture>` to render to.
    fn texture(&self) -> *mut c_void;

    /// The size of the texture in device pixels.
    fn size(&self) -> PixelSize;

    /// The scaling from logical units to device pixels.
    fn scaling(&self) -> f64;

    /// Whether the texture's origin is its bottom-left corner.
    fn is_y_flipped(&self) -> bool;

    /// Finishes and presents the frame.
    fn dispose(&self);
}
