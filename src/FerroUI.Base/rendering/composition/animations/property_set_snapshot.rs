use crate::rendering::composition::expressions::{
    ExpressionVariant, IExpressionObject, IExpressionParameterCollection,
};
use crate::rendering::composition::server::{IAnimatedServerObject, ServerCompositor, ServerExpressionObject, ServerObjectId};
use std::cell::OnceCell;
use std::collections::HashMap;
use std::rc::Rc;

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
    Server(ServerObjectId, OnceCell<ServerExpressionObject>),
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
            PropertySetSnapshotObject::Server(_, resolved) => resolved.get().map(|o| o as &dyn IExpressionObject),
        }
    }

    /// The parameter as a server object, when it refers to one (the
    /// `is ServerObject` test of upstream on `GetObjectParameter`).
    pub fn get_server_object_parameter(&self, name: &str) -> Option<Rc<dyn IAnimatedServerObject>> {
        match self.dic.get(name)?.object.as_ref()? {
            PropertySetSnapshotObject::PropertySet(_) => None,
            PropertySetSnapshotObject::Server(_, resolved) => resolved.get().map(|o| o.0.clone()),
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
                        let _ = resolved.set(ServerExpressionObject(object));
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
