//! Brushes that paint a scene: visual brushes and drawing brushes.
//!
//! The content of such a brush is replayed into an intermediate and the
//! shape is painted with the picture of it, as the Skia backend does it in
//! two ways:
//!
//! - content that is not rasterized at the resolution it is shown at is
//!   drawn into a surface of its own size, which is then the image of a
//!   tile brush;
//! - content with scalable rasterization is, in the Skia backend, recorded
//!   as a picture of one tile, and the shader of the picture replays it into
//!   a tile at the resolution of the canvas. Here the content is replayed
//!   into a surface of one tile at the resolution of the target (the scale
//!   of the transform of the context and of the brush): the same pixels,
//!   without a recorded picture in between. The picture render target of the
//!   Skia backend, which only its drawing context uses, has therefore no
//!   counterpart.

use super::{DrawingContextImpl, PaintWrapper};
use crate::scene::VelloScenePaint;
use crate::vello_extensions::{rect_path, to_affine};
use ferroui_base::media::{Colors, ISceneBrushContent, MediaExtensions, StretchDirection, TileMode};
use ferroui_base::rendering::utilities::TileBrushCalculator;
use ferroui_base::{Matrix, PixelSize, Point, Rect};
use peniko::ImageQuality;

/// The most pixels a tile of scalable content is replayed into, as the
/// picture shader of Skia limits its tile: beyond it the tile is replayed
/// at a lower resolution and enlarged.
const MAX_TILE_AREA: f64 = 2048.0 * 2048.0;

impl DrawingContextImpl {
    /// Configures the paint wrapper for painting with the content of a
    /// scene brush. `opacity` is the opacity the brush is drawn with.
    pub(super) fn configure_scene_brush_content(
        &mut self,
        paint_wrapper: &mut PaintWrapper,
        content: &dyn ISceneBrushContent,
        target_rect: Rect,
        opacity: f32,
    ) {
        if content.use_scalable_rasterization() {
            self.configure_scene_brush_content_with_tile(paint_wrapper, content, target_rect, opacity);
        } else {
            self.configure_scene_brush_content_with_surface(paint_wrapper, content, target_rect, opacity);
        }
    }

    fn configure_scene_brush_content_with_surface(
        &mut self,
        paint_wrapper: &mut PaintWrapper,
        content: &dyn ISceneBrushContent,
        target_rect: Rect,
        opacity: f32,
    ) {
        let rect = content.rect();
        let intermediate_size = rect.size();

        if intermediate_size.width >= 1.0 && intermediate_size.height >= 1.0 {
            crate::perf::count(crate::perf::Phase::BrushSurface, 0);
            let intermediate = self.create_render_target(
                PixelSize::from_size_with_dpi_vector(intermediate_size, self.intermediate_surface_dpi),
                true,
            );

            {
                let mut ctx = intermediate.create_drawing_context();
                ctx.push_render_options(self.render_options);
                ctx.clear(Colors::TRANSPARENT);
                let transform = if rect.top_left() == Point::default() {
                    None
                } else {
                    Some(Matrix::create_translation(-rect.x, -rect.y))
                };
                content.render(&mut *ctx, transform);
                ctx.pop_render_options();
                ctx.dispose();
            }

            self.configure_tile_brush(paint_wrapper, target_rect, &*content.brush(), intermediate.bitmap(), opacity);
            intermediate.dispose();
        }
    }

    fn configure_scene_brush_content_with_tile(
        &mut self,
        paint_wrapper: &mut PaintWrapper,
        content: &dyn ISceneBrushContent,
        target_rect: Rect,
        opacity: f32,
    ) {
        // Brushes ignore whatever layout bounds visuals have and use the
        // content bounds instead: the source rect (view box) is relative to
        // the content bounds, not to the visual or drawing.
        let brush = content.brush();
        let content_rect = content.rect();
        let source_rect = brush.source_rect().to_pixels_rect(content_rect);

        // Early escape: the brush paints nothing.
        if content_rect.width <= 0.0
            || content_rect.height <= 0.0
            || source_rect.width <= 0.0
            || source_rect.height <= 0.0
        {
            return;
        }

        // We are moving the render area to make the top-left corner of the
        // source rect (view box) to be at (0,0) of the tile.
        let mut content_render_transform = Matrix::create_translation(-source_rect.x, -source_rect.y);

        // The destination rect (viewport) is specified relative to the
        // target rect.
        let destination_rect = brush.destination_rect().to_pixels_rect(target_rect);

        // The tile size matches the destination rect size.
        let tile_size = destination_rect.size();
        if !(tile_size.width > 0.0 && tile_size.height > 0.0) {
            return;
        }

        // Apply transforms to stretch the content to match the tile.
        if source_rect.size() != tile_size {
            // Stretch the content rect to match the tile size.
            let scale = MediaExtensions::calculate_scaling(
                brush.stretch(),
                tile_size,
                source_rect.size(),
                StretchDirection::Both,
            );

            // And move the resulting rect according to the alignment rules.
            let alignment_translate = TileBrushCalculator::calculate_translate_sizes(
                brush.alignment_x(),
                brush.alignment_y(),
                source_rect.size() * scale,
                tile_size,
            );

            content_render_transform = content_render_transform
                * Matrix::create_scale_vector(scale)
                * Matrix::create_translation_vector(alignment_translate);
        }

        // If there is no brush transform and the destination rect is at
        // (0,0) we don't need any transforms.
        let mut shader_transform = Matrix::IDENTITY;

        // Apply the destination rect position.
        if destination_rect.position() != Point::default() {
            shader_transform = Matrix::create_translation(destination_rect.x, destination_rect.y);
        }

        // Apply the relative and the absolute brush transform, in that
        // order.
        if let Some(relative_transform) = Self::get_relative_transform(content, target_rect) {
            shader_transform *= relative_transform;
        }

        if let Some(transform) = Self::get_absolute_transform(content, target_rect) {
            shader_transform *= transform;
        }

        // The pixels of the target a unit of the tile covers, along each of
        // its axes.
        let mut on_target = shader_transform * self.current_transform;
        if let Some(post_transform) = self.post_transform {
            on_target *= post_transform;
        }
        let mut scale_x = on_target.m11.hypot(on_target.m12);
        let mut scale_y = on_target.m21.hypot(on_target.m22);
        if !(scale_x.is_finite() && scale_y.is_finite() && scale_x > 0.0 && scale_y > 0.0) {
            return;
        }
        let area = tile_size.width * scale_x * tile_size.height * scale_y;
        if area > MAX_TILE_AREA {
            let shrink = (MAX_TILE_AREA / area).sqrt();
            scale_x *= shrink;
            scale_y *= shrink;
        }

        // Replay the content into one tile at that resolution.
        let pixel_size = PixelSize::new(
            (tile_size.width * scale_x).ceil().clamp(1.0, u16::MAX as f64) as i32,
            (tile_size.height * scale_y).ceil().clamp(1.0, u16::MAX as f64) as i32,
        );
        let pixels_per_unit_x = pixel_size.width as f64 / tile_size.width;
        let pixels_per_unit_y = pixel_size.height as f64 / tile_size.height;

        crate::perf::count(crate::perf::Phase::BrushSurface, 0);
        let intermediate = self.create_render_target(pixel_size, false);
        {
            let mut ctx = intermediate.create_drawing_context();
            ctx.push_render_options(self.render_options);
            content.render(
                &mut *ctx,
                Some(content_render_transform * Matrix::create_scale(pixels_per_unit_x, pixels_per_unit_y)),
            );
            ctx.pop_render_options();
            ctx.dispose();
        }

        // The picture shader of the Skia backend samples the nearest pixel
        // of its tile.
        let tile_mode = brush.tile_mode();
        let (x_extend, y_extend) = Self::get_tile_modes(tile_mode);
        let tile_brush = intermediate.brush(self, (x_extend, y_extend), ImageQuality::Low, opacity);
        intermediate.dispose();
        let Some((tile_brush, tile_pixels)) = tile_brush else {
            return;
        };

        // From the pixels of the tile to the space of the shape.
        let paint_transform = to_affine(
            Matrix::create_scale(1.0 / pixels_per_unit_x, 1.0 / pixels_per_unit_y) * shader_transform,
        );

        if tile_mode == TileMode::None {
            // A tile that is not repeated paints nothing beside itself: the
            // shape is clipped to the tile.
            let mut tile = rect_path(Rect::new(0.0, 0.0, tile_pixels.width as f64, tile_pixels.height as f64));
            tile.apply_affine(paint_transform);
            paint_wrapper.clip = Some(tile);
        }

        paint_wrapper.paint = Some(VelloScenePaint { brush: tile_brush, transform: paint_transform });
    }
}
