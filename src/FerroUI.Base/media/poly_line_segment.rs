use crate::media::{PathSegment, PathSegmentImpl, Points, StreamGeometryContext};
use crate::platform::IGeometryContext;
use crate::{
    ferro_class, ferro_property, instantiate, FerroObjectImpl, FerroObjectImplExt, FerroProperty, Point, Ref,
    StyledProperty,
};

/// Represents a set of line segments defined by a points collection with each
/// point specifying the end point of a line segment.
#[repr(C)]
pub struct PolyLineSegment {
    base: PathSegment,
}

ferro_class!(PolyLineSegment: PathSegment);
crate::ferro_class_info!(PolyLineSegment { new: PolyLineSegment::new });

impl FerroObjectImpl for PolyLineSegment {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_points(Points::new());
    }
}

impl PathSegmentImpl for PolyLineSegment {
    fn apply_to(this: &Self, ctx: &mut StreamGeometryContext) {
        let is_stroked = this.is_stroked();
        for point in this.points().iter() {
            ctx.line_to(point, is_stroked);
        }
    }

    fn to_string(this: &Self) -> String {
        let points = this.points();
        if points.is_empty() {
            return String::new();
        }
        let points: Vec<String> = points.iter().map(|point| point.to_string()).collect();
        format!("L {}", points.join(" "))
    }
}

crate::ferro_properties! { impl PolyLineSegment {
    ferro_property!(pub fn points_property() -> StyledProperty<Option<Points>> {
        FerroProperty::register::<PolyLineSegment, _>("Points", None)
    });
} }

impl PolyLineSegment {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: PathSegment::construct() }
    }

    /// Creates a segment with an empty points collection.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates a segment with a copy of the given points.
    pub fn with_points(points: impl IntoIterator<Item = Point>) -> Ref<Self> {
        let result = Self::new();
        result.set_points(Points::from_items(points));
        result
    }

    /// The points. Panics if the property has been cleared.
    pub fn points(&self) -> Points {
        self.get_value(Self::points_property()).expect("Points is not set")
    }

    pub fn set_points(&self, value: Points) {
        self.set_value(Self::points_property(), Some(value))
    }
}
