use super::{Shape, ShapeImpl};
use crate::ControlImpl;
use ferroui_base::data::BindingPriority;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::{FillRule, Geometry, Points, PolylineGeometry};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    Ref, StyledElementImpl, StyledProperty, VisualImpl,
};

/// Represents an open figure made of straight lines between points.
#[repr(C)]
pub struct Polyline {
    base: Shape,
}

ferro_class!(Polyline: Shape);
ferroui_base::ferro_class_info!(Polyline { new: Polyline::new });
ferro_impl_classes!(
    Polyline: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl FerroObjectImpl for Polyline {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        this.set_value_with_priority(Self::points_property(), Some(Points::new()), BindingPriority::Template);
    }
}

impl ShapeImpl for Polyline {
    fn create_defining_geometry(this: &Self) -> Option<Ref<Geometry>> {
        let is_filled = this.fill().is_some();
        let points = this.points().map(|p| p.to_vec()).unwrap_or_default();
        Some(PolylineGeometry::with_points_and_fill_rule(points, is_filled, this.fill_rule()).upcast())
    }
}

ferroui_base::ferro_properties! { impl Polyline {
    ferro_property!(
        /// Defines the `Points` property.
        pub fn points_property() -> StyledProperty<Option<Points>> {
            FerroProperty::register::<Polyline, _>("Points", None)
        }
    );

    ferro_property!(
        /// Defines the `FillRule` property.
        pub fn fill_rule_property() -> StyledProperty<FillRule> {
            FerroProperty::register::<Polyline, _>("FillRule", FillRule::EvenOdd)
        }
    );
} }

impl Polyline {
    fn static_constructor() {
        Shape::stroke_thickness_property().override_default_value::<Polyline>(1.0);
        Shape::affects_geometry::<Polyline>(&[
            Self::points_property().as_property(),
            Self::fill_rule_property().as_property(),
        ]);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: Shape::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The vertices of the polyline.
    pub fn points(&self) -> Option<Points> {
        self.get_value(Self::points_property())
    }

    pub fn set_points(&self, value: Option<Points>) {
        self.set_value(Self::points_property(), value)
    }

    /// Gets how the interior of the polyline is determined when a fill is
    /// applied.
    pub fn fill_rule(&self) -> FillRule {
        self.get_value(Self::fill_rule_property())
    }

    /// Sets how the interior of the polyline is determined when a fill is
    /// applied.
    pub fn set_fill_rule(&self, value: FillRule) {
        self.set_value(Self::fill_rule_property(), value)
    }
}
