use super::animations::{ICompositionAnimation, PropertySetSnapshot, PropertySetSnapshotSource, PropertySetSnapshotSourceValue};
use super::expressions::{ExpressionVariant, ExpressionVariantValue};
use super::server::{CompositionProperty, PropertySetChanges, ServerCompositionPropertySet, ServerObjectId};
use super::transport::BatchStreamWriter;
use super::{AsCompositionObject, CompositionObject, Compositor, ICompositionObjectAnimations, ICompositorSerializable};
use crate::media::Color;
use crate::numerics::{Matrix3x2, Matrix4x4, Quaternion, Vector2, Vector3, Vector4};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

/// Property sets are composition objects that allow storage of key values pairs
/// that can be shared across the application and are not tied to the lifetime of another composition object.
/// Property sets are most commonly used with animations, where they maintain key-value pairs
/// that are referenced to drive portions of composition animations. Property sets
/// provide the ability to insert key-value pairs or retrieve a value for a given key.
/// A property set does not support a delete function – ensure you use property sets
/// to store values that will be shared across the application.
///
/// A property set made with [`new`](Self::new) (or `Compositor::create_property_set`) is live,
/// as in WinUI: it has a server counterpart that receives its entries with each batch, its
/// entries can be animated with `start_animation`, and an expression that refers to it reads
/// its current values. Upstream's property sets live on the UI thread only, and an expression
/// sees a copy of them taken when its animation starts; the parameters of an animation are such
/// a set (see `docs/porting/winui-composition.md`).
pub struct CompositionPropertySet {
    this: Weak<CompositionPropertySet>,
    object: CompositionObject,
    variants: RefCell<HashMap<String, ExpressionVariant>>,
    objects: RefCell<HashMap<String, Rc<dyn AsCompositionObject>>>,
    /// The values of a live set that the next batch carries.
    pending: RefCell<PropertySetChanges>,
    /// The entries whose animations the next batch starts; the animations themselves wait in
    /// the pending animations of the composition object, by entry.
    pending_animation_names: RefCell<Vec<String>>,
}

/// The result of reading a value of a [`CompositionPropertySet`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CompositionGetValueStatus {
    Succeeded,
    TypeMismatch,
    NotFound,
}

impl CompositionPropertySet {
    /// Creates a live property set (`Compositor.CreatePropertySet`).
    pub fn new(compositor: &Rc<Compositor>) -> Rc<CompositionPropertySet> {
        let server = compositor.create_server_object(|compositor, _| ServerCompositionPropertySet::new(compositor));
        Self::create(compositor, Some(server))
    }

    /// Creates a property set without a server counterpart: the parameters of an animation,
    /// which are copied when the animation starts.
    pub(crate) fn new_parameters(compositor: &Rc<Compositor>) -> Rc<CompositionPropertySet> {
        Self::create(compositor, None)
    }

    fn create(compositor: &Rc<Compositor>, server: Option<ServerObjectId>) -> Rc<CompositionPropertySet> {
        Rc::new_cyclic(|this| CompositionPropertySet {
            this: this.clone(),
            object: CompositionObject::new(compositor, server),
            variants: RefCell::new(HashMap::new()),
            objects: RefCell::new(HashMap::new()),
            pending: RefCell::new(PropertySetChanges::default()),
            pending_animation_names: RefCell::new(Vec::new()),
        })
    }

    /// Whether the set is live (has a server counterpart).
    pub fn is_live(&self) -> bool {
        self.object.server().is_some()
    }

    fn register_for_serialization(&self) {
        let this = self.this.clone();
        self.object.register_for_serialization(move || {
            this.upgrade().map(|this| this as Rc<dyn ICompositorSerializable>)
        });
    }

    pub fn compositor(&self) -> &Rc<Compositor> {
        self.object.compositor()
    }

    pub(crate) fn set(&self, key: &str, value: ExpressionVariant) {
        self.objects.borrow_mut().remove(key);
        self.variants.borrow_mut().insert(key.to_owned(), value);
        if self.is_live() && !self.object.is_disposed() {
            {
                let mut pending = self.pending.borrow_mut();
                // A value set after an animation was started in the same frame replaces it, as
                // the value of a declared property does.
                self.object.pending_animations().remove(CompositionProperty::dynamic(key));
                pending.values.retain(|(name, _)| name != key);
                pending.values.push((key.to_owned(), value));
            }
            self.register_for_serialization();
        }
    }

    // For INTERNAL USE by CompositionAnimation ONLY, we DON'T support expression
    // paths like SomeParam.SomePropertyObject.SomeValue
    pub(crate) fn set_object(&self, key: &str, obj: Rc<dyn AsCompositionObject>) {
        self.objects.borrow_mut().insert(key.to_owned(), obj);
        self.variants.borrow_mut().remove(key);
    }

    pub fn insert_color(&self, property_name: &str, value: Color) {
        self.set(property_name, value.into())
    }

    pub fn insert_matrix3x2(&self, property_name: &str, value: Matrix3x2) {
        self.set(property_name, value.into())
    }

    pub fn insert_matrix4x4(&self, property_name: &str, value: Matrix4x4) {
        self.set(property_name, value.into())
    }

    pub fn insert_quaternion(&self, property_name: &str, value: Quaternion) {
        self.set(property_name, value.into())
    }

    pub fn insert_scalar(&self, property_name: &str, value: f32) {
        self.set(property_name, value.into())
    }

    pub fn insert_vector2(&self, property_name: &str, value: Vector2) {
        self.set(property_name, value.into())
    }

    pub fn insert_vector3(&self, property_name: &str, value: Vector3) {
        self.set(property_name, value.into())
    }

    pub fn insert_vector4(&self, property_name: &str, value: Vector4) {
        self.set(property_name, value.into())
    }

    fn try_get_variant<T: ExpressionVariantValue>(&self, key: &str) -> (CompositionGetValueStatus, T) {
        let Some(v) = self.variants.borrow().get(key).copied() else {
            let status = if self.objects.borrow().contains_key(key) {
                CompositionGetValueStatus::TypeMismatch
            } else {
                CompositionGetValueStatus::NotFound
            };
            return (status, T::default());
        };

        match v.try_cast::<T>() {
            Some(value) => (CompositionGetValueStatus::Succeeded, value),
            None => (CompositionGetValueStatus::TypeMismatch, T::default()),
        }
    }

    pub fn try_get_color(&self, property_name: &str) -> (CompositionGetValueStatus, Color) {
        self.try_get_variant(property_name)
    }

    pub fn try_get_matrix3x2(&self, property_name: &str) -> (CompositionGetValueStatus, Matrix3x2) {
        self.try_get_variant(property_name)
    }

    pub fn try_get_matrix4x4(&self, property_name: &str) -> (CompositionGetValueStatus, Matrix4x4) {
        self.try_get_variant(property_name)
    }

    pub fn try_get_quaternion(&self, property_name: &str) -> (CompositionGetValueStatus, Quaternion) {
        self.try_get_variant(property_name)
    }

    pub fn try_get_scalar(&self, property_name: &str) -> (CompositionGetValueStatus, f32) {
        self.try_get_variant(property_name)
    }

    pub fn try_get_vector2(&self, property_name: &str) -> (CompositionGetValueStatus, Vector2) {
        self.try_get_variant(property_name)
    }

    pub fn try_get_vector3(&self, property_name: &str) -> (CompositionGetValueStatus, Vector3) {
        self.try_get_variant(property_name)
    }

    pub fn try_get_vector4(&self, property_name: &str) -> (CompositionGetValueStatus, Vector4) {
        self.try_get_variant(property_name)
    }

    pub fn insert_boolean(&self, property_name: &str, value: bool) {
        self.set(property_name, value.into())
    }

    pub fn try_get_boolean(&self, property_name: &str) -> (CompositionGetValueStatus, bool) {
        self.try_get_variant(property_name)
    }

    pub(crate) fn clear_all(&self) {
        self.objects.borrow_mut().clear();
        self.variants.borrow_mut().clear();
    }

    pub(crate) fn clear(&self, key: &str) {
        self.objects.borrow_mut().remove(key);
        self.variants.borrow_mut().remove(key);
    }

    /// A snapshot read on the UI thread: a live property set it refers to is copied too, as the
    /// UI thread cannot read its server counterpart.
    pub(crate) fn snapshot(&self) -> PropertySetSnapshot {
        self.snapshot_core(1, true).build()
    }

    /// The snapshot in the form that is sent to the server, which builds
    /// the snapshot from it. A live property set it refers to is referred to by id.
    pub(crate) fn snapshot_source(&self) -> PropertySetSnapshotSource {
        self.snapshot_core(1, false)
    }

    fn snapshot_core(&self, allowed_nesting_level: i32, copy_live_sets: bool) -> PropertySetSnapshotSource {
        let objects: Vec<(String, Rc<dyn AsCompositionObject>)> =
            self.objects.borrow().iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        let variants = self.variants.borrow();
        let mut dic = HashMap::with_capacity(objects.len() + variants.len());
        for (key, value) in objects {
            let type_name = value.composition_type_name();
            let server = value.as_composition_object().server();
            if let Some(ps) = value.clone().as_property_set().filter(|ps| copy_live_sets || !ps.is_live()) {
                if allowed_nesting_level <= 0 {
                    panic!("PropertySet depth limit reached");
                }
                dic.insert(
                    key,
                    PropertySetSnapshotSourceValue::PropertySet(ps.snapshot_core(allowed_nesting_level - 1, copy_live_sets)),
                );
            } else if let Some(server) = server {
                dic.insert(key, PropertySetSnapshotSourceValue::Server(server));
            } else {
                panic!("Object of type {type_name} is not allowed");
            }
        }

        for (key, value) in variants.iter() {
            dic.insert(key.clone(), PropertySetSnapshotSourceValue::Variant(*value));
        }

        PropertySetSnapshotSource::new(dic)
    }
}

impl AsCompositionObject for CompositionPropertySet {
    fn as_composition_object(&self) -> &CompositionObject {
        &self.object
    }

    fn composition_type_name(&self) -> &'static str {
        "CompositionPropertySet"
    }

    fn as_property_set(self: Rc<Self>) -> Option<Rc<CompositionPropertySet>> {
        Some(self)
    }
}

impl ICompositorSerializable for CompositionPropertySet {
    fn try_get_server(&self, c: &Compositor) -> Option<ServerObjectId> {
        self.object.try_get_server(c)
    }

    fn serialize_changes(&self, c: &Compositor, writer: &mut BatchStreamWriter<'_>) {
        self.object.begin_serialize_changes(c);
        let mut changes = std::mem::take(&mut *self.pending.borrow_mut());
        for name in std::mem::take(&mut *self.pending_animation_names.borrow_mut()) {
            if let Some(factory) = self.object.pending_animations().get_and_remove(CompositionProperty::dynamic(&name)) {
                changes.animations.push((name, factory));
            }
        }
        writer.write_value(Some(changes));
    }
}

impl ICompositionObjectAnimations for CompositionPropertySet {
    /// Starts an animation of an entry of a live set. As in WinUI the entry must have been
    /// inserted first; the animation starts from its current value.
    fn try_start_animation(
        &self,
        property_name: &str,
        animation: &dyn ICompositionAnimation,
        final_value: Option<ExpressionVariant>,
    ) -> bool {
        let Some(server) = self.object.server() else { return false };
        if !self.variants.borrow().contains_key(property_name) {
            return false;
        }
        let factory = animation.create_instance(server, final_value);
        self.object.pending_animations().set(CompositionProperty::dynamic(property_name), factory);
        {
            let mut names = self.pending_animation_names.borrow_mut();
            if !names.iter().any(|name| name == property_name) {
                names.push(property_name.to_owned());
            }
        }
        self.register_for_serialization();
        true
    }

    fn get_composition_property(&self, property_name: &str) -> Option<&'static CompositionProperty> {
        self.variants.borrow().contains_key(property_name).then(|| CompositionProperty::dynamic(property_name))
    }

    fn composition_object(&self) -> &CompositionObject {
        &self.object
    }
}
