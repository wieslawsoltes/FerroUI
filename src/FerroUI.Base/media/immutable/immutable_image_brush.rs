use crate::media::imaging::Bitmap;
use crate::media::immutable::immutable_tile_brush::immutable_tile_brush_interfaces;
use crate::media::immutable::{ImmutableTileBrush, ImmutableTransform};
use crate::media::{AlignmentX, AlignmentY, IImageBrush, IImageBrushSource, Stretch, TileMode};
use crate::{RelativePoint, RelativeRect};
use std::rc::Rc;

/// Paints an area with an image.
#[derive(Clone)]
pub struct ImmutableImageBrush {
    base: ImmutableTileBrush,
    source: Option<Rc<dyn IImageBrushSource>>,
}

immutable_tile_brush_interfaces!(ImmutableImageBrush, {
    fn as_image_brush(&self) -> Option<&dyn crate::media::IImageBrush> {
        Some(self)
    }
});

impl ImmutableImageBrush {
    /// Creates a brush. `destination_rect` and `source_rect` default to the
    /// whole area.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        source: Option<Rc<Bitmap>>,
        alignment_x: AlignmentX,
        alignment_y: AlignmentY,
        destination_rect: Option<RelativeRect>,
        opacity: f64,
        transform: Option<Rc<ImmutableTransform>>,
        transform_origin: RelativePoint,
        source_rect: Option<RelativeRect>,
        stretch: Stretch,
        tile_mode: TileMode,
        relative_transform: Option<Rc<ImmutableTransform>>,
    ) -> Self {
        Self {
            base: ImmutableTileBrush::new(
                alignment_x,
                alignment_y,
                destination_rect.unwrap_or(RelativeRect::FILL),
                opacity,
                transform,
                transform_origin,
                source_rect.unwrap_or(RelativeRect::FILL),
                stretch,
                tile_mode,
                relative_transform,
            ),
            source: source.map(|source| source as Rc<dyn IImageBrushSource>),
        }
    }

    /// Creates a brush drawing `source` with default values for everything
    /// else: centered, uniformly stretched, not tiled and fully opaque.
    pub fn from_bitmap(source: Option<Rc<Bitmap>>) -> Self {
        Self::new(
            source,
            AlignmentX::Center,
            AlignmentY::Center,
            None,
            1.0,
            None,
            RelativePoint::default(),
            None,
            Stretch::Uniform,
            TileMode::None,
            None,
        )
    }

    /// Creates an immutable copy of `source`.
    pub fn from_brush(source: &dyn IImageBrush) -> Self {
        Self { base: ImmutableTileBrush::from_brush(source), source: source.source() }
    }

    /// The image to draw.
    pub fn source(&self) -> Option<Rc<dyn IImageBrushSource>> {
        self.source.clone()
    }
}

impl IImageBrush for ImmutableImageBrush {
    fn source(&self) -> Option<Rc<dyn IImageBrushSource>> {
        self.source.clone()
    }
}
