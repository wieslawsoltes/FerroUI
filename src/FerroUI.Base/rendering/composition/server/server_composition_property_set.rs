use super::{
    CompositionProperty, IAnimatedServerObject, IServerAnimatedPropertyHost, IServerObject, IServerPropertyHost,
    ServerCompositionSurfaceChanged, ServerCompositor, ServerObject, ServerValueChange,
};
use crate::rendering::composition::animations::IAnimationInstance;
use crate::rendering::composition::animations::AnimationInstanceFactory;
use crate::rendering::composition::expressions::ExpressionVariant;
use crate::rendering::composition::transport::BatchStreamReader;
use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};
use std::time::Duration;

/// The changes of a property set that one batch carries: the entries that were inserted, in
/// the order of insertion, then the animations that were started.
#[derive(Default)]
pub(crate) struct PropertySetChanges {
    pub(crate) values: Vec<(String, ExpressionVariant)>,
    pub(crate) animations: Vec<(String, AnimationInstanceFactory)>,
}

/// The server-side counterpart of a live
/// [`CompositionPropertySet`](crate::rendering::composition::CompositionPropertySet): its
/// entries, animatable, readable by the expressions that refer to the set.
///
/// Not from upstream, whose property sets have no server side: an expression sees a copy of a
/// referenced set taken when its animation starts. WinUI's property sets are live and their
/// entries can be animated, which the WinUI composition API relies on (the property sets of
/// `ElementCompositionPreview`, the effect property set of a connected animation, the
/// interaction tracker).
pub struct ServerCompositionPropertySet {
    object: ServerObject,
    values: RefCell<HashMap<i32, ExpressionVariant>>,
    /// Raised when an entry changed, directly or by an animation: what reads
    /// the entries outside expressions follows them (the effect brushes,
    /// whose `Properties` hold their animatable properties).
    changed: ServerCompositionSurfaceChanged,
}

impl ServerCompositionPropertySet {
    /// Creates the property set, active: its animations run from the start, as a property set
    /// is not part of a tree that activates it.
    pub fn new(compositor: &Rc<ServerCompositor>) -> Rc<ServerCompositionPropertySet> {
        let set = Rc::new_cyclic(|this: &Weak<ServerCompositionPropertySet>| ServerCompositionPropertySet {
            object: ServerObject::new(compositor, this.clone() as Weak<dyn IAnimatedServerObject>),
            values: RefCell::new(HashMap::new()),
            changed: ServerCompositionSurfaceChanged::new(),
        });
        set.object.activate();
        set
    }

    /// The current value of an entry, animated or not; the default variant for an entry the set
    /// does not have.
    pub fn get_value(&self, name: &str) -> ExpressionVariant {
        self.object.get_property(name)
    }

    fn property(name: &str) -> &'static CompositionProperty {
        CompositionProperty::dynamic(name)
    }

    /// Raised when an entry of the set changed, by a batch or by an
    /// animation (not from upstream; the effect brushes follow the entries
    /// that are their animatable properties).
    pub fn changed(&self) -> &ServerCompositionSurfaceChanged {
        &self.changed
    }
}


impl IServerObject for ServerCompositionPropertySet {
    fn deserialize_changes_core(&self, reader: &mut BatchStreamReader<'_>, committed_at: Duration) {
        let Some(changes) = reader.read_value::<PropertySetChanges>() else { return };
        for (name, value) in changes.values {
            let property = Self::property(&name);
            self.values.borrow_mut().insert(property.id(), value);
            // A value set directly replaces the animation of the entry, and the expressions that
            // read the entry are invalidated (`SetValue` of a declared property).
            self.object.remove_animation_for_property(property);
        }
        for (name, factory) in changes.animations {
            let property = Self::property(&name);
            let current = self.values.borrow().get(&property.id()).copied().unwrap_or_default();
            self.object.set_animated_value(property, current, committed_at, factory.create());
        }
        self.changed.invoke();
    }

    fn as_animated(self: Rc<Self>) -> Option<Rc<dyn IAnimatedServerObject>> {
        Some(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}

impl IServerPropertyHost for ServerCompositionPropertySet {
    fn server_compositor(&self) -> Option<Rc<ServerCompositor>> {
        self.object.compositor()
    }

    fn set_value(&self, property: &'static CompositionProperty, change: ServerValueChange<'_>) {
        self.object.set_value(property, change);
    }
}

impl IServerAnimatedPropertyHost for ServerCompositionPropertySet {
    fn set_animated_value(
        &self,
        property: &'static CompositionProperty,
        current_value: ExpressionVariant,
        committed_at: Duration,
        animation: Rc<dyn IAnimationInstance>,
    ) {
        self.object.set_animated_value(property, current_value, committed_at, animation);
    }

    fn remove_animation_for_property(&self, property: &'static CompositionProperty) {
        self.object.remove_animation_for_property(property);
    }

    fn notify_animated_value_changed(&self, _property: &'static CompositionProperty) {
        self.changed.invoke();
    }
}

impl IAnimatedServerObject for ServerCompositionPropertySet {
    fn server_object(&self) -> &ServerObject {
        &self.object
    }

    /// Every name is an entry of a property set; one that was never inserted reads as the
    /// default variant, as a missing parameter does.
    fn get_composition_property(&self, field_name: &str) -> Option<&'static CompositionProperty> {
        Some(Self::property(field_name))
    }

    fn as_server_object_dyn(&self) -> &dyn IServerObject {
        self
    }

    fn read_property_variant(&self, property: &'static CompositionProperty) -> ExpressionVariant {
        self.values.borrow().get(&property.id()).copied().unwrap_or_default()
    }

    fn write_property_variant(&self, property: &'static CompositionProperty, value: ExpressionVariant) {
        self.values.borrow_mut().insert(property.id(), value);
    }
}
