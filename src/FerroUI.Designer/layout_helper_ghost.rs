//! Optional, bounded native snapshots for source-preserving drag previews.
use ferroui_base::media::DrawingContext;
use ferroui_base::media::imaging::RenderTargetBitmap;
use ferroui_base::platform::IPlatformRenderInterface;
use ferroui_base::{FerroLocator, LocatorExtensions, Matrix, PixelSize, Rect, Ref, Vector, Visual};

/// A native drag snapshot. Capture after the move threshold and drop it when the
/// gesture finishes or cancels; no accepted control property is changed.
///
/// Ancestor affine transforms are preserved. A source's own render transform or
/// mirror is currently declined because the native bitmap renderer applies it
/// inside its local clipping rectangle. Those controls use the geometric ghost.
pub struct DragVisualGhost {
    bitmap: RenderTargetBitmap,
    bounds: Rect,
    to_artboard: Matrix,
    pixels: usize,
}
impl DragVisualGhost {
    /// Capture a laid-out control once. The longest bitmap edge is bounded to
    /// 2048 pixels, retaining aspect ratio. Hosts should also bound the sum of
    /// `pixel_count()` across a large multiselection.
    pub fn capture(visual: &Ref<Visual>, to_artboard: Matrix, raster_scale: f64) -> Option<Self> {
        if !raster_scale.is_finite()
            || raster_scale <= 0.0
            || visual.render_transform().is_some()
            || visual.has_mirror_transform()
            || FerroLocator::current()
                .get_service::<dyn IPlatformRenderInterface>()
                .is_none()
        {
            return None;
        }
        crate::affine_manipulation::affine(to_artboard)?;
        crate::affine_manipulation::inverse(to_artboard)?;
        let bounds = Rect::from_size(visual.bounds().size());
        if !super::valid_rect(bounds) || bounds.width <= 0.0 || bounds.height <= 0.0 {
            return None;
        }
        let scale = raster_scale
            .min(2.0)
            .min(2048.0 / bounds.width.max(bounds.height));
        if !scale.is_finite() || scale <= 0.0 {
            return None;
        }
        let width = (bounds.width * scale).ceil().clamp(1.0, 2048.0) as i32;
        let height = (bounds.height * scale).ceil().clamp(1.0, 2048.0) as i32;
        let bitmap = RenderTargetBitmap::with_dpi(
            PixelSize::new(width, height),
            Vector::new(96.0 * scale, 96.0 * scale),
        );
        bitmap.render(visual);
        Some(Self {
            bitmap,
            bounds,
            to_artboard,
            pixels: width as usize * height as usize,
        })
    }

    /// Number of cached RGBA pixels, for a host-owned aggregate gesture budget.
    pub fn pixel_count(&self) -> usize {
        self.pixels
    }

    /// Render with an artboard-space adjustment, such as the current translation.
    /// The cached content does not refresh or mutate runtime/source while dragging.
    pub fn render(&self, context: &mut DrawingContext, adjustment: Matrix, opacity: f64) {
        if !opacity.is_finite()
            || opacity <= 0.0
            || crate::affine_manipulation::affine(adjustment).is_none()
        {
            return;
        }
        let transform = context.push_transform(self.to_artboard * adjustment);
        let opacity = context.push_opacity(opacity.min(1.0));
        context.draw_image(&self.bitmap, self.bounds);
        context.pop(opacity);
        context.pop(transform);
    }
}
impl Drop for DragVisualGhost {
    fn drop(&mut self) {
        self.bitmap.dispose();
    }
}
