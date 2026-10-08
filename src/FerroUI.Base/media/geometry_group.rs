use crate::media::{FillRule, Geometry, GeometryCollection, GeometryImpl};
use crate::platform::{self, IGeometryImpl};
use crate::{
    ferro_class, ferro_property, instantiate, DirectProperty, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Ref, StyledProperty,
};
use std::cell::RefCell;
use std::sync::Arc;

/// Represents a composite geometry, composed of other [`Geometry`] objects.
#[repr(C)]
pub struct GeometryGroup {
    base: Geometry,
    children: RefCell<Option<GeometryCollection>>,
}

ferro_class!(GeometryGroup: Geometry);
crate::ferro_class_info!(GeometryGroup { new: GeometryGroup::new });

impl FerroObjectImpl for GeometryGroup {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.children_or_panic().set_parent(Some(&this.to_ref()));
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        match change.property().name() {
            "FillRule" | "Children" => this.invalidate_geometry(),
            _ => {}
        }
    }
}

impl GeometryImpl for GeometryGroup {
    fn clone_geometry(this: &Self) -> Ref<Geometry> {
        let result = GeometryGroup::new();
        result.set_fill_rule(this.fill_rule());
        result.set_transform(this.transform());
        let children = this.children_or_panic();
        if !children.is_empty() {
            result.set_children(GeometryCollection::from_items(children.iter()));
        }
        result.upcast()
    }

    fn create_defining_geometry(this: &Self) -> Option<Arc<dyn IGeometryImpl>> {
        let children = this.children_or_panic();
        if children.is_empty() {
            return None;
        }

        let factory = platform::render_interface();
        // Children without a platform implementation contribute nothing.
        let children: Vec<Arc<dyn IGeometryImpl>> = children.iter().filter_map(|child| child.platform_impl()).collect();

        Some(factory.create_geometry_group(this.fill_rule(), &children))
    }
}

crate::ferro_properties! { impl GeometryGroup {
    ferro_property!(pub fn children_property() -> DirectProperty<GeometryGroup, Option<GeometryCollection>> {
        FerroProperty::register_direct::<GeometryGroup, _>(
            "Children",
            |o| o.children.borrow().clone(),
            Some(|o, v| o.set_children_core(v)),
            None,
        )
    });

    ferro_property!(pub fn fill_rule_property() -> StyledProperty<FillRule> {
        FerroProperty::register::<GeometryGroup, _>("FillRule", FillRule::EvenOdd)
    });
} }

impl GeometryGroup {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: Geometry::construct(), children: RefCell::new(Some(GeometryCollection::new())) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    fn children_or_panic(&self) -> GeometryCollection {
        self.children.borrow().clone().expect("Children is not set")
    }

    /// The collection that contains the child geometries. Panics if the
    /// property has been cleared.
    pub fn children(&self) -> GeometryCollection {
        self.children_or_panic()
    }

    pub fn set_children(&self, value: GeometryCollection) {
        self.set_children_core(Some(value));
    }

    fn set_children_core(&self, value: Option<GeometryCollection>) {
        let old = self.children.borrow().clone();
        self.on_children_changed(old.as_ref(), value.as_ref());
        self.set_and_raise(Self::children_property(), &self.children, value);
    }

    /// How the intersecting areas of the children are combined. The default
    /// is [`FillRule::EvenOdd`].
    pub fn fill_rule(&self) -> FillRule {
        self.get_value(Self::fill_rule_property())
    }

    pub fn set_fill_rule(&self, value: FillRule) {
        self.set_value(Self::fill_rule_property(), value)
    }

    fn on_children_changed(&self, old_children: Option<&GeometryCollection>, new_children: Option<&GeometryCollection>) {
        if let Some(old_children) = old_children {
            old_children.set_parent(None);
        }
        if let Some(new_children) = new_children {
            new_children.set_parent(Some(&self.to_ref()));
        }
    }

    pub(crate) fn invalidate(&self) {
        self.invalidate_geometry();
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;
    use super::*;
    use crate::media::StreamGeometry;
    use std::cell::Cell;

    #[test]
    fn children_should_have_initial_collection() {
        let target = GeometryGroup::new();
        assert!(target.children().is_empty());
        assert!(target.children().parent().unwrap().ptr_eq(&target));
    }

    #[test]
    fn children_change_should_raise_changed() {
        let target = GeometryGroup::new();

        let children = GeometryCollection::new();

        target.set_children(children.clone());

        let is_called = Rc::new(Cell::new(false));
        let c = is_called.clone();
        target.changed(move || c.set(true));

        children.add(StreamGeometry::new().upcast());

        assert!(is_called.get());
    }

    #[test]
    fn replaced_children_no_longer_raise_changed() {
        let target = GeometryGroup::new();
        let old = target.children();
        target.set_children(GeometryCollection::new());
        assert!(old.parent().is_none());

        let is_called = Rc::new(Cell::new(false));
        let c = is_called.clone();
        target.changed(move || c.set(true));

        old.add(StreamGeometry::new().upcast());

        assert!(!is_called.get());
    }

    #[test]
    fn empty_group_has_no_platform_impl() {
        let target = GeometryGroup::new();
        assert!(target.platform_impl().is_none());
        assert_eq!(crate::Rect::default(), target.bounds());
    }
}
