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

/// What the geometry of an arc is, by its angles and its size.
enum ArcFigure {
    /// A complete ring: an ellipse in the rectangle.
    Ring(Rect),
    /// No sweep: an empty stream geometry.
    Empty,
    /// A partial arc: one figure of an elliptic arc.
    Partial { start_point: Point, end_point: Point, radius: Size, is_large_arc: bool },
}

impl ShapeImpl for Arc {
    fn create_defining_geometry(this: &Self) -> Option<Ref<Geometry>> {
        Some(match this.figure() {
            ArcFigure::Ring(rect) => EllipseGeometry::with_rect(rect).upcast(),
            figure => {
                let geometry = StreamGeometry::new();
                Self::write(&geometry, figure);
                geometry.upcast()
            }
        })
    }

    /// The ellipse of a ring takes the rectangle, and the stream geometry of a partial or an
    /// empty arc is written again (#805); a ring that becomes an arc, or the other way, makes
    /// another geometry.
    fn update_defining_geometry(this: &Self, geometry: &Ref<Geometry>) -> bool {
        match this.figure() {
            ArcFigure::Ring(rect) => {
                let Some(ellipse) = geometry.cast::<EllipseGeometry>() else { return false };
                ellipse.set_rect(rect);
            }
            figure => {
                let Some(stream) = geometry.cast::<StreamGeometry>() else { return false };
                Self::write(&stream, figure);
            }
        }
        true
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

    /// What the geometry of the arc is now.
    fn figure(&self) -> ArcFigure {
        let angle1 = MathUtilities::deg2rad(self.start_angle());
        let angle2 = angle1 + MathUtilities::deg2rad(self.sweep_angle());

        let start_angle = f64::min(angle1, angle2);
        let sweep_angle = f64::max(angle1, angle2);

        let norm_start = Self::rad_to_norm_rad(start_angle);
        let norm_end = Self::rad_to_norm_rad(sweep_angle);

        let rect = Rect::from_size(self.bounds().size());

        if norm_start == norm_end && start_angle != sweep_angle {
            // Complete ring.
            return ArcFigure::Ring(rect.deflate(self.stroke_thickness() / 2.0));
        }
        if self.sweep_angle() == 0.0 {
            return ArcFigure::Empty;
        }

        // Partial arc.
        let deflated_rect = rect.deflate(self.stroke_thickness() / 2.0);

        let center_x = rect.center().x;
        let center_y = rect.center().y;

        let radius_x = deflated_rect.width / 2.0;
        let radius_y = deflated_rect.height / 2.0;

        let angle_gap = Self::rad_to_norm_rad(sweep_angle - start_angle);

        ArcFigure::Partial {
            start_point: Self::get_ring_point(radius_x, radius_y, center_x, center_y, start_angle),
            end_point: Self::get_ring_point(radius_x, radius_y, center_x, center_y, sweep_angle),
            radius: Size::new(radius_x, radius_y),
            is_large_arc: angle_gap >= PI,
        }
    }

    /// Writes the figures of a partial or an empty arc into `geometry`, in place of the ones it
    /// has.
    fn write(geometry: &StreamGeometry, figure: ArcFigure) {
        let mut context = geometry.reopen();
        if let ArcFigure::Partial { start_point, end_point, radius, is_large_arc } = figure {
            context.begin_figure(start_point, false);
            context.arc_to(end_point, radius, 0.0, is_large_arc, SweepDirection::Clockwise, true);
            context.end_figure(false);
        }
        context.dispose();
    }

    fn rad_to_norm_rad(in_angle: f64) -> f64 {
        ((in_angle % (PI * 2.0)) + (PI * 2.0)) % (PI * 2.0)
    }

    fn get_ring_point(radius_x: f64, radius_y: f64, center_x: f64, center_y: f64, angle: f64) -> Point {
        Point::new((radius_x * angle.cos()) + center_x, (radius_y * angle.sin()) + center_y)
    }
}
