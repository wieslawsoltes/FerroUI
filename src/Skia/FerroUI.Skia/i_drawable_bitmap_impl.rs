use crate::gpu::ISkiaGrContext;
use crate::immutable_bitmap::ImmutableBitmap;
use crate::render_target_bitmap_impl::RenderTargetBitmapImpl;
use crate::surface_render_target::SurfaceRenderTarget;
use crate::writeable_bitmap_impl::WriteableBitmapImpl;
use ferroui_base::platform::IBitmapImpl;
use skia_safe::{Canvas, Paint, Rect, SamplingOptions};

/// Extended bitmap implementation that allows for drawing its contents.
pub trait IDrawableBitmapImpl: IBitmapImpl {
    /// Draws the bitmap onto a canvas.
    ///
    /// `gr_context` is the GPU context the canvas draws with, if any: a
    /// bitmap that draws an image draws the form of it the context can draw
    /// (see [`ISkiaGrContext::drawable_image`]). `source_rect` is the part of
    /// the bitmap to draw and `dest_rect` where to draw it.
    fn draw(
        &self,
        gr_context: Option<&dyn ISkiaGrContext>,
        canvas: &Canvas,
        source_rect: &Rect,
        dest_rect: &Rect,
        sampling_options: SamplingOptions,
        paint: &Paint,
    );
}

/// Recovers the drawable bitmap behind a platform bitmap.
///
/// Returns `None` for bitmaps created by another backend.
pub fn try_get_drawable_bitmap(bitmap: &dyn IBitmapImpl) -> Option<&dyn IDrawableBitmapImpl> {
    let any = bitmap.as_any();
    if let Some(bitmap) = any.downcast_ref::<crate::leased_gpu_bitmap::LeasedGpuBitmap>() {
        return Some(bitmap);
    }

    if let Some(bitmap) = any.downcast_ref::<ImmutableBitmap>() {
        return Some(bitmap);
    }
    if let Some(bitmap) = any.downcast_ref::<WriteableBitmapImpl>() {
        return Some(bitmap);
    }
    if let Some(bitmap) = any.downcast_ref::<SurfaceRenderTarget>() {
        return Some(bitmap);
    }
    if let Some(bitmap) = any.downcast_ref::<RenderTargetBitmapImpl>() {
        return Some(bitmap);
    }

    None
}

/// Whether a Skia bitmap can be drawn by a CPU export recording without a GPU
/// context. `None` means the bitmap belongs to another backend. This performs
/// no GPU readback or transfer: callers must obtain an explicit CPU snapshot
/// before exporting a GPU-only frame or texture-backed surface.
///
/// Being a drawable Skia bitmap is not sufficient: a leased GPU frame silently
/// skips drawing without its owning GPU context, and GPU images cannot be
/// serialized by a CPU SVG or raster canvas.
pub fn is_bitmap_cpu_readable(bitmap: &dyn IBitmapImpl) -> Option<bool> {
    let any = bitmap.as_any();
    if any.is::<crate::leased_gpu_bitmap::LeasedGpuBitmap>() {
        return Some(false);
    }
    // Do not use is_valid(None): with the pinned Skia recorder API a null
    // recorder rejects ordinary raster images too. Encoded/lazy CPU images
    // remain valid export inputs even before their pixels are decoded.
    let readable = |image: skia_safe::Image| !image.is_texture_backed();
    if let Some(bitmap) = any.downcast_ref::<ImmutableBitmap>() {
        return Some(readable(bitmap.image()));
    }
    if let Some(bitmap) = any.downcast_ref::<SurfaceRenderTarget>() {
        // A surface with context affinity must not even be snapshotted from
        // an export recording that does not own its GPU context.
        return Some(!bitmap.has_render_context_affinity() && readable(bitmap.snapshot_image()));
    }
    if any.is::<WriteableBitmapImpl>() || any.is::<RenderTargetBitmapImpl>() {
        return Some(true);
    }
    None
}

#[cfg(test)]
#[path = "bitmap_cpu_export_tests.rs"]
mod bitmap_cpu_export_tests;
