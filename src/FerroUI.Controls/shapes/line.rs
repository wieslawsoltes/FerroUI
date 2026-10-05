use super::{Shape, ShapeImpl};
use crate::ControlImpl;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::{Geometry, LineGeometry};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroObjectImpl, FerroProperty,
    Point, Ref, StyledElementImpl, StyledProperty, VisualImpl,
};

/// Represents a straight line between two points.
#[repr(C)]
pub struct Line {
    base: Shape,
}

ferro_class!(Line: Shape);
ferroui_base::ferro_class_info!(Line { new: Line::new });
ferro_impl_classes!(
    Line: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl FerroObjectImpl for Line {}

impl ShapeImpl for Line {
    fn create_defining_geometry(this: &Self) -> Option<Ref<Geometry>> {
        Some(LineGeometry::with_points(this.start_point(), this.end_point()).upcast())
    }
}

ferroui_base::ferro_properties! { impl Line {
    ferro_property!(
        /// Defines the `StartPoint` property.
        pub fn start_point_property() -> StyledProperty<Point> {
            FerroProperty::register::<Line, _>("StartPoint", Point::default())
        }
    );

    ferro_property!(
        /// Defines the `EndPoint` property.
        pub fn end_point_property() -> StyledProperty<Point> {
            FerroProperty::register::<Line, _>("EndPoint", Point::default())
        }
    );
} }

impl Line {
    fn static_constructor() {
        Shape::stroke_thickness_property().override_default_value::<Line>(1.0);
        Shape::affects_geometry::<Line>(&[
            Self::start_point_property().as_property(),
            Self::end_point_property().as_property(),
        ]);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: Shape::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The point at which the line starts.
    pub fn start_point(&self) -> Point {
        self.get_value(Self::start_point_property())
    }

    pub fn set_start_point(&self, value: Point) {
        self.set_value(Self::start_point_property(), value)
    }

    /// The point at which the line ends.
    pub fn end_point(&self) -> Point {
        self.get_value(Self::end_point_property())
    }

    pub fn set_end_point(&self, value: Point) {
        self.set_value(Self::end_point_property(), value)
    }
}
