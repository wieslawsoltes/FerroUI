use super::{Shape, ShapeImpl};
use crate::ControlImpl;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::{Geometry, RectangleGeometry};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroObjectImpl, FerroProperty,
    Rect, Ref, Size, StyledElementImpl, StyledProperty, Visual, VisualImpl,
};

/// Represents a rectangle with optional rounded corners.
#[repr(C)]
pub struct Rectangle {
    base: Shape,
}

ferro_class!(Rectangle: Shape);
ferroui_base::ferro_class_info!(Rectangle { new: Rectangle::new });
ferro_impl_classes!(Rectangle: StyledElementImpl, VisualImpl, InteractiveImpl, InputElementImpl, ControlImpl);

impl FerroObjectImpl for Rectangle {}

impl ShapeImpl for Rectangle {
    fn create_defining_geometry(this: &Self) -> Option<Ref<Geometry>> {
        let rect = Rect::from_size(this.bounds().size()).deflate(this.stroke_thickness() / 2.0);

        Some(RectangleGeometry::with_rect_and_radii(rect, this.radius_x(), this.radius_y()).upcast())
    }
}

impl LayoutableImpl for Rectangle {
    fn measure_override(this: &Self, _available_size: Size) -> Size {
        Size::new(this.stroke_thickness(), this.stroke_thickness())
    }
}

ferroui_base::ferro_properties! { impl Rectangle {
    ferro_property!(
        /// Defines the `RadiusX` property.
        pub fn radius_x_property() -> StyledProperty<f64> {
            FerroProperty::register::<Rectangle, _>("RadiusX", 0.0)
        }
    );

    ferro_property!(
        /// Defines the `RadiusY` property.
        pub fn radius_y_property() -> StyledProperty<f64> {
            FerroProperty::register::<Rectangle, _>("RadiusY", 0.0)
        }
    );
} }

impl Rectangle {
    fn static_constructor() {
        Shape::affects_geometry::<Rectangle>(&[
            Visual::bounds_property().as_property(),
            Self::radius_x_property().as_property(),
            Self::radius_y_property().as_property(),
            Shape::stroke_thickness_property().as_property(),
        ]);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: Shape::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets the radius in the X-axis of the ellipse that rounds the corners
    /// of the rectangle.
    pub fn radius_x(&self) -> f64 {
        self.get_value(Self::radius_x_property())
    }

    /// Sets the radius in the X-axis of the ellipse that rounds the corners
    /// of the rectangle.
    pub fn set_radius_x(&self, value: f64) {
        self.set_value(Self::radius_x_property(), value)
    }

    /// Gets the radius in the Y-axis of the ellipse that rounds the corners
    /// of the rectangle.
    pub fn radius_y(&self) -> f64 {
        self.get_value(Self::radius_y_property())
    }

    /// Sets the radius in the Y-axis of the ellipse that rounds the corners
    /// of the rectangle.
    pub fn set_radius_y(&self, value: f64) {
        self.set_value(Self::radius_y_property(), value)
    }
}
