use skia_safe::{Image, ImageInfo, MipmapMode, SamplingOptions, Surface, SurfaceProps};
use std::any::Any;
use std::rc::Rc;

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
    /// Direct imported images are bound to one context generation.
    fn frame_context_id(&self) -> Option<u64> {
        None
    }

    /// Retains producer storage until the commands about to be recorded finish.
    fn retain_frame(
        &self,
        _frame: ferroui_base::rendering::composition::frames::FrameLease,
    ) -> Result<(), ferroui_base::rendering::composition::frames::FrameError> {
        Err(ferroui_base::rendering::composition::frames::FrameError::MissingImporter)
    }

    /// The Skia GPU backend of the context.
    fn backend(&self) -> SkiaGpuBackend;

    /// Creates an offscreen GPU surface.
    fn create_surface(
        &self,
        image_info: &ImageInfo,
        surface_props: &SurfaceProps,
    ) -> Option<Surface>;

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

    /// The form of `image` that canvases of this context draw.
    ///
    /// `mipmapped` tells that the image is going to be sampled with
    /// mipmaps (see [`needs_mipmaps`]).
    ///
    /// A context that uploads raster images by itself when they are drawn
    /// (Ganesh) returns the image as it is, which is the default. A context
    /// that draws only images living on the GPU (Graphite) returns a
    /// texture-backed image of the same contents, uploading it the first
    /// time and reusing the texture while the image is alive. When the
    /// image cannot be uploaded it is returned as it is.
    fn drawable_image(&self, image: &Image, mipmapped: bool) -> Image {
        let _ = mipmapped;
        image.clone()
    }

    /// Whether the GPU device behind the context was lost.
    fn is_lost(&self) -> bool;

    /// Begins a prewarm of GPU pipelines off the frames (#437): a context
    /// whose surfaces record into a recorder of their own, and what compiles
    /// the pipelines of what was drawn on them away from the thread that
    /// renders. `None`, the default, when the context has no such recorder:
    /// a prewarm then draws with the context of the frames, whose flush
    /// compiles the programs on the render thread (Ganesh).
    fn begin_prewarm(&self) -> Option<SkiaPrewarm> {
        None
    }

    /// Lets a GPU implementation recover its concrete context type.
    fn as_any(&self) -> &dyn Any;
}

/// A prewarm of GPU pipelines on a recorder of its own; see
/// [`ISkiaGrContext::begin_prewarm`].
pub struct SkiaPrewarm {
    /// The context to draw the prewarm with: its surfaces record into the
    /// recorder of the prewarm, and it uploads the images drawn to it.
    pub gr_context: Rc<dyn ISkiaGrContext>,
    /// Compiles the pipelines of what was drawn, once the surfaces of the
    /// prewarm are released; the context records nothing afterwards.
    pub finish: Box<dyn FnOnce()>,
}

/// The form of `image` that a canvas drawing with `gr_context` draws; see
/// [`ISkiaGrContext::drawable_image`]. Without a GPU context the canvas is
/// a raster one and draws the image as it is.
pub fn drawable_image(
    gr_context: Option<&dyn ISkiaGrContext>,
    image: Image,
    mipmapped: bool,
) -> Image {
    match gr_context {
        Some(gr_context) => gr_context.drawable_image(&image, mipmapped),
        None => image,
    }
}

/// Whether sampling an image with `sampling_options` reads its mipmaps.
pub fn needs_mipmaps(sampling_options: &SamplingOptions) -> bool {
    !sampling_options.use_cubic && sampling_options.mipmap != MipmapMode::None
}
pub(crate) fn next_frame_context_id() -> u64 {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    NEXT.fetch_update(
        std::sync::atomic::Ordering::Relaxed,
        std::sync::atomic::Ordering::Relaxed,
        |id| id.checked_add(1),
    )
    .expect("frame context identity exhausted")
}
