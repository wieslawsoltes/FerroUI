use crate::media::{AlignmentX, AlignmentY, ITileBrush, MediaExtensions, Stretch, StretchDirection, TileMode};
use crate::{Matrix, Point, Rect, RelativeRect, Size, Vector};

/// Calculates the rectangles and transform needed to render a tile brush.
pub struct TileBrushCalculator {
    image_size: Size,
    draw_rect: Rect,
    is_valid: bool,
    destination_rect: Rect,
    intermediate_size: Size,
    intermediate_transform: Matrix,
    source_rect: Rect,
}

impl TileBrushCalculator {
    /// Creates a calculator for a tile brush.
    ///
    /// * `brush` - The brush to be rendered.
    /// * `content_size` - The size of the content of the tile brush.
    /// * `target_size` - The size of the control to which the brush is being rendered.
    pub fn from_brush(brush: &dyn ITileBrush, content_size: Size, target_size: Size) -> Self {
        Self::new(
            brush.tile_mode(),
            brush.stretch(),
            brush.alignment_x(),
            brush.alignment_y(),
            brush.source_rect(),
            brush.destination_rect(),
            content_size,
            target_size,
        )
    }

    /// Initializes a new instance of the [`TileBrushCalculator`] type.
    ///
    /// * `tile_mode` - The brush's tile mode.
    /// * `stretch` - The brush's stretch.
    /// * `alignment_x` - The brush's horizontal alignment.
    /// * `alignment_y` - The brush's vertical alignment.
    /// * `source_rect` - The brush's source rect
    /// * `destination_rect` - The brush's destination rect.
    /// * `content_size` - The size of the content of the tile brush.
    /// * `target_size` - The size of the control to which the brush is being rendered.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tile_mode: TileMode,
        stretch: Stretch,
        alignment_x: AlignmentX,
        alignment_y: AlignmentY,
        source_rect: RelativeRect,
        destination_rect: RelativeRect,
        content_size: Size,
        target_size: Size,
    ) -> Self {
        let image_size = content_size;

        let source_rect = source_rect.to_pixels(image_size);
        let destination_rect = destination_rect.to_pixels(target_size);

        let scale = MediaExtensions::calculate_scaling(
            stretch,
            destination_rect.size(),
            source_rect.size(),
            StretchDirection::Both,
        );
        let translate = Self::calculate_translate(alignment_x, alignment_y, source_rect, destination_rect, scale);

        let intermediate_size = if tile_mode == TileMode::None { target_size } else { destination_rect.size() };
        let (intermediate_transform, draw_rect) =
            Self::calculate_intermediate_transform(tile_mode, source_rect, destination_rect, scale, translate);

        Self {
            image_size,
            draw_rect,
            is_valid: false,
            destination_rect,
            intermediate_size,
            intermediate_transform,
            source_rect,
        }
    }

    /// Upstream declares this get-only property and never assigns it, so it is always `false`.
    pub fn is_valid(&self) -> bool {
        self.is_valid
    }

    /// Gets the rectangle on the destination control to which content should be rendered.
    ///
    /// If the tile mode of the brush is repeating then this is describes rectangle
    /// of a single repeat of the tiled content.
    pub fn destination_rect(&self) -> Rect {
        self.destination_rect
    }

    /// Gets the clip rectangle on the intermediate image with which the brush content should be
    /// drawn when [`needs_intermediate`](Self::needs_intermediate) is true.
    pub fn intermediate_clip(&self) -> Rect {
        self.draw_rect
    }

    /// Gets the size of the intermediate image that should be created when
    /// [`needs_intermediate`](Self::needs_intermediate) is true.
    pub fn intermediate_size(&self) -> Size {
        self.intermediate_size
    }

    /// Gets the transform to be used when rendering to the intermediate image when
    /// [`needs_intermediate`](Self::needs_intermediate) is true.
    pub fn intermediate_transform(&self) -> Matrix {
        self.intermediate_transform
    }

    /// Gets a value indicating whether an intermediate image should be created in order to
    /// render the tile brush.
    ///
    /// Intermediate images are required when a brush's tile mode is not repeating
    /// but the source and destination aspect ratios are unequal, as all of the currently
    /// supported rendering backends do not support non-tiled image brushes.
    pub fn needs_intermediate(&self) -> bool {
        if self.intermediate_transform != Matrix::IDENTITY {
            return true;
        }
        if self.source_rect.position() != Point::default() {
            return true;
        }
        if self.source_rect.size().aspect_ratio() == self.image_size.aspect_ratio() {
            return false;
        }
        if self.source_rect.width != self.image_size.width || self.source_rect.height != self.image_size.height {
            return true;
        }
        false
    }

    /// Gets the area of the source content to be rendered.
    pub fn source_rect(&self) -> Rect {
        self.source_rect
    }

    pub fn calculate_translate(
        alignment_x: AlignmentX,
        alignment_y: AlignmentY,
        source_rect: Rect,
        destination_rect: Rect,
        scale: Vector,
    ) -> Vector {
        Self::calculate_translate_sizes(alignment_x, alignment_y, source_rect.size() * scale, destination_rect.size())
    }

    pub fn calculate_translate_sizes(
        alignment_x: AlignmentX,
        alignment_y: AlignmentY,
        source_size: Size,
        destination_size: Size,
    ) -> Vector {
        let mut x = 0.0;
        let mut y = 0.0;

        match alignment_x {
            AlignmentX::Center => x += (destination_size.width - source_size.width) / 2.0,
            AlignmentX::Right => x += destination_size.width - source_size.width,
            AlignmentX::Left => {}
        }

        match alignment_y {
            AlignmentY::Center => y += (destination_size.height - source_size.height) / 2.0,
            AlignmentY::Bottom => y += destination_size.height - source_size.height,
            AlignmentY::Top => {}
        }

        Vector::new(x, y)
    }

    /// Returns the transform and the draw rectangle (upstream's `out drawRect`).
    pub fn calculate_intermediate_transform(
        tile_mode: TileMode,
        source_rect: Rect,
        destination_rect: Rect,
        scale: Vector,
        translate: Vector,
    ) -> (Matrix, Rect) {
        let mut transform = Matrix::create_translation_vector((-source_rect.position()).into())
            * Matrix::create_scale_vector(scale)
            * Matrix::create_translation_vector(translate);
        let dr;

        if tile_mode == TileMode::None {
            dr = destination_rect;
            transform *= Matrix::create_translation_vector(destination_rect.position().into());
        } else {
            dr = Rect::from_size(destination_rect.size());
        }

        (transform, dr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RelativeUnit;

    #[test]
    fn no_tile_fill_1x() {
        let result = TileBrushCalculator::new(
            TileMode::None,
            Stretch::Fill,
            AlignmentX::Center,
            AlignmentY::Center,
            RelativeRect::FILL,
            RelativeRect::FILL,
            Size::new(100.0, 100.0),
            Size::new(100.0, 100.0),
        );

        assert!(!result.needs_intermediate());
        assert_eq!(Rect::new(0.0, 0.0, 100.0, 100.0), result.source_rect());
        assert_eq!(Rect::new(0.0, 0.0, 100.0, 100.0), result.destination_rect());
    }

    #[test]
    fn no_tile_fill_2x() {
        let result = TileBrushCalculator::new(
            TileMode::None,
            Stretch::Fill,
            AlignmentX::Center,
            AlignmentY::Center,
            RelativeRect::FILL,
            RelativeRect::FILL,
            Size::new(100.0, 100.0),
            Size::new(200.0, 200.0),
        );

        // This doesn't need an intermediate render target (upstream keeps the same note).
        assert!(result.needs_intermediate());
        assert_eq!(Rect::new(0.0, 0.0, 100.0, 100.0), result.source_rect());
        assert_eq!(Rect::new(0.0, 0.0, 200.0, 200.0), result.destination_rect());
    }

    #[test]
    fn no_tile_uniform_center_horiz() {
        let result = TileBrushCalculator::new(
            TileMode::None,
            Stretch::Uniform,
            AlignmentX::Center,
            AlignmentY::Center,
            RelativeRect::FILL,
            RelativeRect::FILL,
            Size::new(100.0, 100.0),
            Size::new(200.0, 100.0),
        );

        assert!(result.needs_intermediate());
        assert_eq!(Rect::new(0.0, 0.0, 100.0, 100.0), result.source_rect());
        assert_eq!(Rect::new(0.0, 0.0, 200.0, 100.0), result.destination_rect());
        assert_eq!(Size::new(200.0, 100.0), result.intermediate_size());
        assert_eq!(Matrix::create_translation(50.0, 0.0), result.intermediate_transform());
    }

    #[test]
    fn no_tile_uniform_center_vert() {
        let result = TileBrushCalculator::new(
            TileMode::None,
            Stretch::Uniform,
            AlignmentX::Center,
            AlignmentY::Center,
            RelativeRect::FILL,
            RelativeRect::FILL,
            Size::new(100.0, 100.0),
            Size::new(100.0, 200.0),
        );

        assert!(result.needs_intermediate());
        assert_eq!(Rect::new(0.0, 0.0, 100.0, 100.0), result.source_rect());
        assert_eq!(Rect::new(0.0, 0.0, 100.0, 200.0), result.destination_rect());
        assert_eq!(Size::new(100.0, 200.0), result.intermediate_size());
        assert_eq!(Matrix::create_translation(0.0, 50.0), result.intermediate_transform());
    }

    #[test]
    fn no_tile_no_stretch_bottom_right_quarter_dest() {
        let result = TileBrushCalculator::new(
            TileMode::None,
            Stretch::None,
            AlignmentX::Center,
            AlignmentY::Center,
            RelativeRect::FILL,
            RelativeRect::new(0.5, 0.5, 0.5, 0.5, RelativeUnit::Relative),
            Size::new(800.0, 800.0),
            Size::new(400.0, 400.0),
        );

        assert!(result.needs_intermediate());
        assert_eq!(Rect::new(0.0, 0.0, 800.0, 800.0), result.source_rect());
        assert_eq!(Rect::new(200.0, 200.0, 200.0, 200.0), result.destination_rect());
        assert_eq!(Size::new(400.0, 400.0), result.intermediate_size());
        assert_eq!(Rect::new(200.0, 200.0, 200.0, 200.0), result.intermediate_clip());
        assert_eq!(Matrix::create_translation(-100.0, -100.0), result.intermediate_transform());
    }

    #[test]
    fn tile_no_stretch_bottom_right_quarter_source_center_quarter_dest() {
        let result = TileBrushCalculator::new(
            TileMode::Tile,
            Stretch::None,
            AlignmentX::Center,
            AlignmentY::Center,
            RelativeRect::new(0.5, 0.5, 0.5, 0.5, RelativeUnit::Relative),
            RelativeRect::new(0.25, 0.25, 0.5, 0.5, RelativeUnit::Relative),
            Size::new(800.0, 800.0),
            Size::new(400.0, 400.0),
        );

        assert!(result.needs_intermediate());
        assert_eq!(Rect::new(400.0, 400.0, 400.0, 400.0), result.source_rect());
        assert_eq!(Rect::new(100.0, 100.0, 200.0, 200.0), result.destination_rect());
        assert_eq!(Size::new(200.0, 200.0), result.intermediate_size());
        assert_eq!(Rect::new(0.0, 0.0, 200.0, 200.0), result.intermediate_clip());
        assert_eq!(Matrix::create_translation(-500.0, -500.0), result.intermediate_transform());
    }

    // --- Not upstream tests: expected values computed by hand from the upstream algorithm.

    #[test]
    fn calculate_translate_follows_the_alignment() {
        let source = Size::new(40.0, 20.0);
        let destination = Size::new(100.0, 100.0);

        for (alignment_x, alignment_y, expected) in [
            (AlignmentX::Left, AlignmentY::Top, Vector::new(0.0, 0.0)),
            (AlignmentX::Center, AlignmentY::Center, Vector::new(30.0, 40.0)),
            (AlignmentX::Right, AlignmentY::Bottom, Vector::new(60.0, 80.0)),
        ] {
            assert_eq!(
                expected,
                TileBrushCalculator::calculate_translate_sizes(alignment_x, alignment_y, source, destination)
            );
        }

        assert_eq!(
            Vector::new(10.0, 60.0),
            TileBrushCalculator::calculate_translate(
                AlignmentX::Center,
                AlignmentY::Bottom,
                Rect::new(5.0, 5.0, 40.0, 20.0),
                Rect::new(7.0, 7.0, 100.0, 100.0),
                Vector::new(2.0, 2.0),
            )
        );
    }

    #[test]
    fn calculate_intermediate_transform_offsets_by_destination_only_when_not_tiled() {
        let source = Rect::new(10.0, 20.0, 50.0, 50.0);
        let destination = Rect::new(5.0, 6.0, 100.0, 100.0);
        let scale = Vector::new(2.0, 2.0);
        let translate = Vector::new(1.0, 2.0);

        let (transform, draw_rect) =
            TileBrushCalculator::calculate_intermediate_transform(TileMode::None, source, destination, scale, translate);
        assert_eq!(Matrix::new(2.0, 0.0, 0.0, 2.0, -20.0 + 1.0 + 5.0, -40.0 + 2.0 + 6.0), transform);
        assert_eq!(destination, draw_rect);

        let (transform, draw_rect) =
            TileBrushCalculator::calculate_intermediate_transform(TileMode::FlipXY, source, destination, scale, translate);
        assert_eq!(Matrix::new(2.0, 0.0, 0.0, 2.0, -19.0, -38.0), transform);
        assert_eq!(Rect::new(0.0, 0.0, 100.0, 100.0), draw_rect);
    }

    #[test]
    fn is_valid_is_never_set() {
        let result = TileBrushCalculator::new(
            TileMode::None,
            Stretch::Fill,
            AlignmentX::Center,
            AlignmentY::Center,
            RelativeRect::FILL,
            RelativeRect::FILL,
            Size::new(100.0, 100.0),
            Size::new(100.0, 100.0),
        );
        assert!(!result.is_valid());
    }

    // Not from upstream.
    #[test]
    fn from_brush_uses_the_parameters_of_the_brush() {
        let brush = crate::media::ImageBrush::new();
        brush.set_tile_mode(TileMode::Tile);
        brush.set_stretch(Stretch::Fill);
        brush.set_destination_rect(RelativeRect::new(0.0, 0.0, 0.5, 0.5, RelativeUnit::Relative));
        let handle: std::rc::Rc<dyn crate::media::IBrush> = brush.into();

        let result =
            TileBrushCalculator::from_brush(handle.as_tile_brush().unwrap(), Size::new(10.0, 10.0), Size::new(40.0, 40.0));
        let expected = TileBrushCalculator::new(
            TileMode::Tile,
            Stretch::Fill,
            AlignmentX::Center,
            AlignmentY::Center,
            RelativeRect::FILL,
            RelativeRect::new(0.0, 0.0, 0.5, 0.5, RelativeUnit::Relative),
            Size::new(10.0, 10.0),
            Size::new(40.0, 40.0),
        );

        assert_eq!(Rect::new(0.0, 0.0, 20.0, 20.0), result.destination_rect());
        assert_eq!(expected.destination_rect(), result.destination_rect());
        assert_eq!(expected.intermediate_size(), result.intermediate_size());
        assert_eq!(expected.intermediate_transform(), result.intermediate_transform());
        assert_eq!(expected.source_rect(), result.source_rect());
    }
}
