use super::animations::{PropertySetSnapshot, PropertySetSnapshotSource, PropertySetSnapshotSourceValue};
use super::expressions::{ExpressionVariant, ExpressionVariantValue};
use super::{AsCompositionObject, CompositionObject, Compositor};
use crate::media::Color;
use crate::numerics::{Matrix3x2, Matrix4x4, Quaternion, Vector2, Vector3, Vector4};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// Property sets are composition objects that allow storage of key values pairs
/// that can be shared across the application and are not tied to the lifetime of another composition object.
/// Property sets are most commonly used with animations, where they maintain key-value pairs
/// that are referenced to drive portions of composition animations. Property sets
/// provide the ability to insert key-value pairs or retrieve a value for a given key.
/// A property set does not support a delete function – ensure you use property sets
/// to store values that will be shared across the application.
pub struct CompositionPropertySet {
    object: CompositionObject,
    variants: RefCell<HashMap<String, ExpressionVariant>>,
    objects: RefCell<HashMap<String, Rc<dyn AsCompositionObject>>>,
}

/// The result of reading a value of a [`CompositionPropertySet`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CompositionGetValueStatus {
    Succeeded,
    TypeMismatch,
    NotFound,
}

impl CompositionPropertySet {
    pub(crate) fn new(compositor: &Rc<Compositor>) -> Rc<CompositionPropertySet> {
        Rc::new(CompositionPropertySet {
            object: CompositionObject::new(compositor, None),
            variants: RefCell::new(HashMap::new()),
            objects: RefCell::new(HashMap::new()),
        })
    }

    pub fn compositor(&self) -> &Rc<Compositor> {
        self.object.compositor()
    }

    pub(crate) fn set(&self, key: &str, value: ExpressionVariant) {
        self.objects.borrow_mut().remove(key);
        self.variants.borrow_mut().insert(key.to_owned(), value);
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

    pub(crate) fn snapshot(&self) -> PropertySetSnapshot {
        self.snapshot_source().build()
    }

    /// The snapshot in the form that is sent to the server, which builds
    /// the snapshot from it.
    pub(crate) fn snapshot_source(&self) -> PropertySetSnapshotSource {
        self.snapshot_core(1)
    }

    fn snapshot_core(&self, allowed_nesting_level: i32) -> PropertySetSnapshotSource {
        let objects: Vec<(String, Rc<dyn AsCompositionObject>)> =
            self.objects.borrow().iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        let variants = self.variants.borrow();
        let mut dic = HashMap::with_capacity(objects.len() + variants.len());
        for (key, value) in objects {
            let type_name = value.composition_type_name();
            let server = value.as_composition_object().server();
            if let Some(ps) = value.as_property_set() {
                if allowed_nesting_level <= 0 {
                    panic!("PropertySet depth limit reached");
                }
                dic.insert(
                    key,
                    PropertySetSnapshotSourceValue::PropertySet(ps.snapshot_core(allowed_nesting_level - 1)),
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
