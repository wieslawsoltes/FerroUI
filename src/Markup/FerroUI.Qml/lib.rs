//! A component owns its objects, bindings, dependency graph and signal dispatch.
//! Native integrations register their type contracts before constructing an instance.
//! Public mutations settle bindings and handlers atomically before exposing changes.
mod evaluation;
mod instantiate;
mod registry;
mod transaction;
mod value;

pub use ferroui_qml_compiler as ir;
pub use ferroui_qml_compiler::{Document, Parameter};
pub use registry::{NativeCallback, TypeDefinition, TypeRegistry};
pub use value::{Error, ObjectId, Value};

use ferroui_qml_compiler::{Binding, Stmt};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

type SlotId = usize;
type Locals = BTreeMap<String, Value>;

#[derive(Clone)]
struct Procedure {
    parameters: Vec<Parameter>,
    result: Option<String>,
    body: Vec<Stmt>,
}

struct Node {
    module: String,
    name: String,
    lineage: Vec<(String, String)>,
    parent: Option<ObjectId>,
    children: Vec<ObjectId>,
    properties: BTreeMap<String, SlotId>,
    default_property: Option<String>,
    signals: BTreeMap<String, Vec<Parameter>>,
    native: BTreeMap<String, registry::NativeMethod>,
    methods: BTreeMap<String, Rc<Procedure>>,
    handlers: BTreeMap<String, Rc<Procedure>>,
}

enum Alias {
    Slot(SlotId),
    Object(ObjectId),
}

struct Property {
    owner: ObjectId,
    name: String,
    ty: String,
    readonly: bool,
    required: bool,
    alias: Option<Alias>,
}

#[derive(Clone)]
struct Slot {
    value: Value,
    binding: Option<Rc<Binding>>,
    dirty: bool,
    reads: BTreeSet<SlotId>,
    dependents: BTreeSet<SlotId>,
}

/// A settled property value ready for a native host to apply.
#[derive(Clone, Debug, PartialEq)]
pub struct Change {
    pub object: ObjectId,
    pub property: String,
    pub value: Value,
}

type Validator = Box<dyn Fn(&Instance, &[Change]) -> Result<(), Error>>;

/// One observed emission from a successfully committed mutation.
#[derive(Clone, Debug, PartialEq)]
pub struct SignalEmission {
    pub object: ObjectId,
    pub signal: String,
    pub arguments: Vec<Value>,
}

pub struct Instance {
    validator: Option<Validator>,
    watched: BTreeSet<(ObjectId, String)>,
    emissions: Vec<SignalEmission>,
    nodes: Vec<Node>,
    ids: BTreeMap<String, ObjectId>,
    properties: Vec<Property>,
    slots: Vec<Slot>,
    changes: BTreeMap<SlotId, Value>,
    identity: u64,
}

impl Instance {
    pub fn root(&self) -> ObjectId {
        ObjectId {
            index: 0,
            instance: self.identity,
        }
    }

    pub fn id(&self, name: &str) -> Option<ObjectId> {
        if name.starts_with("$qml:") {
            return None;
        }
        self.ids.get(name).copied()
    }

    fn node(&self, object: ObjectId) -> Option<&Node> {
        (object.instance == self.identity)
            .then(|| self.nodes.get(object.index))
            .flatten()
    }

    pub fn type_name(&self, object: ObjectId) -> Option<&str> {
        self.node(object).map(|node| node.name.as_str())
    }

    pub fn module(&self, object: ObjectId) -> Option<&str> {
        self.node(object).map(|node| node.module.as_str())
    }

    pub fn children(&self, object: ObjectId) -> &[ObjectId] {
        self.node(object)
            .map(|node| node.children.as_slice())
            .unwrap_or_default()
    }

    pub fn parent(&self, object: ObjectId) -> Option<ObjectId> {
        self.node(object).and_then(|node| node.parent)
    }

    pub fn property_names(&self, object: ObjectId) -> Vec<String> {
        self.node(object)
            .map(|n| n.properties.keys().cloned().collect())
            .unwrap_or_default()
    }

    fn key(&self, object: ObjectId, property: &str) -> Result<SlotId, Error> {
        let node = self
            .node(object)
            .ok_or_else(|| Error::new("foreign or unknown object"))?;
        node.properties
            .get(property)
            .copied()
            .ok_or_else(|| Error::new(format!("{} has no property '{property}'", node.name)))
    }

    pub fn get(&self, object: ObjectId, property: &str) -> Result<Value, Error> {
        Ok(self.slots[self.key(object, property)?].value.clone())
    }

    /// A native projection can reject unsupported final property values before
    /// any change is published. A validator must have no external side effects.
    pub fn set_validator(
        &mut self,
        validate: impl Fn(&Instance, &[Change]) -> Result<(), Error> + 'static,
    ) {
        self.validator = Some(Box::new(validate));
    }

    /// Observe a declared signal from Rust. Emissions are released only after
    /// the QML handler, dependent bindings and projection validator succeed.
    pub fn watch_signal(&mut self, object: ObjectId, name: &str) -> Result<(), Error> {
        let node = self
            .node(object)
            .ok_or_else(|| Error::new("foreign or unknown object"))?;
        if !node.signals.contains_key(name) {
            return Err(Error::new(format!("unknown signal '{name}'")));
        }
        self.watched.insert((object, name.to_owned()));
        Ok(())
    }

    pub fn unwatch_signal(&mut self, object: ObjectId, name: &str) {
        self.watched.remove(&(object, name.to_owned()));
    }

    pub fn drain_signals(&mut self) -> Vec<SignalEmission> {
        std::mem::take(&mut self.emissions)
    }

    /// Replaces an existing binding, even if its current value equals the assigned value.
    pub fn set(&mut self, object: ObjectId, property: &str, value: Value) -> Result<(), Error> {
        let key = self.key(object, property)?;
        self.transaction(|tx| tx.write(key, value))
    }

    pub fn emit(
        &mut self,
        object: ObjectId,
        signal: &str,
        arguments: Vec<Value>,
    ) -> Result<(), Error> {
        self.transaction(|tx| tx.signal(object, signal, arguments))
    }

    pub fn call(
        &mut self,
        object: ObjectId,
        name: &str,
        arguments: Vec<Value>,
    ) -> Result<Value, Error> {
        self.transaction(|tx| tx.invoke(object, name, arguments, &mut BTreeSet::new()))
    }

    /// Coalesces changes by property across successful transactions. Creation can
    /// be inspected via `get`; only subsequent mutations produce changes here.
    pub fn drain_changes(&mut self) -> Vec<Change> {
        std::mem::take(&mut self.changes)
            .into_iter()
            .map(|(key, value)| Change {
                object: self.properties[key].owner,
                property: self.properties[key].name.clone(),
                value,
            })
            .collect()
    }
}
