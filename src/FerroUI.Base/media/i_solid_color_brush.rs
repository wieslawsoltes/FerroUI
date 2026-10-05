use crate::media::{Color, IBrush, IImmutableBrush};

/// Fills an area with a solid color.
pub trait ISolidColorBrush: IBrush {
    /// The color of the brush.
    fn color(&self) -> Color;
}

/// Fills an area with a solid color; the brush cannot change.
pub trait IImmutableSolidColorBrush: ISolidColorBrush + IImmutableBrush {}
