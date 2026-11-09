//! Image and scene brushes share the tile layout calculator with the other backends.
//! Content and intermediate tiles remain GPU textures, including imported media.
use super::*;
use ferroui_base::{RelativeUnit, Size};
impl DrawingContext {
    pub(super) fn brush_transform<B: IBrush + ?Sized>(brush: &B, bounds: Rect) -> Matrix {
        let mut paint = Matrix::IDENTITY;
        if let Some(relative) = brush
            .relative_transform()
            .filter(|_| bounds.width > 0. && bounds.height > 0.)
        {
            paint = Matrix::create_translation(-bounds.x, -bounds.y)
                * Matrix::create_scale(1. / bounds.width, 1. / bounds.height)
                * relative.value()
                * Matrix::create_scale(bounds.width, bounds.height)
                * Matrix::create_translation(bounds.x, bounds.y);
        }
        if let Some(absolute) = brush.transform() {
            let origin = brush.transform_origin().to_pixels_rect(bounds);
            paint *= Matrix::create_translation(-origin.x, -origin.y)
                * absolute.value()
                * Matrix::create_translation(origin.x, origin.y);
        }
        paint
    }
    fn raster_bitmap(&self, size: Size) -> Option<Bitmap> {
        if !size.width.is_finite()
            || !size.height.is_finite()
            || size.width <= 0.
            || size.height <= 0.
        {
            return None;
        }
        let m = self.transform;
        let sx = (m.m11 * self.scale.x).hypot(m.m12 * self.scale.y).max(1e-3);
        let sy = (m.m21 * self.scale.x).hypot(m.m22 * self.scale.y).max(1e-3);
        let w = (size.width * sx).ceil().clamp(1., 16384.) as i32;
        let h = (size.height * sy).ceil().clamp(1., 16384.) as i32;
        Some(Bitmap::new(
            PixelSize::new(w, h),
            Vector::new(w as f64 / size.width * 96., h as f64 / size.height * 96.),
        ))
    }
    pub(super) fn tile_brush(
        &mut self,
        brush: &dyn ITileBrush,
        source: &dyn IBitmapImpl,
        bounds: Rect,
    ) {
        let source_size = source.pixel_size().to_size_with_dpi_vector(source.dpi());
        let calc = ferroui_base::rendering::utilities::TileBrushCalculator::from_brush(
            brush,
            source_size,
            bounds.size(),
        );
        let logical = calc.intermediate_size();
        let Some(tile) = self.raster_bitmap(logical) else {
            return;
        };
        let mut draw = tile.drawing(true, None);
        if let Some(options) = self.options.last() {
            draw.push_render_options(*options);
        }
        draw.push_clip(calc.intermediate_clip());
        draw.set_transform(calc.intermediate_transform());
        let pixels = source.pixel_size();
        draw.draw_bitmap(
            source,
            1.,
            Rect::new(0., 0., pixels.width as f64, pixels.height as f64),
            Rect::from_size(source_size),
        );
        draw.pop_clip();
        draw.dispose();
        let dest = calc.destination_rect();
        let tiled = brush.tile_mode() != TileMode::None;
        let offset = if brush.destination_rect().unit == RelativeUnit::Relative {
            bounds.top_left()
        } else {
            Point::default()
        };
        let rect = if tiled {
            Rect::new(
                offset.x + dest.x,
                offset.y + dest.y,
                dest.width,
                dest.height,
            )
        } else {
            Rect::new(offset.x, offset.y, logical.width, logical.height)
        };
        if rect.width <= 0. || rect.height <= 0. {
            return;
        }
        let paint = Self::brush_transform(brush, bounds) * self.transform;
        if paint.try_invert().is_none() {
            return;
        }
        let tile = tile.retained_surface();
        self.bitmap.context(|c| unsafe {
            ffi::q_set_transform(
                c,
                paint.m11,
                paint.m12,
                paint.m21,
                paint.m22,
                paint.m31,
                paint.m32,
                self.scale.x,
                self.scale.y,
            );
            ffi::qm_draw_tile(
                c,
                tile.0,
                rect.x,
                rect.y,
                rect.width,
                rect.height,
                brush.opacity(),
                brush.tile_mode() as i32,
            );
        });
    }
    pub(super) fn scene_brush(&mut self, scene: &dyn ISceneBrush, bounds: Rect) {
        if let Some(content) = scene.create_content() {
            let rect = content.rect();
            if let Some(bitmap) = self.raster_bitmap(rect.size()) {
                let mut draw = bitmap.drawing(true, None);
                if let Some(options) = self.options.last() {
                    draw.push_render_options(*options);
                }
                content.render(
                    &mut draw,
                    Some(Matrix::create_translation(-rect.x, -rect.y)),
                );
                draw.dispose();
                self.tile_brush(&*content.brush(), &bitmap, bounds);
            }
            content.dispose();
        }
    }
}
