use crate::media::immutable::immutable_gradient_brush::immutable_gradient_brush_interfaces;
use crate::media::immutable::{ImmutableGradientBrush, ImmutableGradientStop, ImmutableTransform};
use crate::media::{ConicGradientBrush, GradientSpreadMethod, IConicGradientBrush};
use crate::RelativePoint;
use std::rc::Rc;

/// A brush that draws with a sweep gradient.
#[derive(Clone)]
pub struct ImmutableConicGradientBrush {
    base: ImmutableGradientBrush,
    center: RelativePoint,
    angle: f64,
}

immutable_gradient_brush_interfaces!(
    ImmutableConicGradientBrush,
    as_conic_gradient_brush,
    crate::media::IConicGradientBrush
);

impl ImmutableConicGradientBrush {
    /// Creates a brush. `transform_origin` defaults to the top left corner
    /// and `center` to the center.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        gradient_stops: &[ImmutableGradientStop],
        opacity: f64,
        transform: Option<Rc<ImmutableTransform>>,
        transform_origin: Option<RelativePoint>,
        spread_method: GradientSpreadMethod,
        center: Option<RelativePoint>,
        angle: f64,
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
            center: center.unwrap_or(RelativePoint::CENTER),
            angle,
        }
    }

    /// Creates a fully opaque brush with the given stops and default values
    /// for everything else.
    pub fn from_stops(gradient_stops: &[ImmutableGradientStop]) -> Self {
        Self::new(gradient_stops, 1.0, None, None, GradientSpreadMethod::Pad, None, 0.0, None)
    }

    /// Creates an immutable copy of `source`.
    pub fn from_brush(source: &ConicGradientBrush) -> Self {
        Self { base: ImmutableGradientBrush::from_brush(source), center: source.center(), angle: source.angle() }
    }

    /// The center point for the gradient.
    #[inline]
    pub fn center(&self) -> RelativePoint {
        self.center
    }

    /// The starting angle of the gradient's sweep, in degrees.
    #[inline]
    pub fn angle(&self) -> f64 {
        self.angle
    }
}

impl IConicGradientBrush for ImmutableConicGradientBrush {
    #[inline]
    fn center(&self) -> RelativePoint {
        self.center
    }

    #[inline]
    fn angle(&self) -> f64 {
        self.angle
    }
}
