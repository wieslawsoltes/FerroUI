use crate::media::{PathSegment, PathSegmentImpl, StreamGeometryContext};
use crate::platform::IGeometryContext;
use crate::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroObjectImpl, FerroProperty, Point, Ref,
    StyledProperty,
};

/// A cubic Bezier curve segment of a path figure.
#[repr(C)]
pub struct BezierSegment {
    base: PathSegment,
}

ferro_class!(BezierSegment: PathSegment);
crate::ferro_class_info!(BezierSegment { new: BezierSegment::new });
ferro_impl_classes!(BezierSegment: FerroObjectImpl);

impl PathSegmentImpl for BezierSegment {
    fn apply_to(this: &Self, ctx: &mut StreamGeometryContext) {
        ctx.cubic_bezier_to(this.point1(), this.point2(), this.point3(), this.is_stroked());
    }

    fn to_string(this: &Self) -> String {
        format!("C {} {} {}", this.point1(), this.point2(), this.point3())
    }
}

crate::ferro_properties! { impl BezierSegment {
    ferro_property!(pub fn point1_property() -> StyledProperty<Point> {
        FerroProperty::register::<BezierSegment, _>("Point1", Point::default())
    });

    ferro_property!(pub fn point2_property() -> StyledProperty<Point> {
        FerroProperty::register::<BezierSegment, _>("Point2", Point::default())
    });

    ferro_property!(pub fn point3_property() -> StyledProperty<Point> {
        FerroProperty::register::<BezierSegment, _>("Point3", Point::default())
    });
} }

impl BezierSegment {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: PathSegment::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The first control point of the curve.
    pub fn point1(&self) -> Point {
        self.get_value(Self::point1_property())
    }

    pub fn set_point1(&self, value: Point) {
        self.set_value(Self::point1_property(), value)
    }

    /// The second control point of the curve.
    pub fn point2(&self) -> Point {
        self.get_value(Self::point2_property())
    }

    pub fn set_point2(&self, value: Point) {
        self.set_value(Self::point2_property(), value)
    }

    /// The end point of the curve.
    pub fn point3(&self) -> Point {
        self.get_value(Self::point3_property())
    }

    pub fn set_point3(&self, value: Point) {
        self.set_value(Self::point3_property(), value)
    }
}
