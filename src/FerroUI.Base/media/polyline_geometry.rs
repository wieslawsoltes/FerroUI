use crate::media::{FillRule, Geometry, GeometryImpl, Points};
use crate::platform::{self, IGeometryImpl};
use crate::reactive::IDisposable;
use crate::{
    ferro_class, ferro_property, instantiate, DirectProperty, FerroObjectImpl, FerroProperty,
    Point, Ref, StyledProperty,
};
use std::cell::RefCell;
use std::rc::Rc;

/// Represents the geometry of a polyline or polygon.
#[repr(C)]
pub struct PolylineGeometry {
    base: Geometry,
    points: RefCell<Option<Points>>,
    points_observer: RefCell<Option<Rc<dyn IDisposable>>>,
    fill_rule: FillRule,
}

ferro_class!(PolylineGeometry: Geometry);
crate::ferro_class_info!(PolylineGeometry { new: PolylineGeometry::new });

impl FerroObjectImpl for PolylineGeometry {}

impl GeometryImpl for PolylineGeometry {
    fn clone_geometry(this: &Self) -> Ref<Geometry> {
        let result = PolylineGeometry::with_points_and_fill_rule(this.points().iter(), this.is_filled(), this.fill_rule);
        result.set_transform(this.transform());
        result.upcast()
    }

    fn create_defining_geometry(this: &Self) -> Option<Rc<dyn IGeometryImpl>> {
        let factory = platform::render_interface();
        let geometry = factory.create_stream_geometry();

        let mut context = geometry.open();
        context.set_fill_rule(this.fill_rule);

        let points = this.points().to_vec();
        let is_filled = this.is_filled();

        if !points.is_empty() {
            context.begin_figure(points[0], is_filled);
            for point in &points[1..] {
                context.line_to(*point, true);
            }
            context.end_figure(is_filled);
        }
        context.dispose();

        Some(geometry)
    }
}

crate::ferro_properties! { impl PolylineGeometry {
    ferro_property!(pub fn points_property() -> DirectProperty<PolylineGeometry, Option<Points>> {
        FerroProperty::register_direct::<PolylineGeometry, _>(
            "Points",
            |g| g.points.borrow().clone(),
            Some(|g, f| {
                g.set_and_raise(PolylineGeometry::points_property(), &g.points, f);
            }),
            None,
        )
    });

    ferro_property!(pub fn is_filled_property() -> StyledProperty<bool> {
        FerroProperty::register::<PolylineGeometry, _>("IsFilled", false)
    });
} }

impl PolylineGeometry {
    fn static_constructor() {
        Geometry::affects_geometry(&[Self::is_filled_property().as_property()]);
        Self::points_property().changed().add_class_handler::<PolylineGeometry>(|s, e| {
            s.on_points_changed(e.get_new_value::<Option<Points>>())
        });
    }

    /// Creates the class data.
    pub fn construct() -> Self {
        Self::construct_with(Points::new(), FillRule::EvenOdd)
    }

    fn construct_with(points: Points, fill_rule: FillRule) -> Self {
        Self {
            base: Geometry::construct(),
            points: RefCell::new(Some(points)),
            points_observer: RefCell::new(None),
            fill_rule,
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates a polyline geometry with a copy of the given points.
    pub fn with_points(points: impl IntoIterator<Item = Point>, is_filled: bool) -> Ref<Self> {
        Self::with_points_and_fill_rule(points, is_filled, FillRule::EvenOdd)
    }

    /// Creates a polyline geometry with a copy of the given points and the
    /// given fill rule.
    pub fn with_points_and_fill_rule(
        points: impl IntoIterator<Item = Point>,
        is_filled: bool,
        fill_rule: FillRule,
    ) -> Ref<Self> {
        let result = instantiate(Self::construct_with(Points::from_items(points), fill_rule));
        result.set_is_filled(is_filled);
        result
    }

    /// The figure's points. Panics if the property has been cleared.
    pub fn points(&self) -> Points {
        self.points.borrow().clone().expect("Points is not set")
    }

    pub fn set_points(&self, value: Points) {
        self.set_and_raise(Self::points_property(), &self.points, Some(value));
    }

    /// Whether the geometry is filled.
    pub fn is_filled(&self) -> bool {
        self.get_value(Self::is_filled_property())
    }

    pub fn set_is_filled(&self, value: bool) {
        self.set_value(Self::is_filled_property(), value)
    }

    /// How the intersecting areas of the geometry are combined.
    pub fn fill_rule(&self) -> FillRule {
        self.fill_rule
    }

    fn on_points_changed(&self, new_value: Option<Points>) {
        if let Some(observer) = self.points_observer.take() {
            observer.dispose();
        }

        if let Some(new_value) = new_value {
            let weak = self.to_ref().downgrade();
            let observer = new_value.collection_changed(move |_| {
                if let Some(this) = weak.upgrade() {
                    this.invalidate_geometry();
                }
            });
            *self.points_observer.borrow_mut() = Some(observer);
        }
    }
}
