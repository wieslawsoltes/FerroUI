//! A component owns its objects, bindings, dependency graph and signal dispatch.
//! Native integrations register their type contracts before constructing an instance.
//! Public mutations settle bindings and handlers atomically before exposing changes.
mod animations;
mod arrays;
mod binding_object;
mod catalog;
pub mod compiled;
mod connections;
mod deferred;
mod dynamic;
mod incubation;
mod incubator;
pub use incubator::{IncubationMode, IncubationStatus, Incubator};
mod initial;
mod lifetime;
pub use compiled::{CompiledComponent, ComponentSource};
pub use initial::InitialProperty;
mod enums;
pub use enums::Namespace;
mod evaluation;
mod geometry;
pub use geometry::Geometry;
mod instantiate;
mod list_model;
mod loader;
pub mod network;
pub use list_model::RowId;
mod promises;
mod records;
mod repeater;
pub use promises::PromiseState;
mod registry;
pub use records::Record;
mod scripting;
mod signal_callbacks;
pub use scripting::{ExternalValue, ScriptContext, ScriptEngine, ScriptInstance, ScriptScope};
mod states;
mod timers;
mod transaction;
mod value;
pub mod workers;

pub use ferroui_qml_compiler as ir;
pub use ferroui_qml_compiler::{Document, Parameter};
pub use registry::{NativeCallback, TypeDefinition, TypeRegistry};
pub use value::{AnchorEdge, Error, MethodRef, ObjectId, Value};

use ferroui_qml_compiler::{Binding, Stmt};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

type SlotId = usize;
type Locals = BTreeMap<String, Value>;

#[derive(Clone)]
struct Procedure {
    scope: Option<u32>,
    parameters: Vec<Parameter>,
    result: Option<String>,
    body: Vec<Stmt>,
}

struct NodeEntry {
    id: ObjectId,
    live: bool,
    data: Option<Box<Node>>,
}
impl std::ops::Deref for NodeEntry {
    type Target = Node;
    fn deref(&self) -> &Node {
        self.data.as_deref().expect("access to retired QML object")
    }
}
impl std::ops::DerefMut for NodeEntry {
    fn deref_mut(&mut self) -> &mut Node {
        self.data
            .as_deref_mut()
            .expect("access to retired QML object")
    }
}
struct Node {
    source_url: Option<String>,
    source_layers: Vec<ir::SourceLayer>,
    context: Option<ObjectId>,
    visual_children: Option<String>,
    shadows: Vec<ir::PropertyShadow>,
    type_scope: Rc<Namespace>,
    module: String,
    name: String,
    lineage: Vec<(String, String)>,
    composite_types: Vec<String>,
    parent: Option<ObjectId>,
    children: Vec<ObjectId>,
    properties: BTreeMap<String, SlotId>,
    default_property: Option<String>,
    signals: BTreeMap<String, Vec<Parameter>>,
    native: BTreeMap<String, registry::NativeMethod>,
    methods: BTreeMap<String, Rc<Procedure>>,
    handlers: BTreeMap<String, Rc<Procedure>>,
}

impl Node {
    fn signal(&self, name: &str) -> Option<&[Parameter]> {
        if let Some(args) = self.signals.get(name) {
            return Some(args.as_slice());
        }
        name.strip_suffix("Changed")
            .filter(|n| self.properties.contains_key(*n))
            .map(|_| [].as_slice())
    }
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
enum InstalledBinding {
    Expression(Rc<Binding>),
    GeometryFields {
        base: Geometry,
        fields: Rc<Vec<(&'static str, SlotId)>>,
    },
    Guarded {
        when: SlotId,
        expression: Rc<Binding>,
    },
    StateExpression {
        declaration: ObjectId,
        receiver: ObjectId,
        expression: Rc<Binding>,
    },
    PropertyRead(SlotId),
    Guest {
        callback: ExternalValue,
        receiver: ObjectId,
        scope: Option<u32>,
    },
}

#[derive(Clone)]
struct Slot {
    delegate_local: bool,
    scope: Option<u32>,
    overlay: Option<Box<binding_object::Overlay>>,
    active_target: Option<SlotId>,
    assigned: bool,
    value: Value,
    binding: Option<InstalledBinding>,
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
    dynamic: dynamic::Components,
    javascript: scripting::ScriptState,
    models: BTreeMap<ObjectId, list_model::Model>,
    records: records::Heap,
    native_first_reads: &'static [compiled::FirstRead],
    bindings: BTreeMap<ObjectId, Vec<binding_object::Declaration>>,
    native_scripts: &'static [compiled::Script],
    native_functions: &'static [compiled::Evaluator],
    network: network::Network,
    workers: workers::Workers,
    timers: timers::Timers,
    listeners: BTreeMap<String, Vec<connections::Listener>>,
    validator: Option<Validator>,
    watched: BTreeSet<(ObjectId, String)>,
    emissions: Vec<SignalEmission>,
    nodes: Vec<NodeEntry>,
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
            serial: 0,
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
            .filter(|n| n.live && n.id == object)
            .and_then(|n| n.data.as_deref())
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
            .map(|n| {
                n.properties
                    .keys()
                    .filter(|name| !name.starts_with("$qml:"))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    fn public_key(&self, object: ObjectId, name: &str) -> Result<SlotId, Error> {
        if name.starts_with("$qml:") {
            return Err(Error::new("private QML property"));
        }
        self.key(object, name)
    }

    fn key(&self, object: ObjectId, property: &str) -> Result<SlotId, Error> {
        self.key_in(object, property, None)
    }

    fn key_in(
        &self,
        object: ObjectId,
        property: &str,
        scope: Option<u32>,
    ) -> Result<SlotId, Error> {
        let node = self
            .node(object)
            .ok_or_else(|| Error::new("foreign or unknown object"))?;
        let property = node
            .shadows
            .iter()
            .find(|s| Some(s.scope) == scope && s.name == property)
            .map_or(property, |s| s.storage.as_str());
        node.properties
            .get(property)
            .copied()
            .ok_or_else(|| Error::new(format!("{} has no property '{property}'", node.name)))
    }

    /// True if QML or Rust supplied a local value or binding. Reflection-based
    /// hosts use this to preserve theme/style precedence for untouched defaults.
    pub fn is_assigned(&self, object: ObjectId, property: &str) -> bool {
        self.public_key(object, property)
            .is_ok_and(|key| self.slots[key].assigned)
    }

    /// Obtain a bound method that can be stored in a var, passed to Rust or
    /// posted with Qt.callLater. Its receiver belongs to this instance.
    pub fn method(&self, object: ObjectId, name: &str) -> Result<MethodRef, Error> {
        if name.starts_with("$qml:") || !self.script_method(object, name) {
            return Err(Error::new(format!(
                "unknown public method or signal '{name}'"
            )));
        }
        Ok(MethodRef {
            receiver: object,
            name: name.into(),
        })
    }

    pub fn get(&self, object: ObjectId, property: &str) -> Result<Value, Error> {
        if let Some((key, field)) = self.geometry_path(object, property) {
            let Value::Geometry(shape) = self.slots[key].value else {
                unreachable!()
            };
            return Ok(Value::Number(shape.field(field).unwrap()));
        }
        Ok(self.slots[self.public_key(object, property)?].value.clone())
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
        if name.starts_with("$qml:") || node.signal(name).is_none() {
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
        if let Some((key, field)) = self.geometry_path(object, property) {
            return self.transaction(|tx| tx.write_geometry_slot(key, field, value));
        }
        let key = self.public_key(object, property)?;
        self.transaction(|tx| tx.write(key, value))
    }

    pub fn emit(
        &mut self,
        object: ObjectId,
        signal: &str,
        arguments: Vec<Value>,
    ) -> Result<(), Error> {
        if signal.starts_with("$qml:shadow:") {
            return Err(Error::new("private QML signal"));
        }
        self.transaction(|tx| tx.signal(object, signal, arguments))
    }

    pub fn call(
        &mut self,
        object: ObjectId,
        name: &str,
        arguments: Vec<Value>,
    ) -> Result<Value, Error> {
        self.transaction(|tx| {
            let value = tx.invoke(object, name, arguments, &mut BTreeSet::new())?;
            Ok(match tx.model.geometry_snapshot(&value)? {
                Some(shape) => Value::Geometry(shape),
                None => value,
            })
        })
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
