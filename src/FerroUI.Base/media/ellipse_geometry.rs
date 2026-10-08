use crate::media::{Geometry, GeometryImpl};
use crate::platform::{self, IGeometryImpl};
use crate::{
    ferro_class, ferro_property, instantiate, FerroObjectImpl, FerroProperty, Point, Rect, Ref,
    StyledProperty,
};
use std::sync::Arc;

/// Represents the geometry of an ellipse or circle.
#[repr(C)]
pub struct EllipseGeometry {
    base: Geometry,
}

ferro_class!(EllipseGeometry: Geometry);
crate::ferro_class_info!(EllipseGeometry { new: EllipseGeometry::new });

impl FerroObjectImpl for EllipseGeometry {}

impl GeometryImpl for EllipseGeometry {
    fn clone_geometry(this: &Self) -> Ref<Geometry> {
        // Note that the ellipse properties are used in two modes:
        //
        //  1. Rect-only Mode:
        //     Directly set the rectangle bounds the ellipse will fill
        //
        //  2. Center + Radii Mode:
        //     Set a center-point and then X/Y-axis radii that are used to
        //     calculate the rectangle bounds the ellipse will fill.
        //
        // Rendering the ellipse will only ever use one of these two modes
        // based on if the Rect property is set (not equal to default).
        //
        // This means it would normally be fine to copy ONLY the Rect property
        // when it is set. However, while it would render the same, it isn't
        // a true clone. We want to include all the properties here regardless
        // of the rendering mode that will eventually be used.
        let result = EllipseGeometry::new();
        result.set_rect(this.rect());
        result.set_radius_x(this.radius_x());
        result.set_radius_y(this.radius_y());
        result.set_center(this.center());
        result.upcast()
    }

    fn create_defining_geometry(this: &Self) -> Option<Arc<dyn IGeometryImpl>> {
        let factory = platform::render_interface();

        let rect = this.rect();
        if rect != Rect::default() {
            return Some(factory.create_ellipse_geometry(rect));
        }

        let (center, radius_x, radius_y) = (this.center(), this.radius_x(), this.radius_y());
        let origin_x = center.x - radius_x;
        let origin_y = center.y - radius_y;
        let width = radius_x * 2.0;
        let height = radius_y * 2.0;

        Some(factory.create_ellipse_geometry(Rect::new(origin_x, origin_y, width, height)))
    }
}

crate::ferro_properties! { impl EllipseGeometry {
    ferro_property!(pub fn rect_property() -> StyledProperty<Rect> {
        FerroProperty::register::<EllipseGeometry, _>("Rect", Rect::default())
    });

    ferro_property!(pub fn radius_x_property() -> StyledProperty<f64> {
        FerroProperty::register::<EllipseGeometry, _>("RadiusX", 0.0)
    });

    ferro_property!(pub fn radius_y_property() -> StyledProperty<f64> {
        FerroProperty::register::<EllipseGeometry, _>("RadiusY", 0.0)
    });

    ferro_property!(pub fn center_property() -> StyledProperty<Point> {
        FerroProperty::register::<EllipseGeometry, _>("Center", Point::default())
    });
} }

impl EllipseGeometry {
    fn static_constructor() {
        Geometry::affects_geometry(&[
            Self::rect_property().as_property(),
            Self::radius_x_property().as_property(),
            Self::radius_y_property().as_property(),
            Self::center_property().as_property(),
        ]);
    }

    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: Geometry::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates an ellipse geometry filling the given bounds.
    pub fn with_rect(rect: Rect) -> Ref<Self> {
        let result = Self::new();
        result.set_rect(rect);
        result
    }

    /// A rect that defines the bounds of the ellipse. When set, this takes priority over the other properties that define an ellipse using a center point and X/Y-axis radii.
    pub fn rect(&self) -> Rect {
        self.get_value(Self::rect_property())
    }

    pub fn set_rect(&self, value: Rect) {
        self.set_value(Self::rect_property(), value)
    }

    /// A double that defines the radius in the X-axis of the ellipse. In other words, the horizontal half-extent of the ellipse.
    pub fn radius_x(&self) -> f64 {
        self.get_value(Self::radius_x_property())
    }

    pub fn set_radius_x(&self, value: f64) {
        self.set_value(Self::radius_x_property(), value)
    }

    /// A double that defines the radius in the Y-axis of the ellipse. In other words, the vertical half-extent of the ellipse.
    pub fn radius_y(&self) -> f64 {
        self.get_value(Self::radius_y_property())
    }

    pub fn set_radius_y(&self, value: f64) {
        self.set_value(Self::radius_y_property(), value)
    }

    /// A point that defines the center of the ellipse.
    pub fn center(&self) -> Point {
        self.get_value(Self::center_property())
    }

    pub fn set_center(&self, value: Point) {
        self.set_value(Self::center_property(), value)
    }
}
