use crate::immutable_bitmap::ImmutableBitmap;
use crate::render_target_bitmap_impl::RenderTargetBitmapImpl;
use crate::surface_render_target::SurfaceRenderTarget;
use crate::writeable_bitmap_impl::WriteableBitmapImpl;
use ferroui_base::platform::IBitmapImpl;
use peniko::ImageData;

/// Extended bitmap implementation that allows for drawing its contents.
///
/// A bitmap of the Skia backend draws itself onto a canvas; the scene of a
/// frame of this backend paints with images, so a bitmap hands out its
/// pixels as one.
pub trait IDrawableBitmapImpl: IBitmapImpl {
    /// The current pixels of the bitmap as an image of premultiplied RGBA
    /// pixels, or `None` once the bitmap is disposed. The image is kept
    /// until the pixels change: drawing a bitmap again copies nothing.
    fn image(&self) -> Option<ImageData>;
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
