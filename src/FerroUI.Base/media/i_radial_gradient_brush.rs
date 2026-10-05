use crate::media::IGradientBrush;
use crate::{RelativePoint, RelativeScalar};

/// Paints an area with a radial gradient.
pub trait IRadialGradientBrush: IGradientBrush {
    /// The start point for the gradient.
    fn center(&self) -> RelativePoint;

    /// The location of the two-dimensional focal point that defines the
    /// beginning of the gradient.
    fn gradient_origin(&self) -> RelativePoint;

    /// The horizontal radius of the outermost circle of the radial gradient.
    fn radius_x(&self) -> RelativeScalar;

    /// The vertical radius of the outermost circle of the radial gradient.
    fn radius_y(&self) -> RelativeScalar;
}
