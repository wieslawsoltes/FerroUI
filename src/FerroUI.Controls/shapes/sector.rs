use super::{Shape, ShapeImpl};
use crate::ControlImpl;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::{EllipseGeometry, Geometry, StreamGeometry, SweepDirection};
use ferroui_base::platform::IGeometryContext;
use ferroui_base::utilities::MathUtilities;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroObjectImpl, FerroProperty,
    Point, Rect, Ref, Size, StyledElementImpl, StyledProperty, Visual, VisualImpl,
};

/// Represents a circular or elliptical sector (a pie-shaped closed region of
/// a circle or ellipse).
#[repr(C)]
pub struct Sector {
    base: Shape,
}

ferro_class!(Sector: Shape);
ferroui_base::ferro_class_info!(Sector { new: Sector::new });
ferro_impl_classes!(
    Sector: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl FerroObjectImpl for Sector {}

impl ShapeImpl for Sector {
    fn create_defining_geometry(this: &Self) -> Option<Ref<Geometry>> {
        let rect = Rect::from_size(this.bounds().size());
        let deflated_rect = rect.deflate(this.stroke_thickness() * 0.5);
        let sweep = this.sweep_angle();

        if sweep >= 360.0 || sweep <= -360.0 {
            return Some(EllipseGeometry::with_rect(deflated_rect).upcast());
        }

        if sweep == 0.0 {
            return Some(StreamGeometry::new().upcast());
        }

        let (start_angle, end_angle) = MathUtilities::get_min_max_from_delta(
            MathUtilities::deg2rad(this.start_angle()),
            MathUtilities::deg2rad(sweep),
        );

        let centre = Point::new(rect.width * 0.5, rect.height * 0.5);
        let radius_x = deflated_rect.width * 0.5;
        let radius_y = deflated_rect.height * 0.5;
        let start_curve_point = MathUtilities::get_ellipse_point(centre, radius_x, radius_y, start_angle);
        let end_curve_point = MathUtilities::get_ellipse_point(centre, radius_x, radius_y, end_angle);
        let size = Size::new(radius_x, radius_y);

        let stream_geometry = StreamGeometry::new();
        {
            let mut context = stream_geometry.open();
            context.begin_figure(start_curve_point, true);
            context.arc_to(end_curve_point, size, 0.0, sweep.abs() > 180.0, SweepDirection::Clockwise, true);
            context.line_to(centre, true);
            context.end_figure(true);
            context.dispose();
        }

        Some(stream_geometry.upcast())
    }
}

ferroui_base::ferro_properties! { impl Sector {
    ferro_property!(
        /// Defines the `StartAngle` property.
        pub fn start_angle_property() -> StyledProperty<f64> {
            FerroProperty::register::<Sector, _>("StartAngle", 0.0)
        }
    );

    ferro_property!(
        /// Defines the `SweepAngle` property.
        pub fn sweep_angle_property() -> StyledProperty<f64> {
            FerroProperty::register::<Sector, _>("SweepAngle", 0.0)
        }
    );
} }

impl Sector {
    fn static_constructor() {
        Shape::stroke_thickness_property().override_default_value::<Sector>(1.0);
        Shape::affects_geometry::<Sector>(&[
            Visual::bounds_property().as_property(),
            Shape::stroke_thickness_property().as_property(),
            Self::start_angle_property().as_property(),
            Self::sweep_angle_property().as_property(),
        ]);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: Shape::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets the angle at which the sector's arc starts, in degrees.
    pub fn start_angle(&self) -> f64 {
        self.get_value(Self::start_angle_property())
    }

    /// Sets the angle at which the sector's arc starts, in degrees.
    pub fn set_start_angle(&self, value: f64) {
        self.set_value(Self::start_angle_property(), value)
    }

    /// Gets the angle, in degrees, added to the `StartAngle` defining where
    /// the sector's arc ends. A positive value is clockwise, negative is
    /// counter-clockwise.
    pub fn sweep_angle(&self) -> f64 {
        self.get_value(Self::sweep_angle_property())
    }

    /// Sets the angle, in degrees, added to the `StartAngle` defining where
    /// the sector's arc ends. A positive value is clockwise, negative is
    /// counter-clockwise.
    pub fn set_sweep_angle(&self, value: f64) {
        self.set_value(Self::sweep_angle_property(), value)
    }
}
