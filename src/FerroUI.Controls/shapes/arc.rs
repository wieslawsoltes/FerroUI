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
use std::f64::consts::PI;

/// Represents a circular or elliptical arc (a segment of a curve).
#[repr(C)]
pub struct Arc {
    base: Shape,
}

ferro_class!(Arc: Shape);
ferroui_base::ferro_class_info!(Arc { new: Arc::new });
ferro_impl_classes!(
    Arc: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

ferroui_base::ferro_impl_classes!(Arc: FerroObjectImpl);

impl ShapeImpl for Arc {
    fn create_defining_geometry(this: &Self) -> Option<Ref<Geometry>> {
        let angle1 = MathUtilities::deg2rad(this.start_angle());
        let angle2 = angle1 + MathUtilities::deg2rad(this.sweep_angle());

        let start_angle = f64::min(angle1, angle2);
        let sweep_angle = f64::max(angle1, angle2);

        let norm_start = Self::rad_to_norm_rad(start_angle);
        let norm_end = Self::rad_to_norm_rad(sweep_angle);

        let rect = Rect::from_size(this.bounds().size());

        if norm_start == norm_end && start_angle != sweep_angle {
            // Complete ring.
            Some(EllipseGeometry::with_rect(rect.deflate(this.stroke_thickness() / 2.0)).upcast())
        } else if this.sweep_angle() == 0.0 {
            Some(StreamGeometry::new().upcast())
        } else {
            // Partial arc.
            let deflated_rect = rect.deflate(this.stroke_thickness() / 2.0);

            let center_x = rect.center().x;
            let center_y = rect.center().y;

            let radius_x = deflated_rect.width / 2.0;
            let radius_y = deflated_rect.height / 2.0;

            let angle_gap = Self::rad_to_norm_rad(sweep_angle - start_angle);

            let start_point = Self::get_ring_point(radius_x, radius_y, center_x, center_y, start_angle);
            let end_point = Self::get_ring_point(radius_x, radius_y, center_x, center_y, sweep_angle);

            let arc_geometry = StreamGeometry::new();

            {
                let mut context = arc_geometry.open();
                context.begin_figure(start_point, false);
                context.arc_to(
                    end_point,
                    Size::new(radius_x, radius_y),
                    angle_gap,
                    angle_gap >= PI,
                    SweepDirection::Clockwise,
                    true,
                );
                context.end_figure(false);
                context.dispose();
            }

            Some(arc_geometry.upcast())
        }
    }
}

ferroui_base::ferro_properties! { impl Arc {
    ferro_property!(
        /// Defines the `StartAngle` property.
        pub fn start_angle_property() -> StyledProperty<f64> {
            FerroProperty::register::<Arc, _>("StartAngle", 0.0)
        }
    );

    ferro_property!(
        /// Defines the `SweepAngle` property.
        pub fn sweep_angle_property() -> StyledProperty<f64> {
            FerroProperty::register::<Arc, _>("SweepAngle", 0.0)
        }
    );
} }

impl Arc {
    fn static_constructor() {
        Shape::stroke_thickness_property().override_default_value::<Arc>(1.0);
        Shape::affects_geometry::<Arc>(&[
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

    /// Gets the angle at which the arc starts, in degrees.
    pub fn start_angle(&self) -> f64 {
        self.get_value(Self::start_angle_property())
    }

    /// Sets the angle at which the arc starts, in degrees.
    pub fn set_start_angle(&self, value: f64) {
        self.set_value(Self::start_angle_property(), value)
    }

    /// Gets the angle, in degrees, added to the `StartAngle` defining where
    /// the arc ends. A positive value is clockwise, negative is
    /// counter-clockwise.
    pub fn sweep_angle(&self) -> f64 {
        self.get_value(Self::sweep_angle_property())
    }

    /// Sets the angle, in degrees, added to the `StartAngle` defining where
    /// the arc ends. A positive value is clockwise, negative is
    /// counter-clockwise.
    pub fn set_sweep_angle(&self, value: f64) {
        self.set_value(Self::sweep_angle_property(), value)
    }

    fn rad_to_norm_rad(in_angle: f64) -> f64 {
        ((in_angle % (PI * 2.0)) + (PI * 2.0)) % (PI * 2.0)
    }

    fn get_ring_point(radius_x: f64, radius_y: f64, center_x: f64, center_y: f64, angle: f64) -> Point {
        Point::new((radius_x * angle.cos()) + center_x, (radius_y * angle.sin()) + center_y)
    }
}
