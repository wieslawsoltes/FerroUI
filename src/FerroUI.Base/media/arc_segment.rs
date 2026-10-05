use crate::media::{PathSegment, PathSegmentImpl, StreamGeometryContext, SweepDirection};
use crate::platform::IGeometryContext;
use crate::utilities::span_helpers::write_double;
use crate::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroObjectImpl, FerroProperty, Point, Size, Ref,
    StyledProperty,
};

/// An elliptical arc segment of a path figure.
#[repr(C)]
pub struct ArcSegment {
    base: PathSegment,
}

ferro_class!(ArcSegment: PathSegment);
crate::ferro_class_info!(ArcSegment { new: ArcSegment::new });
ferro_impl_classes!(ArcSegment: FerroObjectImpl);

impl PathSegmentImpl for ArcSegment {
    fn apply_to(this: &Self, ctx: &mut StreamGeometryContext) {
        ctx.arc_to(
            this.point(),
            this.size(),
            this.rotation_angle(),
            this.is_large_arc(),
            this.sweep_direction(),
            this.is_stroked(),
        );
    }

    fn to_string(this: &Self) -> String {
        let mut rotation_angle = String::new();
        let _ = write_double(&mut rotation_angle, this.rotation_angle());
        format!(
            "A {} {} {} {} {}",
            this.size(),
            rotation_angle,
            if this.is_large_arc() { 1 } else { 0 },
            this.sweep_direction() as i32,
            this.point()
        )
    }
}

crate::ferro_properties! { impl ArcSegment {
    ferro_property!(pub fn is_large_arc_property() -> StyledProperty<bool> {
        FerroProperty::register::<ArcSegment, _>("IsLargeArc", false)
    });

    ferro_property!(pub fn point_property() -> StyledProperty<Point> {
        FerroProperty::register::<ArcSegment, _>("Point", Point::default())
    });

    ferro_property!(pub fn rotation_angle_property() -> StyledProperty<f64> {
        FerroProperty::register::<ArcSegment, _>("RotationAngle", 0.0)
    });

    ferro_property!(pub fn size_property() -> StyledProperty<Size> {
        FerroProperty::register::<ArcSegment, _>("Size", Size::default())
    });

    ferro_property!(pub fn sweep_direction_property() -> StyledProperty<SweepDirection> {
        FerroProperty::register::<ArcSegment, _>("SweepDirection", SweepDirection::Clockwise)
    });
} }

impl ArcSegment {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: PathSegment::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Whether the arc is the large arc (greater than 180 degrees).
    pub fn is_large_arc(&self) -> bool {
        self.get_value(Self::is_large_arc_property())
    }

    pub fn set_is_large_arc(&self, value: bool) {
        self.set_value(Self::is_large_arc_property(), value)
    }

    /// The end point of the arc.
    pub fn point(&self) -> Point {
        self.get_value(Self::point_property())
    }

    pub fn set_point(&self, value: Point) {
        self.set_value(Self::point_property(), value)
    }

    /// The rotation angle of the ellipse, in degrees.
    pub fn rotation_angle(&self) -> f64 {
        self.get_value(Self::rotation_angle_property())
    }

    pub fn set_rotation_angle(&self, value: f64) {
        self.set_value(Self::rotation_angle_property(), value)
    }

    /// The radii of the ellipse whose path is used to draw the arc.
    pub fn size(&self) -> Size {
        self.get_value(Self::size_property())
    }

    pub fn set_size(&self, value: Size) {
        self.set_value(Self::size_property(), value)
    }

    /// The direction in which the arc is drawn.
    pub fn sweep_direction(&self) -> SweepDirection {
        self.get_value(Self::sweep_direction_property())
    }

    pub fn set_sweep_direction(&self, value: SweepDirection) {
        self.set_value(Self::sweep_direction_property(), value)
    }
}
