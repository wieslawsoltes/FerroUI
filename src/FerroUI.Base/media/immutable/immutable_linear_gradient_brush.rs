use crate::media::immutable::immutable_gradient_brush::immutable_gradient_brush_interfaces;
use crate::media::immutable::{ImmutableGradientBrush, ImmutableGradientStop, ImmutableTransform};
use crate::media::{GradientSpreadMethod, ILinearGradientBrush, LinearGradientBrush};
use crate::RelativePoint;
use std::rc::Rc;

/// A brush that draws with a linear gradient.
#[derive(Clone)]
pub struct ImmutableLinearGradientBrush {
    base: ImmutableGradientBrush,
    start_point: RelativePoint,
    end_point: RelativePoint,
}

immutable_gradient_brush_interfaces!(
    ImmutableLinearGradientBrush,
    as_linear_gradient_brush,
    crate::media::ILinearGradientBrush
);

impl ImmutableLinearGradientBrush {
    /// Creates a brush. `transform_origin` defaults to the top left corner,
    /// `start_point` to the top left and `end_point` to the bottom right.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        gradient_stops: &[ImmutableGradientStop],
        opacity: f64,
        transform: Option<Rc<ImmutableTransform>>,
        transform_origin: Option<RelativePoint>,
        spread_method: GradientSpreadMethod,
        start_point: Option<RelativePoint>,
        end_point: Option<RelativePoint>,
        relative_transform: Option<Rc<ImmutableTransform>>,
    ) -> Self {
        Self {
            base: ImmutableGradientBrush::new(
                gradient_stops,
                opacity,
                transform,
                transform_origin,
                spread_method,
                relative_transform,
            ),
            start_point: start_point.unwrap_or(RelativePoint::TOP_LEFT),
            end_point: end_point.unwrap_or(RelativePoint::BOTTOM_RIGHT),
        }
    }

    /// Creates a fully opaque brush with the given stops and default values
    /// for everything else.
    pub fn from_stops(gradient_stops: &[ImmutableGradientStop]) -> Self {
        Self::new(gradient_stops, 1.0, None, None, GradientSpreadMethod::Pad, None, None, None)
    }

    /// Creates an immutable copy of `source`.
    pub fn from_brush(source: &LinearGradientBrush) -> Self {
        Self {
            base: ImmutableGradientBrush::from_brush(source),
            start_point: source.start_point(),
            end_point: source.end_point(),
        }
    }

    /// The start point for the gradient.
    #[inline]
    pub fn start_point(&self) -> RelativePoint {
        self.start_point
    }

    /// The end point for the gradient.
    #[inline]
    pub fn end_point(&self) -> RelativePoint {
        self.end_point
    }
}

impl ILinearGradientBrush for ImmutableLinearGradientBrush {
    #[inline]
    fn start_point(&self) -> RelativePoint {
        self.start_point
    }

    #[inline]
    fn end_point(&self) -> RelativePoint {
        self.end_point
    }
}
