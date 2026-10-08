use crate::media::{Geometry, GeometryImpl, Transform};
use crate::platform::{self, IGeometryImpl};
use crate::reactive::IDisposable;
use crate::{
    ferro_class, ferro_property, instantiate, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Nullable, Ref, StyledProperty,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

/// Specifies the different methods by which two geometries can be combined.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum GeometryCombineMode {
    /// The two regions are combined by taking the union of both. The
    /// resulting geometry is geometry A + geometry B.
    #[default]
    Union = 0,

    /// The two regions are combined by taking the intersection of both. The
    /// new area consists of the overlapping region between the two
    /// geometries.
    Intersect = 1,

    /// The two regions are combined by taking the area that exists in the
    /// first region but not the second and the area that exists in the
    /// second region but not the first. The new region consists of
    /// (A-B) + (B-A), where A and B are geometries.
    Xor = 2,

    /// The second region is excluded from the first. Given two geometries, A
    /// and B, the area of geometry B is removed from the area of geometry A,
    /// producing a region that is A-B.
    Exclude = 3,
}

/// Represents a 2-D geometric shape defined by the combination of two
/// geometry objects.
#[repr(C)]
pub struct CombinedGeometry {
    base: Geometry,
    geometry1_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    geometry2_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class!(CombinedGeometry: Geometry);
crate::ferro_class_info!(CombinedGeometry { new: CombinedGeometry::new });

impl FerroObjectImpl for CombinedGeometry {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        let slot = if change.property() == Self::geometry1_property().as_property() {
            &this.geometry1_subscription
        } else if change.property() == Self::geometry2_property().as_property() {
            &this.geometry2_subscription
        } else {
            return;
        };

        if let Some(subscription) = slot.take() {
            subscription.dispose();
        }

        if let Some(new_value) = change.get_new_value::<Option<Ref<Geometry>>>() {
            let weak = this.to_ref().downgrade();
            let subscription = new_value.changed(move || {
                if let Some(this) = weak.upgrade() {
                    this.invalidate_geometry();
                }
            });
            *slot.borrow_mut() = Some(subscription);
        }
    }
}

impl GeometryImpl for CombinedGeometry {
    fn clone_geometry(this: &Self) -> Ref<Geometry> {
        CombinedGeometry::with_mode_and_transform(
            this.geometry_combine_mode(),
            this.geometry1(),
            this.geometry2(),
            this.transform(),
        )
        .upcast()
    }

    fn create_defining_geometry(this: &Self) -> Option<Arc<dyn IGeometryImpl>> {
        let g1 = this.geometry1().and_then(|g| g.platform_impl());
        let g2 = this.geometry2().and_then(|g| g.platform_impl());

        if let (Some(g1), Some(g2)) = (&g1, &g2) {
            let factory = platform::render_interface();
            return Some(factory.create_combined_geometry(this.geometry_combine_mode(), g1.clone(), g2.clone()));
        }

        if this.geometry_combine_mode() == GeometryCombineMode::Intersect {
            return None;
        }

        g1.or(g2)
    }
}

crate::ferro_properties! { impl CombinedGeometry {
    ferro_property!(pub fn geometry1_property() -> StyledProperty<Option<Ref<Geometry>>> {
        FerroProperty::register::<CombinedGeometry, _>("Geometry1", None)
    });

    ferro_property!(pub fn geometry2_property() -> StyledProperty<Option<Ref<Geometry>>> {
        FerroProperty::register::<CombinedGeometry, _>("Geometry2", None)
    });

    ferro_property!(pub fn geometry_combine_mode_property() -> StyledProperty<GeometryCombineMode> {
        FerroProperty::register::<CombinedGeometry, _>("GeometryCombineMode", GeometryCombineMode::Union)
    });
} }

impl CombinedGeometry {
    fn static_constructor() {
        Geometry::affects_geometry(&[
            Self::geometry1_property().as_property(),
            Self::geometry2_property().as_property(),
            Self::geometry_combine_mode_property().as_property(),
        ]);
    }

    /// Creates the class data.
    pub fn construct() -> Self {
        Self {
            base: Geometry::construct(),
            geometry1_subscription: RefCell::new(None),
            geometry2_subscription: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates the union of the two geometries.
    pub fn with_geometries(geometry1: Ref<Geometry>, geometry2: Ref<Geometry>) -> Ref<Self> {
        let result = Self::new();
        result.set_geometry1(geometry1);
        result.set_geometry2(geometry2);
        result
    }

    /// Creates a combination of the two geometries with the given mode.
    pub fn with_mode(
        combine_mode: GeometryCombineMode,
        geometry1: Option<Ref<Geometry>>,
        geometry2: Option<Ref<Geometry>>,
    ) -> Ref<Self> {
        let result = Self::new();
        result.set_geometry1(geometry1);
        result.set_geometry2(geometry2);
        result.set_geometry_combine_mode(combine_mode);
        result
    }

    /// Creates a combination of the two geometries with the given mode and
    /// transform.
    pub fn with_mode_and_transform(
        combine_mode: GeometryCombineMode,
        geometry1: Option<Ref<Geometry>>,
        geometry2: Option<Ref<Geometry>>,
        transform: impl Into<Nullable<Transform>>,
    ) -> Ref<Self> {
        let result = Self::new();
        result.set_geometry1(geometry1);
        result.set_geometry2(geometry2);
        result.set_geometry_combine_mode(combine_mode);
        result.set_transform(transform);
        result
    }

    /// The first geometry that should be combined.
    pub fn geometry1(&self) -> Option<Ref<Geometry>> {
        self.get_value(Self::geometry1_property())
    }

    pub fn set_geometry1(&self, value: impl Into<Nullable<Geometry>>) {
        self.set_value(Self::geometry1_property(), value.into().0)
    }

    /// The second geometry that should be combined.
    pub fn geometry2(&self) -> Option<Ref<Geometry>> {
        self.get_value(Self::geometry2_property())
    }

    pub fn set_geometry2(&self, value: impl Into<Nullable<Geometry>>) {
        self.set_value(Self::geometry2_property(), value.into().0)
    }

    /// The method by which the two geometries (specified by the
    /// [`geometry1`](Self::geometry1) and [`geometry2`](Self::geometry2)
    /// properties) are combined. The default value is
    /// [`GeometryCombineMode::Union`].
    pub fn geometry_combine_mode(&self) -> GeometryCombineMode {
        self.get_value(Self::geometry_combine_mode_property())
    }

    pub fn set_geometry_combine_mode(&self, value: GeometryCombineMode) {
        self.set_value(Self::geometry_combine_mode_property(), value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use crate::media::{GeometryGroup, PathGeometry};

    #[test]
    fn child_geometry_change_invalidates() {
        let child = PathGeometry::new();
        let target = CombinedGeometry::with_mode(GeometryCombineMode::Xor, Some(child.clone().upcast()), None);
        let count = Rc::new(Cell::new(0));
        let c = count.clone();
        target.changed(move || c.set(c.get() + 1));

        child.set_fill_rule(crate::media::FillRule::NonZero);
        // PathGeometry.FillRule is not registered with affects_geometry, so
        // nothing is raised.
        assert_eq!(0, count.get());

        child.set_figures(None);
        child.set_figures(Some(crate::media::PathFigures::new()));
        child.figures().unwrap().add(crate::media::PathFigure::new());
        assert_eq!(1, count.get());

        target.set_geometry1(None);
        assert_eq!(2, count.get());
        child.figures().unwrap().add(crate::media::PathFigure::new());
        assert_eq!(2, count.get());
    }

    #[test]
    fn missing_operands() {
        let target = CombinedGeometry::new();
        assert!(target.platform_impl().is_none());
        target.set_geometry_combine_mode(GeometryCombineMode::Intersect);
        target.set_geometry1(GeometryGroup::new());
        assert!(target.platform_impl().is_none());
        assert_eq!(GeometryCombineMode::Intersect, target.geometry_combine_mode());
        let clone = target.clone_geometry();
        assert!(clone.is::<CombinedGeometry>());
    }
}
