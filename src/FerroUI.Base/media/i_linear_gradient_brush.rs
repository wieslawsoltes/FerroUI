use crate::media::IGradientBrush;
use crate::RelativePoint;

/// A brush that draws with a linear gradient.
pub trait ILinearGradientBrush: IGradientBrush {
    /// The start point for the gradient.
    fn start_point(&self) -> RelativePoint;

    /// The end point for the gradient.
    fn end_point(&self) -> RelativePoint;
}
