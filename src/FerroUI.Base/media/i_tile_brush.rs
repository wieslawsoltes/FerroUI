use crate::media::ref_adapter::RefAdapter;
use crate::media::{AlignmentX, AlignmentY, Brush, IBrush, Stretch, TileBrush, TileMode};
use crate::{ObjectType, RelativeRect, Upcast};

/// A brush which displays a repeating image.
pub trait ITileBrush: IBrush {
    /// The horizontal alignment of a tile in the destination.
    fn alignment_x(&self) -> AlignmentX;

    /// The vertical alignment of a tile in the destination.
    fn alignment_y(&self) -> AlignmentY;

    /// The rectangle on the destination in which to paint a tile.
    fn destination_rect(&self) -> RelativeRect;

    /// The rectangle of the source image that will be displayed.
    fn source_rect(&self) -> RelativeRect;

    /// A value controlling how the source rectangle will be stretched to
    /// fill the destination rect.
    fn stretch(&self) -> Stretch;

    /// The brush's tile mode.
    fn tile_mode(&self) -> TileMode;
}

impl<T: ObjectType + Upcast<Brush>> ITileBrush for RefAdapter<T> {
    #[inline]
    fn alignment_x(&self) -> AlignmentX {
        self.class::<TileBrush>().alignment_x()
    }

    #[inline]
    fn alignment_y(&self) -> AlignmentY {
        self.class::<TileBrush>().alignment_y()
    }

    #[inline]
    fn destination_rect(&self) -> RelativeRect {
        self.class::<TileBrush>().destination_rect()
    }

    #[inline]
    fn source_rect(&self) -> RelativeRect {
        self.class::<TileBrush>().source_rect()
    }

    #[inline]
    fn stretch(&self) -> Stretch {
        self.class::<TileBrush>().stretch()
    }

    #[inline]
    fn tile_mode(&self) -> TileMode {
        self.class::<TileBrush>().tile_mode()
    }
}
