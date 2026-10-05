use crate::rendering::composition::expressions::{
    ExpressionObjectKey, ExpressionVariant, IExpressionObject, IExpressionParameterCollection,
};
use crate::rendering::composition::server::{IAnimatedServerObject, ServerCompositor, ServerObjectId};
use std::cell::OnceCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

/// A snapshot of properties used by an animation
///
/// The snapshot is taken on the UI thread, where a reference parameter is
/// known by the id of its server object; the ids are resolved on the server
/// by [`resolve`](Self::resolve), before the animation instance that holds
/// the snapshot is initialized.
pub struct PropertySetSnapshot {
    dic: HashMap<String, PropertySetSnapshotValue>,
}

/// The object a value of a snapshot refers to.
pub enum PropertySetSnapshotObject {
    /// A nested property set.
    PropertySet(Rc<PropertySetSnapshot>),
    /// A composition object, by the id of its server object; the server
    /// object once resolved.
    Server(ServerObjectId, OnceCell<SnapshotServerObject>),
}

/// The server object a reference parameter refers to, held weakly.
///
/// Upstream the snapshot holds the server object, and the garbage collector
/// reclaims two animations that refer to each other's targets. Here the
/// animations of an object hold its instances, and an instance holds its
/// parameters, so a strong reference would form a cycle between two such
/// objects; the snapshot holds the object weakly, as the instance holds its
/// target. The object outlives its disposal while its composition object is
/// alive, so the reference goes dead only once nothing refers to the object
/// any more; a dead reference is read as a parameter without an object.
pub struct SnapshotServerObject(Weak<dyn IAnimatedServerObject>);

impl SnapshotServerObject {
    /// The server object, while it is alive.
    pub fn upgrade(&self) -> Option<Rc<dyn IAnimatedServerObject>> {
        self.0.upgrade()
    }

    fn is_alive(&self) -> bool {
        self.0.strong_count() > 0
    }
}

impl IExpressionObject for SnapshotServerObject {
    fn get_property(&self, name: &str) -> ExpressionVariant {
        match self.0.upgrade() {
            Some(object) => object.server_object().get_property(name),
            None => ExpressionVariant::default(),
        }
    }

    fn key(&self) -> ExpressionObjectKey {
        ExpressionObjectKey(Weak::as_ptr(&self.0) as *const () as usize)
    }
}

/// One value of a [`PropertySetSnapshot`]: a variant or an object.
#[derive(Default)]
pub struct PropertySetSnapshotValue {
    pub variant: ExpressionVariant,
    pub object: Option<PropertySetSnapshotObject>,
}

impl PropertySetSnapshotValue {
    /// A value that refers to an object.
    pub fn from_object(o: PropertySetSnapshotObject) -> Self {
        Self { object: Some(o), variant: ExpressionVariant::default() }
    }
}

impl From<ExpressionVariant> for PropertySetSnapshotValue {
    fn from(v: ExpressionVariant) -> Self {
        Self { variant: v, object: None }
    }
}

impl PropertySetSnapshot {
    pub fn new(dic: HashMap<String, PropertySetSnapshotValue>) -> Self {
        Self { dic }
    }

    pub fn get_parameter(&self, name: &str) -> ExpressionVariant {
        self.dic.get(name).map(|v| v.variant).unwrap_or_default()
    }

    pub fn get_object_parameter(&self, name: &str) -> Option<&dyn IExpressionObject> {
        match self.dic.get(name)?.object.as_ref()? {
            PropertySetSnapshotObject::PropertySet(snapshot) => Some(&**snapshot as &dyn IExpressionObject),
            PropertySetSnapshotObject::Server(_, resolved) => {
                resolved.get().filter(|o| o.is_alive()).map(|o| o as &dyn IExpressionObject)
            }
        }
    }

    /// The parameter as a server object, when it refers to one (the
    /// `is ServerObject` test of upstream on `GetObjectParameter`).
    pub fn get_server_object_parameter(&self, name: &str) -> Option<Rc<dyn IAnimatedServerObject>> {
        match self.dic.get(name)?.object.as_ref()? {
            PropertySetSnapshotObject::PropertySet(_) => None,
            PropertySetSnapshotObject::Server(_, resolved) => resolved.get().and_then(SnapshotServerObject::upgrade),
        }
    }

    pub fn get_property(&self, name: &str) -> ExpressionVariant {
        self.get_parameter(name)
    }

    /// Resolves the server objects the snapshot refers to on `compositor`,
    /// nested property sets included.
    ///
    /// Upstream the snapshot holds the server objects themselves. An id
    /// names its server object until the composition object that holds it is
    /// dropped, disposed or not, and the snapshot is resolved by the batch
    /// that carries it, which precedes the release of any object it refers
    /// to; an id that names no animatable object leaves the parameter
    /// without an object, as a null reference does upstream.
    pub fn resolve(&self, compositor: &ServerCompositor) {
        for value in self.dic.values() {
            match &value.object {
                Some(PropertySetSnapshotObject::PropertySet(snapshot)) => snapshot.resolve(compositor),
                Some(PropertySetSnapshotObject::Server(id, resolved)) => {
                    if resolved.get().is_some() {
                        continue;
                    }
                    if let Some(object) = compositor.get_animated_object(*id) {
                        let _ = resolved.set(SnapshotServerObject(Rc::downgrade(&object)));
                    }
                }
                None => {}
            }
        }
    }
}

impl IExpressionParameterCollection for PropertySetSnapshot {
    fn get_parameter(&self, name: &str) -> ExpressionVariant {
        PropertySetSnapshot::get_parameter(self, name)
    }

    fn get_object_parameter(&self, name: &str) -> Option<&dyn IExpressionObject> {
        PropertySetSnapshot::get_object_parameter(self, name)
    }
}

impl IExpressionObject for PropertySetSnapshot {
    fn get_property(&self, name: &str) -> ExpressionVariant {
        PropertySetSnapshot::get_property(self, name)
    }
}
