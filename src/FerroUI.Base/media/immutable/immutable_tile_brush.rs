use crate::media::immutable::ImmutableTransform;
use crate::media::{AlignmentX, AlignmentY, ITileBrush, ITransform, Stretch, TileMode, TransformExtensions};
use crate::{RelativePoint, RelativeRect};
use std::rc::Rc;

/// The data shared by all immutable tile brushes.
#[derive(Clone)]
pub struct ImmutableTileBrush {
    alignment_x: AlignmentX,
    alignment_y: AlignmentY,
    destination_rect: RelativeRect,
    opacity: f64,
    transform: Option<Rc<dyn ITransform>>,
    transform_origin: RelativePoint,
    source_rect: RelativeRect,
    stretch: Stretch,
    tile_mode: TileMode,
    relative_transform: Option<Rc<dyn ITransform>>,
}

impl ImmutableTileBrush {
    /// Creates the tile brush data.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        alignment_x: AlignmentX,
        alignment_y: AlignmentY,
        destination_rect: RelativeRect,
        opacity: f64,
        transform: Option<Rc<ImmutableTransform>>,
        transform_origin: RelativePoint,
        source_rect: RelativeRect,
        stretch: Stretch,
        tile_mode: TileMode,
        relative_transform: Option<Rc<ImmutableTransform>>,
    ) -> Self {
        Self {
            alignment_x,
            alignment_y,
            destination_rect,
            opacity,
            transform: transform.map(|t| t as Rc<dyn ITransform>),
            transform_origin,
            source_rect,
            stretch,
            tile_mode,
            relative_transform: relative_transform.map(|t| t as Rc<dyn ITransform>),
        }
    }

    /// Creates an immutable copy of the tile brush data of `source`.
    pub fn from_brush(source: &dyn ITileBrush) -> Self {
        Self::new(
            source.alignment_x(),
            source.alignment_y(),
            source.destination_rect(),
            source.opacity(),
            source.transform().map(|t| Rc::new(TransformExtensions::to_immutable(&*t))),
            source.transform_origin(),
            source.source_rect(),
            source.stretch(),
            source.tile_mode(),
            source.relative_transform().map(|t| Rc::new(TransformExtensions::to_immutable(&*t))),
        )
    }

    /// The horizontal alignment of a tile in the destination.
    #[inline]
    pub fn alignment_x(&self) -> AlignmentX {
        self.alignment_x
    }

    /// The vertical alignment of a tile in the destination.
    #[inline]
    pub fn alignment_y(&self) -> AlignmentY {
        self.alignment_y
    }

    /// The rectangle on the destination in which to paint a tile.
    #[inline]
    pub fn destination_rect(&self) -> RelativeRect {
        self.destination_rect
    }

    /// The opacity of the brush.
    #[inline]
    pub fn opacity(&self) -> f64 {
        self.opacity
    }

    /// The transform of the brush.
    #[inline]
    pub fn transform(&self) -> Option<Rc<dyn ITransform>> {
        self.transform.clone()
    }

    /// The transform origin of the brush.
    #[inline]
    pub fn transform_origin(&self) -> RelativePoint {
        self.transform_origin
    }

    /// The rectangle of the source image that will be displayed.
    #[inline]
    pub fn source_rect(&self) -> RelativeRect {
        self.source_rect
    }

    /// A value controlling how the source rectangle will be stretched to
    /// fill the destination rect.
    #[inline]
    pub fn stretch(&self) -> Stretch {
        self.stretch
    }

    /// The brush's tile mode.
    #[inline]
    pub fn tile_mode(&self) -> TileMode {
        self.tile_mode
    }

    /// The transform of the brush, relative to the bounds of the painted
    /// area.
    #[inline]
    pub fn relative_transform(&self) -> Option<Rc<dyn ITransform>> {
        self.relative_transform.clone()
    }
}

/// Implements the brush interfaces of an immutable tile brush whose `base`
/// field is an [`ImmutableTileBrush`]. The optional trailing tokens are
/// further `IBrush` members (interface casts of the deriving type).
macro_rules! immutable_tile_brush_interfaces {
    ($name:ty $(, { $($extra:tt)* })?) => {
        impl ::std::ops::Deref for $name {
            type Target = $crate::media::immutable::ImmutableTileBrush;
            #[inline]
            fn deref(&self) -> &Self::Target {
                &self.base
            }
        }

        impl $crate::media::IBrush for $name {
            #[inline]
            fn opacity(&self) -> f64 {
                self.base.opacity()
            }

            fn transform(&self) -> Option<::std::rc::Rc<dyn $crate::media::ITransform>> {
                self.base.transform()
            }

            #[inline]
            fn transform_origin(&self) -> $crate::RelativePoint {
                self.base.transform_origin()
            }

            fn relative_transform(&self) -> Option<::std::rc::Rc<dyn $crate::media::ITransform>> {
                self.base.relative_transform()
            }

            fn as_any(&self) -> &dyn ::std::any::Any {
                self
            }

            fn as_tile_brush(&self) -> Option<&dyn $crate::media::ITileBrush> {
                Some(self)
            }

            fn into_immutable_brush(
                self: ::std::rc::Rc<Self>,
            ) -> Option<::std::rc::Rc<dyn $crate::media::IImmutableBrush>> {
                Some(self)
            }

            $($($extra)*)?
        }

        impl $crate::media::ITileBrush for $name {
            #[inline]
            fn alignment_x(&self) -> $crate::media::AlignmentX {
                self.base.alignment_x()
            }

            #[inline]
            fn alignment_y(&self) -> $crate::media::AlignmentY {
                self.base.alignment_y()
            }

            #[inline]
            fn destination_rect(&self) -> $crate::RelativeRect {
                self.base.destination_rect()
            }

            #[inline]
            fn source_rect(&self) -> $crate::RelativeRect {
                self.base.source_rect()
            }

            #[inline]
            fn stretch(&self) -> $crate::media::Stretch {
                self.base.stretch()
            }

            #[inline]
            fn tile_mode(&self) -> $crate::media::TileMode {
                self.base.tile_mode()
            }
        }

        impl $crate::media::IImmutableBrush for $name {}
    };
}

pub(crate) use immutable_tile_brush_interfaces;
