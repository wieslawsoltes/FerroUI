use crate::media::IGradientBrush;
use crate::RelativePoint;

/// Paints an area with a conic gradient.
pub trait IConicGradientBrush: IGradientBrush {
    /// The center point for the gradient.
    fn center(&self) -> RelativePoint;

    /// The starting angle for the gradient in degrees, measured from the
    /// point above the center point.
    fn angle(&self) -> f64;
}
