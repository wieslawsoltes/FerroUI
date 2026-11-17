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

/// Represents a closed figure made of straight lines between points.
#[repr(C)]
pub struct Polygon {
    base: Shape,
}

ferro_class!(Polygon: Shape);
ferroui_base::ferro_class_info!(Polygon { new: Polygon::new });
ferro_impl_classes!(
    Polygon: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl FerroObjectImpl for Polygon {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        this.set_value_with_priority(Self::points_property(), Some(Points::new()), BindingPriority::Template);
    }
}

impl ShapeImpl for Polygon {
    fn create_defining_geometry(this: &Self) -> Option<Ref<Geometry>> {
        // The geometry is made of the live collection, so that it observes changes to it
        // (#20565).
        let geometry = match this.points() {
            Some(points) => PolylineGeometry::of_points(points, true, this.fill_rule()),
            None => PolylineGeometry::with_points_and_fill_rule(std::iter::empty(), true, this.fill_rule()),
        };
        Some(geometry.upcast())
    }

    /// The polyline geometry takes the collection of points the shape has now and follows it.
    /// The fill rule is what a polyline geometry is made with: another one is another geometry.
    fn update_defining_geometry(this: &Self, geometry: &Ref<Geometry>) -> bool {
        let (Some(geometry), Some(points)) = (geometry.cast::<PolylineGeometry>(), this.points()) else {
            return false;
        };
        if geometry.fill_rule() != this.fill_rule() {
            return false;
        }
        geometry.set_is_filled(true);
        if !geometry.points().ptr_eq(&points) {
            geometry.set_points(points);
            // The geometry follows the changes of a collection, not the change of collection.
            geometry.invalidate_geometry();
        }
        true
    }
}

ferroui_base::ferro_properties! { impl Polygon {
    ferro_property!(
        /// Defines the `Points` property.
        pub fn points_property() -> StyledProperty<Option<Points>> {
            FerroProperty::register::<Polygon, _>("Points", None)
        }
    );

    ferro_property!(
        /// Defines the `FillRule` property.
        pub fn fill_rule_property() -> StyledProperty<FillRule> {
            FerroProperty::register::<Polygon, _>("FillRule", FillRule::EvenOdd)
        }
    );
} }

impl Polygon {
    fn static_constructor() {
        Shape::affects_geometry::<Polygon>(&[
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

    /// The vertices of the polygon.
    pub fn points(&self) -> Option<Points> {
        self.get_value(Self::points_property())
    }

    pub fn set_points(&self, value: Option<Points>) {
        self.set_value(Self::points_property(), value)
    }

    /// Gets how the interior of the polygon is determined when a fill is
    /// applied.
    pub fn fill_rule(&self) -> FillRule {
        self.get_value(Self::fill_rule_property())
    }

    /// Sets how the interior of the polygon is determined when a fill is
    /// applied.
    pub fn set_fill_rule(&self, value: FillRule) {
        self.set_value(Self::fill_rule_property(), value)
    }
}
