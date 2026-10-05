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
    /// `source_rect` is the part of the bitmap to draw and `dest_rect` where
    /// to draw it.
    fn draw(
        &self,
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
