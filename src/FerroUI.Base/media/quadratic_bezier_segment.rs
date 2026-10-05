use crate::media::{PathSegment, PathSegmentImpl, StreamGeometryContext};
use crate::platform::IGeometryContext;
use crate::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroObjectImpl, FerroProperty, Point, Ref,
    StyledProperty,
};

/// A quadratic Bezier curve segment of a path figure.
#[repr(C)]
pub struct QuadraticBezierSegment {
    base: PathSegment,
}

ferro_class!(QuadraticBezierSegment: PathSegment);
crate::ferro_class_info!(QuadraticBezierSegment { new: QuadraticBezierSegment::new });
ferro_impl_classes!(QuadraticBezierSegment: FerroObjectImpl);

impl PathSegmentImpl for QuadraticBezierSegment {
    fn apply_to(this: &Self, ctx: &mut StreamGeometryContext) {
        ctx.quadratic_bezier_to(this.point1(), this.point2(), this.is_stroked());
    }

    fn to_string(this: &Self) -> String {
        format!("Q {} {}", this.point1(), this.point2())
    }
}

crate::ferro_properties! { impl QuadraticBezierSegment {
    ferro_property!(pub fn point1_property() -> StyledProperty<Point> {
        FerroProperty::register::<QuadraticBezierSegment, _>("Point1", Point::default())
    });

    ferro_property!(pub fn point2_property() -> StyledProperty<Point> {
        FerroProperty::register::<QuadraticBezierSegment, _>("Point2", Point::default())
    });
} }

impl QuadraticBezierSegment {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: PathSegment::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The control point of the curve.
    pub fn point1(&self) -> Point {
        self.get_value(Self::point1_property())
    }

    pub fn set_point1(&self, value: Point) {
        self.set_value(Self::point1_property(), value)
    }

    /// The end point of the curve.
    pub fn point2(&self) -> Point {
        self.get_value(Self::point2_property())
    }

    pub fn set_point2(&self, value: Point) {
        self.set_value(Self::point2_property(), value)
    }
}
