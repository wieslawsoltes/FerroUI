use crate::media::{Geometry, GeometryImpl};
use crate::platform::{self, IGeometryImpl};
use crate::{
    ferro_class, ferro_property, instantiate, FerroObjectImpl, FerroProperty, Point, Ref,
    StyledProperty,
};
use std::sync::Arc;

/// Represents the geometry of a line.
#[repr(C)]
pub struct LineGeometry {
    base: Geometry,
}

ferro_class!(LineGeometry: Geometry);
crate::ferro_class_info!(LineGeometry { new: LineGeometry::new });

impl FerroObjectImpl for LineGeometry {}

impl GeometryImpl for LineGeometry {
    fn clone_geometry(this: &Self) -> Ref<Geometry> {
        LineGeometry::with_points(this.start_point(), this.end_point()).upcast()
    }

    fn create_defining_geometry(this: &Self) -> Option<Arc<dyn IGeometryImpl>> {
        let factory = platform::render_interface();

        Some(factory.create_line_geometry(this.start_point(), this.end_point()))
    }
}

crate::ferro_properties! { impl LineGeometry {
    ferro_property!(pub fn start_point_property() -> StyledProperty<Point> {
        FerroProperty::register::<LineGeometry, _>("StartPoint", Point::default())
    });

    ferro_property!(pub fn end_point_property() -> StyledProperty<Point> {
        FerroProperty::register::<LineGeometry, _>("EndPoint", Point::default())
    });
} }

impl LineGeometry {
    fn static_constructor() {
        Geometry::affects_geometry(&[
            Self::start_point_property().as_property(),
            Self::end_point_property().as_property(),
        ]);
    }

    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: Geometry::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates a line geometry between the given points.
    pub fn with_points(start_point: Point, end_point: Point) -> Ref<Self> {
        let result = Self::new();
        result.set_start_point(start_point);
        result.set_end_point(end_point);
        result
    }

    /// The start point of the line.
    pub fn start_point(&self) -> Point {
        self.get_value(Self::start_point_property())
    }

    pub fn set_start_point(&self, value: Point) {
        self.set_value(Self::start_point_property(), value)
    }

    /// The end point of the line.
    pub fn end_point(&self) -> Point {
        self.get_value(Self::end_point_property())
    }

    pub fn set_end_point(&self, value: Point) {
        self.set_value(Self::end_point_property(), value)
    }
}
