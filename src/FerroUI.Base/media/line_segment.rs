use crate::media::{PathSegment, PathSegmentImpl, StreamGeometryContext};
use crate::platform::IGeometryContext;
use crate::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroObjectImpl, FerroProperty, Point, Ref,
    StyledProperty,
};

/// A line segment of a path figure.
#[repr(C)]
pub struct LineSegment {
    base: PathSegment,
}

ferro_class!(LineSegment: PathSegment);
crate::ferro_class_info!(LineSegment { new: LineSegment::new });
ferro_impl_classes!(LineSegment: FerroObjectImpl);

impl PathSegmentImpl for LineSegment {
    fn apply_to(this: &Self, ctx: &mut StreamGeometryContext) {
        ctx.line_to(this.point(), this.is_stroked());
    }

    fn to_string(this: &Self) -> String {
        format!("L {}", this.point())
    }
}

crate::ferro_properties! { impl LineSegment {
    ferro_property!(pub fn point_property() -> StyledProperty<Point> {
        FerroProperty::register::<LineSegment, _>("Point", Point::default())
    });
} }

impl LineSegment {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: PathSegment::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The end point of the line.
    pub fn point(&self) -> Point {
        self.get_value(Self::point_property())
    }

    pub fn set_point(&self, value: Point) {
        self.set_value(Self::point_property(), value)
    }
}
