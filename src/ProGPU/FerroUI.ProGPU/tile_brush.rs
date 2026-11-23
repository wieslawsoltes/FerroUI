//! Image brushes and scene brushes (visual and drawing brushes), on the GPU.
//!
//! The tile is rendered into a layer of the device. The shape is then a clip of the state, and
//! one image draw covers it: the sampler of the image repeats or mirrors the tile, so the number
//! of tiles costs nothing. Strokes and text arrive here as the filled outlines they are.
//!
//! The geometry of the tile (the stretch, the alignment, the source and destination rectangles and
//! the two brush transforms) comes from `TileBrushCalculator` and the members of the brush.

use super::*;
use crate::layer::ProGpuLayer;
use ferroui_base::media::{Colors, ISceneBrushContent, ITileBrush, MediaExtensions, StretchDirection, TileMode};
use ferroui_base::rendering::utilities::TileBrushCalculator;
use ferroui_base::RelativeUnit;

/// The longest side a tile of scalable content is replayed at. A longer tile is replayed smaller,
/// in the same proportions, and enlarged by the sampler.
const MAX_SCALABLE_TILE_SIDE: f64 = 4096.0;

/// The largest side of the layer of a tile.
const MAX_TILE_SIDE: i32 = 8192;

/// Whether the brush paints with a tile.
pub(super) fn is_tile_brush(brush: &dyn IBrush) -> bool {
    brush.as_scene_brush().is_some() || brush.as_image_brush().is_some()
}

/// A tile ready to paint with.
struct TilePaint {
    layer: ProGpuLayer,
    width: u32,
    height: u32,
    /// From the pixels of the tile to the space of the shape.
    to_shape: Matrix,
    mode: TileMode,
    opacity: f32,
}

/// An opacity mask of a tile brush, waiting for the content of its group. Without a tile the
/// brush paints nothing, so the mask keeps nothing.
pub(super) struct TileMask {
    paint: Option<TilePaint>,
    bounds: Rect,
    transform: Matrix,
}

/// The relative transform of a brush, on the box of the shape as a unit square.
fn relative_transform(brush: &dyn IBrush, target: Rect) -> Option<Matrix> {
    let relative = brush.relative_transform()?;
    if target.width <= 0.0 || target.height <= 0.0 {
        return None;
    }
    let matrix = relative.value();
    if matrix.is_identity() {
        return None;
    }
    Some(
        Matrix::create_translation(-target.x, -target.y)
            * Matrix::create_scale(1.0 / target.width, 1.0 / target.height)
            * matrix
            * Matrix::create_scale(target.width, target.height)
            * Matrix::create_translation(target.x, target.y),
    )
}

/// The absolute transform of a brush around its transform origin.
fn absolute_transform(brush: &dyn IBrush, target: Rect) -> Option<Matrix> {
    let absolute = brush.transform()?;
    let origin = brush.transform_origin().to_pixels_rect(target);
    let offset = Matrix::create_translation(origin.x, origin.y);
    Some((-offset) * absolute.value() * offset)
}

fn with_brush_transforms(mut matrix: Matrix, brush: &dyn IBrush, target: Rect) -> Matrix {
    if let Some(relative) = relative_transform(brush, target) {
        matrix = matrix * relative;
    }
    if let Some(absolute) = absolute_transform(brush, target) {
        matrix = matrix * absolute;
    }
    matrix
}

/// The address mode of the sampler along an axis of a tile mode.
pub(super) fn address_modes(mode: TileMode) -> (u32, u32) {
    let along = |flipped: bool| {
        if mode == TileMode::None {
            PROGPU_NATIVE_IMAGE_ADDRESS_CLAMP
        } else if flipped {
            PROGPU_NATIVE_IMAGE_ADDRESS_MIRROR_REPEAT
        } else {
            PROGPU_NATIVE_IMAGE_ADDRESS_REPEAT
        }
    };
    (
        along(mode == TileMode::FlipX || mode == TileMode::FlipXY) as u32,
        along(mode == TileMode::FlipY || mode == TileMode::FlipXY) as u32,
    )
}

/// The start of a source range moved by whole periods of the mirrored tile to be at least zero:
/// the sampler repeats, and the engine asks for a source that does not start before the image.
pub(super) fn positive_start(start: f64, tile: f64) -> f64 {
    let period = tile * 2.0;
    start + (-start / period).ceil().max(0.0) * period
}

impl ProGpuDrawingContext {
    /// Fills the outline with a tile brush; `target` is the box of the shape the brush is
    /// placed on. Whether anything was drawn.
    pub(super) fn fill_with_tile_brush(&mut self, data: &PathData, brush: &dyn IBrush, target: Rect) -> bool {
        self.refuse_fragment();
        let bounds = data.bounds();
        if !has_area(bounds) || !invertible(&self.current.transform) {
            return false;
        }
        let Some(paint) = self.tile_paint(brush, target) else { return false };
        let mask = self.vector_mask(data, PROGPU_NATIVE_CLIP_INTERSECT as u32);
        self.push_mask_clip(mask);
        let drawn = self.draw_tiles(&paint, bounds);
        self.pop_clip();
        drawn
    }

    /// Opens a group whose content the tile brush masks when it closes: the tile is composed over
    /// the content with destination-in, which keeps the content where the tile has alpha.
    pub(super) fn push_tile_mask(&mut self, mask: &dyn IBrush, bounds: Rect) {
        self.refuse_fragment();
        let paint = self.tile_paint(mask, bounds);
        let target = transform_bounds(bounds, &self.current.transform);
        self.push_isolated_layer(target);
        self.layers.push(LayerEntry::TileMask(Box::new(TileMask { paint, bounds, transform: self.current.transform })));
    }

    /// Composes the tile of a mask over the content of its group, which is still open.
    pub(super) fn apply_tile_mask(&mut self, mask: TileMask) {
        let saved = self.current;
        self.current.transform = mask.transform;
        // The clips of the state apply to the group when it is composed, once.
        self.current.mask = NO_INDEX;
        self.current.clip_rect = None;
        self.changed();
        self.push_scene_layer(Some(mask.bounds), 1.0, PROGPU_NATIVE_BLEND_DST_IN as u32);
        if let Some(paint) = &mask.paint {
            self.draw_tiles(paint, mask.bounds);
        }
        self.pop_scene_layer();
        self.current = saved;
        self.changed();
    }

    /// The pixels of the target per logical unit along the axes of the transform.
    fn tile_scale(&self) -> (f64, f64) {
        let matrix = &self.current.transform;
        let along = |x: f64, y: f64| {
            let scale = x.hypot(y);
            if scale.is_finite() && scale > 0.0 { scale.clamp(0.25, 16.0) } else { 1.0 }
        };
        (along(matrix.m11, matrix.m12), along(matrix.m21, matrix.m22))
    }

    fn tile_layer(&self, size: PixelSize) -> Option<ProGpuLayer> {
        if size.width < 1 || size.height < 1 || self.gpu.is_none() {
            return None;
        }
        let size = PixelSize::new(size.width.min(MAX_TILE_SIDE), size.height.min(MAX_TILE_SIDE));
        let layer = ProGpuLayer::new(self.gpu.clone(), size, Vector::new(96.0, 96.0));
        layer.texture().is_some().then_some(layer)
    }

    fn tile_paint(&mut self, brush: &dyn IBrush, target: Rect) -> Option<TilePaint> {
        let opacity = brush.opacity().clamp(0.0, 1.0) as f32;
        if let Some(scene) = brush.as_scene_brush() {
            // A scene brush without content paints nothing.
            let content = scene.create_content()?;
            let paint = if content.use_scalable_rasterization() {
                self.scalable_tile(&*content, target, opacity)
            } else {
                self.surface_tile(&*content, target, opacity)
            };
            content.dispose();
            return paint;
        }
        let image_brush = brush.as_image_brush()?;
        // A tile brush without an image paints nothing.
        let bitmap = image_brush.source().and_then(|source| source.get_bitmap())?;
        self.image_tile(image_brush, &*bitmap, target, opacity)
    }

    /// The tile of an image: the image placed in one tile by the stretch and alignment.
    fn image_tile(
        &mut self,
        brush: &dyn ITileBrush,
        image: &dyn IBitmapImpl,
        target: Rect,
        opacity: f32,
    ) -> Option<TilePaint> {
        let (scale_x, scale_y) = self.tile_scale();
        let dpi = Vector::new(96.0 * scale_x, 96.0 * scale_y);
        let image_size = image.pixel_size();
        let calc = TileBrushCalculator::from_brush(brush, image_size.to_size_with_dpi_vector(dpi), target.size());
        let layer = self.tile_layer(PixelSize::from_size_with_dpi_vector(calc.intermediate_size(), dpi))?;
        let size = layer.pixel_size();
        {
            let scale = Matrix::create_scale(scale_x, scale_y);
            let mut context = layer.create_drawing_context();
            context.clear(Colors::TRANSPARENT);
            context.set_transform(scale);
            context.push_clip(calc.intermediate_clip());
            if let Some(options) = self.render_options.last() {
                context.push_render_options(*options);
            }
            context.set_transform(calc.intermediate_transform() * scale);
            context.draw_bitmap(
                image,
                1.0,
                Rect::from_size(image_size.to_size_with_dpi(96.0)),
                Rect::from_size(image_size.to_size_with_dpi_vector(dpi)),
            );
            if self.render_options.last().is_some() {
                context.pop_render_options();
            }
            context.pop_clip();
            context.dispose();
        }
        let mode = brush.tile_mode();
        let tiling = if mode != TileMode::None {
            Matrix::create_translation(-calc.destination_rect().x, -calc.destination_rect().y)
        } else {
            Matrix::IDENTITY
        };
        let mut to_shape = Matrix::create_scale(1.0 / scale_x, 1.0 / scale_y) * tiling;
        if brush.destination_rect().unit == RelativeUnit::Relative {
            to_shape = Matrix::create_translation(target.x, target.y) * to_shape;
        }
        let to_shape = with_brush_transforms(to_shape, brush, target);
        Some(TilePaint { width: size.width as u32, height: size.height as u32, layer, to_shape, mode, opacity })
    }

    /// Content that is not rasterized at the resolution it is shown at: a surface of its size,
    /// which is then the image of a tile.
    fn surface_tile(&mut self, content: &dyn ISceneBrushContent, target: Rect, opacity: f32) -> Option<TilePaint> {
        let rect = content.rect();
        if rect.width < 1.0 || rect.height < 1.0 {
            return None;
        }
        let (scale_x, scale_y) = self.tile_scale();
        let dpi = Vector::new(96.0 * scale_x, 96.0 * scale_y);
        let surface = self.tile_layer(PixelSize::from_size_with_dpi_vector(rect.size(), dpi))?;
        {
            let mut context = surface.create_drawing_context();
            if let Some(options) = self.render_options.last() {
                context.push_render_options(*options);
            }
            context.clear(Colors::TRANSPARENT);
            context.set_transform(Matrix::create_scale(scale_x, scale_y));
            let offset = (rect.top_left() != Point::default()).then(|| Matrix::create_translation(-rect.x, -rect.y));
            content.render(&mut *context, offset);
            if self.render_options.last().is_some() {
                context.pop_render_options();
            }
            context.dispose();
        }
        self.image_tile(&*content.brush(), &surface, target, opacity)
    }

    /// Content with scalable rasterization: replayed into one tile at the resolution of the target.
    fn scalable_tile(&mut self, content: &dyn ISceneBrushContent, target: Rect, opacity: f32) -> Option<TilePaint> {
        let brush = content.brush();
        let content_rect = content.rect();
        let source = brush.source_rect().to_pixels_rect(content_rect);
        if content_rect.width <= 0.0 || content_rect.height <= 0.0 || source.width <= 0.0 || source.height <= 0.0 {
            return None;
        }
        let mut content_transform = Matrix::create_translation(-source.x, -source.y);
        let destination = brush.destination_rect().to_pixels_rect(target);
        let tile = destination.size();
        if !(tile.width > 0.0 && tile.height > 0.0) {
            return None;
        }
        if source.size() != tile {
            let scale =
                MediaExtensions::calculate_scaling(brush.stretch(), tile, source.size(), StretchDirection::Both);
            let alignment = TileBrushCalculator::calculate_translate_sizes(
                brush.alignment_x(),
                brush.alignment_y(),
                source.size() * scale,
                tile,
            );
            content_transform = content_transform
                * Matrix::create_scale_vector(scale)
                * Matrix::create_translation_vector(alignment);
        }
        let mut placement = Matrix::IDENTITY;
        if destination.position() != Point::default() {
            placement = Matrix::create_translation(destination.x, destination.y);
        }
        let placement = with_brush_transforms(placement, content, target);

        let on_target = placement * self.current.transform;
        let (mut scale_x, mut scale_y) = (on_target.m11.hypot(on_target.m12), on_target.m21.hypot(on_target.m22));
        if !(scale_x.is_finite() && scale_y.is_finite() && scale_x > 0.0 && scale_y > 0.0) {
            return None;
        }
        let longest = (tile.width * scale_x).max(tile.height * scale_y);
        if longest > MAX_SCALABLE_TILE_SIDE {
            let fit = MAX_SCALABLE_TILE_SIDE / longest;
            scale_x *= fit;
            scale_y *= fit;
        }
        let pixels = PixelSize::new(
            (tile.width * scale_x).ceil().clamp(1.0, f64::from(MAX_TILE_SIDE)) as i32,
            (tile.height * scale_y).ceil().clamp(1.0, f64::from(MAX_TILE_SIDE)) as i32,
        );
        let (per_unit_x, per_unit_y) = (f64::from(pixels.width) / tile.width, f64::from(pixels.height) / tile.height);
        let layer = self.tile_layer(pixels)?;
        {
            let mut context = layer.create_drawing_context();
            if let Some(options) = self.render_options.last() {
                context.push_render_options(*options);
            }
            context.clear(Colors::TRANSPARENT);
            content.render(&mut *context, Some(content_transform * Matrix::create_scale(per_unit_x, per_unit_y)));
            if self.render_options.last().is_some() {
                context.pop_render_options();
            }
            context.dispose();
        }
        Some(TilePaint {
            layer,
            width: pixels.width as u32,
            height: pixels.height as u32,
            to_shape: Matrix::create_scale(1.0 / per_unit_x, 1.0 / per_unit_y) * placement,
            mode: brush.tile_mode(),
            opacity,
        })
    }

    /// One image draw over what the shape covers of the tiling.
    fn draw_tiles(&mut self, paint: &TilePaint, shape: Rect) -> bool {
        let Some(from_shape) = paint.to_shape.try_invert() else { return false };
        let Some(texture) = paint.layer.texture().cloned() else { return false };
        let Some(image) = self.sampled_texture(&texture, false) else { return false };
        let (tile_width, tile_height) = (f64::from(paint.width), f64::from(paint.height));
        let (address_u, address_v) = address_modes(paint.mode);
        let mut flags = PROGPU_NATIVE_SCENE_IMAGE_SOURCE_PREMULTIPLIED as u32;
        let (source, destination) = if paint.mode == TileMode::None {
            let tile = Rect::new(0.0, 0.0, tile_width, tile_height);
            (tile, tile)
        } else {
            flags |= (address_u << PROGPU_NATIVE_SCENE_IMAGE_ADDRESS_U_SHIFT)
                | (address_v << PROGPU_NATIVE_SCENE_IMAGE_ADDRESS_V_SHIFT)
                | PROGPU_NATIVE_SCENE_IMAGE_EXTENDED_SOURCE_RECT as u32;
            let covered = transform_bounds(shape, &from_shape);
            let covered = Rect::new(covered.x - 1.0, covered.y - 1.0, covered.width + 2.0, covered.height + 2.0);
            let source = Rect::new(
                positive_start(covered.x, tile_width),
                positive_start(covered.y, tile_height),
                covered.width,
                covered.height,
            );
            (source, covered)
        };
        let draw = progpu_native_scene_image_draw {
            struct_size: std::mem::size_of::<progpu_native_scene_image_draw>() as u32,
            flags,
            image_width: image.width,
            image_height: image.height,
            row_bytes: image.row_bytes,
            // As the other backends: the nearest pixel of the tile, which is at target resolution.
            sampling: PROGPU_NATIVE_IMAGE_SAMPLING_NEAREST as u32,
            source_rect: image_rect(source),
            destination_rect: image_rect(destination),
            transform: affine(&paint.to_shape),
            opacity: paint.opacity,
            max_anisotropy: 1,
        };
        let bounds = transform_bounds(shape, &self.current.transform);
        let Some(state) = self.state() else { return false };
        let result = self.scene().draw_image(image.index, &draw, image_rect(bounds), state);
        let drawn = result.is_ok();
        self.record(result);
        drawn
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tile_mode_is_the_address_mode_of_each_axis() {
        let (clamp, repeat, mirror) = (
            PROGPU_NATIVE_IMAGE_ADDRESS_CLAMP as u32,
            PROGPU_NATIVE_IMAGE_ADDRESS_REPEAT as u32,
            PROGPU_NATIVE_IMAGE_ADDRESS_MIRROR_REPEAT as u32,
        );
        assert_eq!((clamp, clamp), address_modes(TileMode::None));
        assert_eq!((repeat, repeat), address_modes(TileMode::Tile));
        assert_eq!((mirror, repeat), address_modes(TileMode::FlipX));
        assert_eq!((repeat, mirror), address_modes(TileMode::FlipY));
        assert_eq!((mirror, mirror), address_modes(TileMode::FlipXY));
    }

    #[test]
    fn a_source_range_starts_at_zero_or_after_by_whole_periods() {
        assert_eq!(3.0, positive_start(3.0, 10.0));
        assert_eq!(0.0, positive_start(-20.0, 10.0));
        assert_eq!(15.0, positive_start(-5.0, 10.0));
        assert_eq!(0.0, positive_start(0.0, 10.0));
    }
}
