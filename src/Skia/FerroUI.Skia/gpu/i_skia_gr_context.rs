use skia_safe::{Image, ImageInfo, Surface, SurfaceProps};
use std::any::Any;

/// The Skia GPU backend a context runs on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SkiaGpuBackend {
    /// Skia's Graphite backend.
    Graphite,
    /// Skia's Ganesh backend.
    Ganesh,
}

/// Skia's GPU context, independent of the Skia GPU backend in use.
///
/// The backend's drawing code needs only a handful of operations from the
/// GPU context: creating offscreen surfaces, flushing recorded work and
/// reading surfaces back. A Graphite implementation backs them with a
/// context and its recorder, a Ganesh one with a direct context.
pub trait ISkiaGrContext {
    /// The Skia GPU backend of the context.
    fn backend(&self) -> SkiaGpuBackend;

    /// Creates an offscreen GPU surface.
    fn create_surface(&self, image_info: &ImageInfo, surface_props: &SurfaceProps) -> Option<Surface>;

    /// The largest render target the context can create, when known.
    fn max_render_target_size(&self) -> Option<i32>;

    /// Sends all pending drawing to the GPU.
    fn flush(&self);

    /// Tells Skia that the state of the underlying graphics API was changed
    /// behind its back.
    fn reset_context(&self);

    /// Limits the bytes of GPU memory the context caches.
    fn set_resource_cache_limit(&self, max_resource_bytes: i64);

    /// Copies the contents of a GPU surface of this context into a raster
    /// image.
    fn snapshot_to_raster(&self, surface: &mut Surface) -> Option<Image>;

    /// Whether the GPU device behind the context was lost.
    fn is_lost(&self) -> bool;

    /// Lets a GPU implementation recover its concrete context type.
    fn as_any(&self) -> &dyn Any;
}
