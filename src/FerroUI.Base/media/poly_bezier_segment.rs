use crate::media::{PathSegment, PathSegmentImpl, Points, StreamGeometryContext};
use crate::platform::IGeometryContext;
use crate::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, DirectProperty, FerroObjectImpl, FerroProperty,
    Point, Ref,
};
use std::cell::RefCell;

/// Represents one or more cubic Bezier curves: every three points define the
/// two control points and the end point of one curve.
#[repr(C)]
pub struct PolyBezierSegment {
    base: PathSegment,
    points: RefCell<Option<Points>>,
}

ferro_class!(PolyBezierSegment: PathSegment);
crate::ferro_class_info!(PolyBezierSegment { new: PolyBezierSegment::new });
ferro_impl_classes!(PolyBezierSegment: FerroObjectImpl);

impl PathSegmentImpl for PolyBezierSegment {
    fn apply_to(this: &Self, ctx: &mut StreamGeometryContext) {
        let is_stroked = this.is_stroked();
        if let Some(points) = this.points() {
            let points = points.to_vec();
            // A trailing group of fewer than three points is invalid and is
            // ignored.
            for curve in points.chunks_exact(3) {
                ctx.cubic_bezier_to(curve[0], curve[1], curve[2], is_stroked);
            }
        }
    }

    fn to_string(this: &Self) -> String {
        match this.points() {
            Some(points) if !points.is_empty() => {
                let points: Vec<String> = points.iter().map(|point| point.to_string()).collect();
                format!("C {}", points.join(" "))
            }
            _ => String::new(),
        }
    }
}

crate::ferro_properties! { impl PolyBezierSegment {
    ferro_property!(pub fn points_property() -> DirectProperty<PolyBezierSegment, Option<Points>> {
        FerroProperty::register_direct::<PolyBezierSegment, _>(
            "Points",
            |o| o.points.borrow().clone(),
            Some(|o, v| o.set_points(v)),
            None,
        )
    });
} }

impl PolyBezierSegment {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: PathSegment::construct(), points: RefCell::new(Some(Points::new())) }
    }

    /// Creates a segment with an empty points collection.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates a segment with a copy of the given points.
    pub fn with_points(points: impl IntoIterator<Item = Point>, is_stroked: bool) -> Ref<Self> {
        let result = Self::new();
        result.set_points(Some(Points::from_items(points)));
        result.set_is_stroked(is_stroked);
        result
    }

    /// The points that define the curves.
    pub fn points(&self) -> Option<Points> {
        self.points.borrow().clone()
    }

    pub fn set_points(&self, value: Option<Points>) {
        self.set_and_raise(Self::points_property(), &self.points, value);
    }
}
