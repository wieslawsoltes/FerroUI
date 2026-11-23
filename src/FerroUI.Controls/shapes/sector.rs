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

ferroui_base::ferro_impl_classes!(Sector: FerroObjectImpl);

/// What the geometry of a sector is, by its angles and its size.
enum SectorFigure {
    /// A full turn or more: an ellipse in the rectangle.
    Full(Rect),
    /// No sweep: an empty stream geometry.
    Empty,
    /// A pie slice: an arc and the two lines to the centre.
    Slice { start: Point, end: Point, centre: Point, radius: Size, is_large_arc: bool },
}

impl ShapeImpl for Sector {
    fn create_defining_geometry(this: &Self) -> Option<Ref<Geometry>> {
        Some(match this.figure() {
            SectorFigure::Full(rect) => EllipseGeometry::with_rect(rect).upcast(),
            figure => {
                let geometry = StreamGeometry::new();
                Self::write(&geometry, figure);
                geometry.upcast()
            }
        })
    }

    /// The ellipse of a full sector takes the rectangle, and the stream geometry of a slice or an
    /// empty sector is written again (#805); a full sector that becomes a slice, or the other
    /// way, makes another geometry.
    fn update_defining_geometry(this: &Self, geometry: &Ref<Geometry>) -> bool {
        match this.figure() {
            SectorFigure::Full(rect) => {
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

    /// What the geometry of the sector is now.
    fn figure(&self) -> SectorFigure {
        let rect = Rect::from_size(self.bounds().size());
        let deflated_rect = rect.deflate(self.stroke_thickness() * 0.5);
        let sweep = self.sweep_angle();

        if sweep >= 360.0 || sweep <= -360.0 {
            return SectorFigure::Full(deflated_rect);
        }

        if sweep == 0.0 {
            return SectorFigure::Empty;
        }

        let (start_angle, end_angle) = MathUtilities::get_min_max_from_delta(
            MathUtilities::deg2rad(self.start_angle()),
            MathUtilities::deg2rad(sweep),
        );

        let centre = Point::new(rect.width * 0.5, rect.height * 0.5);
        let radius_x = deflated_rect.width * 0.5;
        let radius_y = deflated_rect.height * 0.5;
        SectorFigure::Slice {
            start: MathUtilities::get_ellipse_point(centre, radius_x, radius_y, start_angle),
            end: MathUtilities::get_ellipse_point(centre, radius_x, radius_y, end_angle),
            centre,
            radius: Size::new(radius_x, radius_y),
            is_large_arc: sweep.abs() > 180.0,
        }
    }

    /// Writes the figures of a slice or an empty sector into `geometry`, in place of the ones it
    /// has.
    fn write(geometry: &StreamGeometry, figure: SectorFigure) {
        let mut context = geometry.reopen();
        if let SectorFigure::Slice { start, end, centre, radius, is_large_arc } = figure {
            context.begin_figure(start, true);
            context.arc_to(end, radius, 0.0, is_large_arc, SweepDirection::Clockwise, true);
            context.line_to(centre, true);
            context.end_figure(true);
        }
        context.dispose();
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
