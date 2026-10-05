use crate::media::immutable::immutable_gradient_brush::immutable_gradient_brush_interfaces;
use crate::media::immutable::{ImmutableGradientBrush, ImmutableGradientStop, ImmutableTransform};
use crate::media::{GradientSpreadMethod, IRadialGradientBrush, RadialGradientBrush};
use crate::{RelativePoint, RelativeScalar, RelativeUnit};
use std::rc::Rc;

/// A brush that draws with a radial gradient.
#[derive(Clone)]
pub struct ImmutableRadialGradientBrush {
    base: ImmutableGradientBrush,
    center: RelativePoint,
    gradient_origin: RelativePoint,
    radius_x: RelativeScalar,
    radius_y: RelativeScalar,
}

immutable_gradient_brush_interfaces!(
    ImmutableRadialGradientBrush,
    as_radial_gradient_brush,
    crate::media::IRadialGradientBrush
);

impl ImmutableRadialGradientBrush {
    /// Creates a brush. `transform_origin` defaults to the top left corner,
    /// `center` and `gradient_origin` to the center and the radii to one
    /// half, relative.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        gradient_stops: &[ImmutableGradientStop],
        opacity: f64,
        transform: Option<Rc<ImmutableTransform>>,
        transform_origin: Option<RelativePoint>,
        spread_method: GradientSpreadMethod,
        center: Option<RelativePoint>,
        gradient_origin: Option<RelativePoint>,
        radius_x: Option<RelativeScalar>,
        radius_y: Option<RelativeScalar>,
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
            gradient_origin: gradient_origin.unwrap_or(RelativePoint::CENTER),
            radius_x: radius_x.unwrap_or(RelativeScalar::MIDDLE),
            radius_y: radius_y.unwrap_or(RelativeScalar::MIDDLE),
        }
    }

    /// Creates a brush whose radii are both `radius`, relative to the
    /// painted area.
    #[allow(clippy::too_many_arguments)]
    pub fn with_radius(
        gradient_stops: &[ImmutableGradientStop],
        opacity: f64,
        transform: Option<Rc<ImmutableTransform>>,
        transform_origin: Option<RelativePoint>,
        spread_method: GradientSpreadMethod,
        center: Option<RelativePoint>,
        gradient_origin: Option<RelativePoint>,
        radius: f64,
    ) -> Self {
        Self::new(
            gradient_stops,
            opacity,
            transform,
            transform_origin,
            spread_method,
            center,
            gradient_origin,
            Some(RelativeScalar::new(radius, RelativeUnit::Relative)),
            Some(RelativeScalar::new(radius, RelativeUnit::Relative)),
            None,
        )
    }

    /// Creates a fully opaque brush with the given stops and default values
    /// for everything else.
    pub fn from_stops(gradient_stops: &[ImmutableGradientStop]) -> Self {
        Self::new(gradient_stops, 1.0, None, None, GradientSpreadMethod::Pad, None, None, None, None, None)
    }

    /// Creates an immutable copy of `source`.
    pub fn from_brush(source: &RadialGradientBrush) -> Self {
        Self {
            base: ImmutableGradientBrush::from_brush(source),
            center: source.center(),
            gradient_origin: source.gradient_origin(),
            radius_x: source.radius_x(),
            // The reference implementation copies the horizontal radius here.
            radius_y: source.radius_x(),
        }
    }

    /// The start point for the gradient.
    #[inline]
    pub fn center(&self) -> RelativePoint {
        self.center
    }

    /// The location of the two-dimensional focal point that defines the
    /// beginning of the gradient.
    #[inline]
    pub fn gradient_origin(&self) -> RelativePoint {
        self.gradient_origin
    }

    /// The horizontal radius of the outermost circle of the radial gradient.
    #[inline]
    pub fn radius_x(&self) -> RelativeScalar {
        self.radius_x
    }

    /// The vertical radius of the outermost circle of the radial gradient.
    #[inline]
    pub fn radius_y(&self) -> RelativeScalar {
        self.radius_y
    }
}

impl IRadialGradientBrush for ImmutableRadialGradientBrush {
    #[inline]
    fn center(&self) -> RelativePoint {
        self.center
    }

    #[inline]
    fn gradient_origin(&self) -> RelativePoint {
        self.gradient_origin
    }

    #[inline]
    fn radius_x(&self) -> RelativeScalar {
        self.radius_x
    }

    #[inline]
    fn radius_y(&self) -> RelativeScalar {
        self.radius_y
    }
}
