//! Engine-neutral host interface. A context borrows one active QML transaction
//! and must be used synchronously on its owner thread. Nested calls reuse that
//! transaction and its dependency tracker. No raw pointer to a context escapes.
use super::*;
use crate::transaction::Transaction;
use std::any::Any;
use std::fmt;

/// Stable realm key for backends shared between component instances.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ScriptInstance(u64);

impl ScriptInstance {
    pub fn as_u64(self) -> u64 {
        self.0
    }
}

/// A backend-owned JS reference returned to a QML var or Rust caller. Equal ids
/// in the same realm represent the same object. The adapter owns the payload's
/// lifetime (e.g. a persistent function handle in its JS engine).
#[derive(Clone)]
pub struct ExternalValue {
    instance: ScriptInstance,
    id: u64,
    payload: Rc<dyn Any>,
}

impl ExternalValue {
    pub fn id(&self) -> u64 {
        self.id
    }
    pub fn instance(&self) -> ScriptInstance {
        self.instance
    }
    pub fn downcast<T: Any>(&self) -> Option<Rc<T>> {
        self.payload.clone().downcast().ok()
    }
}
impl PartialEq for ExternalValue {
    fn eq(&self, other: &Self) -> bool {
        self.instance == other.instance && self.id == other.id
    }
}
impl fmt::Debug for ExternalValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExternalValue")
            .field("instance", &self.instance)
            .field("id", &self.id)
            .finish()
    }
}

/// A JavaScript environment. Implementations may retain independent realms
/// keyed by ScriptInstance. They must tolerate nested evaluate calls during a
/// QML host call. Very long guest computations need their own execution limit.
pub trait ScriptEngine {
    fn evaluate(
        &self,
        context: &mut ScriptContext<'_, '_>,
        script: &ir::JavaScript,
    ) -> Result<Value, Error>;

    /// Check a retained value; the backend must return within its configured
    /// execution limit and must not wait beyond the supplied timeout for a
    /// pending Promise. A zero duration requests an immediate inspection.
    fn observe_promise(
        &self,
        _context: &mut ScriptContext<'_, '_>,
        _value: &ExternalValue,
        _timeout: std::time::Duration,
    ) -> Result<PromiseState, Error> {
        Err(Error::new(
            "the JavaScript backend does not support Promise observation",
        ))
    }

    /// Optional VM transaction hooks for engine-owned state. QML slots,
    /// records, models, timers and signal emission have their own undo logs.
    fn start_worker(
        &self,
        _resources: std::sync::Arc<BTreeMap<String, ir::ScriptResource>>,
        _origin: &str,
        _source: &str,
        _events: workers::WorkerDelivery,
    ) -> Result<Box<dyn workers::WorkerEndpoint>, Error> {
        Err(Error::new("script backend has no WorkerScript support"))
    }
    fn begin_transaction(&self, _instance: ScriptInstance) -> Result<(), Error> {
        Ok(())
    }
    fn end_transaction(&self, _instance: ScriptInstance, _committed: bool) {}
    /// Release projected QML identities on committed destruction or failed construction.
    fn release_objects(&self, _instance: ScriptInstance, _objects: &BTreeSet<ObjectId>) {}
    fn release_instance(&self, _instance: ScriptInstance) {}
    /// Permanently expired callbacks may be discarded after their worker realm restarts.
    fn is_reference_live(&self, _value: &ExternalValue) -> bool {
        true
    }

    /// Snapshot the elements of a guest array while retaining object elements by
    /// identity. Return None for non-arrays or backends without array projection.
    fn array_elements(
        &self,
        _context: &mut ScriptContext<'_, '_>,
        _value: &ExternalValue,
        _limit: usize,
    ) -> Result<Option<Vec<Value>>, Error> {
        Ok(None)
    }

    /// Optional interop when the built-in evaluator reads a foreign JS object.
    fn get_external(
        &self,
        _context: &mut ScriptContext<'_, '_>,
        _value: &ExternalValue,
        _name: &str,
    ) -> Result<Value, Error> {
        Err(Error::new(
            "the JavaScript backend does not support external property reads",
        ))
    }
    fn set_external(
        &self,
        _context: &mut ScriptContext<'_, '_>,
        _value: &ExternalValue,
        _name: &str,
        _input: Value,
    ) -> Result<(), Error> {
        Err(Error::new(
            "the JavaScript backend does not support external property writes",
        ))
    }
    /// Evaluate an imperative property binding. Backends can use the target type
    /// to marshal typed lists while retaining ordinary JS objects by identity.
    fn evaluate_binding(
        &self,
        context: &mut ScriptContext<'_, '_>,
        callback: &ExternalValue,
        receiver: ObjectId,
        _property_type: &str,
    ) -> Result<Value, Error> {
        self.call_external(context, callback, Value::Object(receiver), Vec::new())
    }
    fn call_external(
        &self,
        _context: &mut ScriptContext<'_, '_>,
        _value: &ExternalValue,
        _receiver: Value,
        _args: Vec<Value>,
    ) -> Result<Value, Error> {
        Err(Error::new(
            "the JavaScript backend does not support calling external values",
        ))
    }
}

#[derive(Default)]
pub(crate) struct ScriptState {
    pub(crate) deferred: deferred::Queue,
    pub(crate) connections: signal_callbacks::SignalCallbacks,
    pub(crate) engine: Option<Rc<dyn ScriptEngine>>,
    pub(crate) resources: std::sync::Arc<BTreeMap<String, ir::ScriptResource>>,
}

/// Reusable lexical QML scope for closures. It contains no reference to the
/// current transaction and cannot be used with a different engine instance.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ScriptScope {
    owner: ObjectId,
    root: ObjectId,
    declaration: Option<u32>,
    ids: Rc<BTreeMap<String, String>>,
}

impl ScriptScope {
    pub fn owner(&self) -> ObjectId {
        self.owner
    }
    pub fn root(&self) -> ObjectId {
        self.root
    }
}

pub struct ScriptContext<'a, 'model> {
    tx: &'a mut Transaction<'model>,
    owner: ObjectId,
    root: ObjectId,
    script: Option<&'a ir::JavaScript>,
    ids: Option<&'a BTreeMap<String, String>>,
    locals: &'a mut Locals,
    reads: &'a mut BTreeSet<SlotId>,
}

impl ScriptContext<'_, '_> {
    pub fn capture_scope(&self) -> ScriptScope {
        ScriptScope {
            owner: self.owner,
            root: self.root,
            declaration: self.tx.declaration_scope(),
            ids: Rc::new(self.ids.cloned().unwrap_or_default()),
        }
    }

    /// The lexical QML scope of a classic JavaScript file imported by the
    /// component. Unqualified names refer to the component root irrespective of
    /// which child first accesses an export.
    pub fn capture_component_scope(&self) -> ScriptScope {
        ScriptScope {
            owner: self.root,
            ..self.capture_scope()
        }
    }

    pub fn in_scope<T>(
        &mut self,
        scope: &ScriptScope,
        op: impl FnOnce(&mut ScriptContext<'_, '_>) -> Result<T, Error>,
    ) -> Result<T, Error> {
        if self.tx.model.node(scope.owner).is_none() {
            return Err(Error::new("foreign or expired JavaScript scope"));
        }
        let reads = &mut *self.reads;
        self.tx.with_scope(scope.declaration, |tx| {
            op(&mut ScriptContext {
                tx,
                owner: scope.owner,
                root: scope.root,
                script: None,
                ids: Some(scope.ids.as_ref()),
                locals: &mut Locals::new(),
                reads,
            })
        })
    }

    /// Absolute deadline shared by a bounded call, including nested guest jobs.
    pub fn deadline(&self) -> Option<std::time::Instant> {
        self.tx.script_deadline
    }

    /// Execute a microtask on the current owner thread. Captured QML scope is
    /// restored by name resolution; reads belong to this continuation.
    pub fn in_continuation<T>(
        &mut self,
        op: impl FnOnce(&mut ScriptContext<'_, '_>) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let root = self.tx.model.root();
        self.tx.with_scope(None, |tx| {
            op(&mut ScriptContext {
                tx,
                owner: root,
                root,
                script: None,
                ids: None,
                locals: &mut Locals::new(),
                reads: &mut BTreeSet::new(),
            })
        })
    }

    pub fn instance(&self) -> ScriptInstance {
        self.tx.model.script_instance()
    }
    pub fn owner(&self) -> ObjectId {
        self.owner
    }
    pub fn root(&self) -> ObjectId {
        self.root
    }
    pub fn in_binding(&self) -> bool {
        self.tx.in_binding()
    }
    pub fn expected_type(&self) -> Option<&str> {
        match self.script?.form {
            ir::ScriptForm::Expression | ir::ScriptForm::BindingBlock => self
                .tx
                .current_binding
                .map(|key| self.tx.model.properties[key].ty.as_str()),
            ir::ScriptForm::FunctionBody => self.tx.guest_return_type.as_deref(),
            ir::ScriptForm::HandlerBody => Some("void"),
        }
    }
    pub fn property_type(&self, object: ObjectId, name: &str) -> Option<&str> {
        if private(name) {
            return None;
        }
        let mut key = self.tx.scoped_key(object, name).ok()?;
        loop {
            let property = &self.tx.model.properties[key];
            match property.alias {
                Some(Alias::Slot(target)) => key = target,
                Some(Alias::Object(target)) => return self.tx.model.type_name(target),
                None => return Some(&property.ty),
            }
        }
    }
    pub fn argument_types(&self, object: ObjectId, method: &str) -> Vec<Option<&str>> {
        let Some(node) = self.tx.model.node(object) else {
            return Vec::new();
        };
        let params = node
            .methods
            .get(method)
            .map(|m| m.parameters.as_slice())
            .or_else(|| node.native.get(method).map(|m| m.parameters.as_slice()))
            .or_else(|| node.signal(method))
            .unwrap_or_default();
        params.iter().map(|p| p.ty.as_deref()).collect()
    }
    pub fn create_with_properties(
        &mut self,
        factory: ObjectId,
        parent: Option<ObjectId>,
        props: BTreeMap<String, InitialProperty>,
    ) -> Result<Value, Error> {
        let result = self
            .tx
            .try_create_with_properties(factory, parent, props)
            .map(|id| id.map(Value::Object).unwrap_or(Value::Null));
        self.tx.host_result(result)
    }
    pub fn incubate_with_properties(
        &mut self,
        factory: ObjectId,
        parent: Option<ObjectId>,
        props: BTreeMap<String, InitialProperty>,
        mode: IncubationMode,
    ) -> Result<Value, Error> {
        let result = self
            .tx
            .start_incubation(factory, parent, props, mode)
            .map(|h| h.map_or(Value::Null, |h| Value::Record(h.record().clone())));
        self.tx.host_result(result)
    }

    pub fn force_incubation(&mut self, handle: &Record) -> Result<(), Error> {
        let result = self.tx.force_incubator(handle);
        self.tx.host_result(result)
    }
    /// URL helpers share the QML declaration context with component factories.
    pub fn url_from_arguments(
        &self,
        name: &str,
        origin: Option<&str>,
        args: &[Value],
    ) -> Result<Value, Error> {
        let origin = origin.or_else(|| self.script.map(|script| script.origin.as_str()));
        self.tx
            .model
            .url_utility(self.owner, self.tx.declaration_scope(), origin, name, args)
    }

    pub fn component_from_arguments(
        &mut self,
        origin: Option<&str>,
        args: &[Value],
    ) -> Result<Value, Error> {
        self.tx.component_call(self.owner, origin, args)
    }
    pub fn qml_from_arguments(&mut self, args: &[Value]) -> Result<Value, Error> {
        let before = self.tx.model.structure_revision();
        let result = self.tx.text_component_call(self.owner, args);
        if result.is_err() && before != self.tx.model.structure_revision() {
            self.tx.host_result(result)
        } else {
            result
        }
    }
    pub fn is_loader(&self, object: ObjectId) -> bool {
        self.tx.model.dynamic.loaders.contains_key(&object)
    }
    pub fn set_loader_source(
        &mut self,
        object: ObjectId,
        url: String,
        properties: BTreeMap<String, InitialProperty>,
    ) -> Result<(), Error> {
        let origin = self
            .script
            .map(|script| script.origin.clone())
            .filter(|s| !s.is_empty());
        self.tx.set_loader_source(object, url, origin, properties)?;
        self.tx.settle()
    }
    pub fn is_component(&self, object: ObjectId) -> bool {
        self.tx.model.is_component(object)
    }
    pub fn is_worker_script(&self, id: ObjectId) -> bool {
        self.tx.model.workers.contains(id)
    }
    pub fn is_list_model(&self, object: ObjectId) -> bool {
        self.tx.model.models.contains_key(&object)
    }
    pub fn name_type(&self, name: &str) -> Option<&str> {
        if self.locals.contains_key(name) || self.id(name).is_some() {
            return None;
        }
        [self.owner, self.root, self.tx.model.root()]
            .into_iter()
            .find_map(|o| self.property_type(o, name))
    }
    pub fn keys(&self, base: &Value) -> Result<Vec<String>, Error> {
        self.tx.model.validate_handles(base)?;
        Ok(match base {
            Value::Object(o) => self.members(*o),
            Value::Record(r) => self.record_keys(r)?,
            Value::List(items) => (0..items.len())
                .map(|i| i.to_string())
                .chain(std::iter::once("length".into()))
                .collect(),
            Value::ModelRow(row) => self.tx.model.model_keys(*row)?,
            Value::DelegateRow(owner) => self.tx.model.delegate_row(*owner)?.keys(),
            Value::AnchorGroup(owner) => self
                .tx
                .model
                .property_names(*owner)
                .into_iter()
                .filter_map(|p| p.strip_prefix("anchors.").map(str::to_owned))
                .collect(),
            Value::Namespace(namespace) => namespace.names().map(str::to_owned).collect(),
            _ => Vec::new(),
        })
    }

    pub fn argument_type(&self, object: ObjectId, method: &str, index: usize) -> Option<&str> {
        let node = self.tx.model.node(object)?;
        let params = node
            .methods
            .get(method)
            .map(|m| m.parameters.as_slice())
            .or_else(|| node.native.get(method).map(|m| m.parameters.as_slice()))
            .or_else(|| node.signal(method))?;
        params.get(index)?.ty.as_deref()
    }
    pub fn resources(&self) -> &BTreeMap<String, ir::ScriptResource> {
        &self.tx.model.javascript.resources
    }
    pub fn arguments(&self) -> &BTreeMap<String, Value> {
        self.locals
    }

    /// Formal parameter names in declaration order, suitable for a VM's
    /// arguments object. Names visible through a retained closure belong to
    /// the VM's lexical environment.
    pub fn parameter_names(&self) -> &[String] {
        if self.script.is_some_and(|s| {
            matches!(
                s.form,
                ir::ScriptForm::FunctionBody | ir::ScriptForm::HandlerBody
            )
        }) {
            &self.tx.guest_parameters
        } else {
            &[]
        }
    }
    /// Schedule a function on the owner thread after the current transaction commits.
    pub fn bound_method(&self, receiver: ObjectId, name: &str) -> Result<MethodRef, Error> {
        self.tx.model.method(receiver, name)
    }

    pub fn defer_method(&mut self, method: MethodRef, arguments: Vec<Value>) -> Result<(), Error> {
        self.tx
            .model
            .validate_handles(&Value::Method(method.clone()))?;
        for argument in &arguments {
            self.tx.model.validate_handles(argument)?;
        }
        self.tx
            .deferred
            .push(deferred::Task::Method(method, arguments));
        Ok(())
    }

    pub fn call_later(
        &mut self,
        owner: Option<ObjectId>,
        function: ExternalValue,
        arguments: Vec<Value>,
    ) -> Result<(), Error> {
        let owner = owner.unwrap_or(self.owner);
        if self.tx.model.node(owner).is_none() || function.instance() != self.instance() {
            return Err(Error::new("foreign or destroyed deferred callback"));
        }
        for value in &arguments {
            self.tx.model.validate_handles(value)?;
        }
        self.tx.deferred.push(deferred::Task::Call {
            owner,
            function,
            arguments,
        });
        Ok(())
    }

    pub(crate) fn list_view_call(&mut self, name: &str, args: &[Value]) -> Result<Value, Error> {
        self.tx.list_view_call(self.owner, name, args, self.reads)
    }

    pub(crate) fn repeated_item(&mut self, index: i32) -> Result<Value, Error> {
        self.tx.repeated_item(self.owner, index, self.reads)
    }

    pub fn consume_work(&mut self) -> Result<(), Error> {
        self.tx.budget()
    }

    /// Visible names only; enumeration never reads a property or activates an
    /// otherwise-unused reactive dependency.
    pub fn names(&self) -> Vec<String> {
        let mut names = BTreeSet::from(["parent".to_owned()]);
        names.extend(self.locals.keys().cloned());
        self.tx.model.lexical_names(self.owner, &mut names);
        self.tx.model.delegate_context_names(self.owner, &mut names);
        if let Some(ids) = self.ids {
            names.extend(ids.keys().filter(|n| !private(n)).cloned());
        }
        for object in [self.owner, self.root, self.tx.model.root()] {
            names.extend(self.members(object));
        }
        if let Some(node) = self.tx.model.node(self.owner) {
            names.extend(node.type_scope.names().map(str::to_owned));
        }
        names.into_iter().collect()
    }

    pub fn members(&self, object: ObjectId) -> Vec<String> {
        let Some(node) = self.tx.model.node(object) else {
            return Vec::new();
        };
        let mut names: BTreeSet<String> =
            self.tx.model.property_names(object).into_iter().collect();
        names.extend(
            node.properties
                .keys()
                .filter(|n| !private(n))
                .map(|name| format!("{name}Changed")),
        );
        names.extend(
            node.methods
                .keys()
                .chain(node.native.keys())
                .chain(node.signals.keys())
                .cloned(),
        );
        if self.is_worker_script(object) {
            names.insert("sendMessage".into());
        }
        if self.tx.model.is_animation(object) {
            names.extend(animations::METHODS.map(str::to_owned));
        }
        if self.tx.model.timers.is_timer(object) {
            names.extend(["start", "restart", "stop"].map(str::to_owned));
        }
        if self.tx.model.dynamic.contexts.contains_key(&object) {
            names.insert("destroy".into());
        }
        if self.is_loader(object) {
            names.insert("setSource".to_owned());
        }
        if self.tx.model.is_component(object) {
            names.extend(["createObject", "incubateObject", "errorString"].map(str::to_owned));
        }
        if self.tx.model.models.contains_key(&object) {
            names.extend(
                [
                    "append",
                    "clear",
                    "get",
                    "insert",
                    "move",
                    "remove",
                    "set",
                    "setProperty",
                    "sync",
                ]
                .map(str::to_owned),
            );
        }
        names.retain(|name| !private(name));
        names.into_iter().collect()
    }

    fn id(&self, name: &str) -> Option<ObjectId> {
        if private(name) {
            return None;
        }
        if let Some(mapped) = self.ids.and_then(|ids| ids.get(name)) {
            return self.tx.model.lexical_id(self.owner, mapped);
        }
        self.tx.model.lexical_id(self.owner, name)
    }

    pub fn resolve(&mut self, name: &str) -> Result<Option<Value>, Error> {
        if private(name) {
            return Ok(None);
        }
        self.tx.budget()?;
        if let Some(v) = self.locals.get(name) {
            return Ok(Some(v.clone()));
        }
        match name {
            "this" => return Ok(Some(Value::Object(self.owner))),
            "parent" => {
                return Ok(Some(
                    self.tx
                        .model
                        .parent(self.owner)
                        .map(Value::Object)
                        .unwrap_or(Value::Null),
                ))
            }
            _ => (),
        }
        if let Some(id) = self.id(name) {
            return Ok(Some(Value::Object(id)));
        }
        if let Ok(key) = self.tx.scoped_key(self.owner, name) {
            return self.read_key(key).map(Some);
        }
        if let Some(v) = self.tx.model.nodes[self.owner.index]
            .type_scope
            .member(name)
        {
            return Ok(Some(v));
        }
        if let Ok(key) = self.tx.lookup_property(self.owner, self.root, name) {
            return self.read_key(key).map(Some);
        }
        Ok(None)
    }

    fn read_key(&mut self, key: SlotId) -> Result<Value, Error> {
        let result = self.tx.read(key, self.reads);
        self.tx.host_result(result)
    }

    pub fn is_signal(&self, object: ObjectId, name: &str) -> bool {
        !private(name)
            && self
                .tx
                .model
                .node(object)
                .is_some_and(|node| node.signal(name).is_some())
    }

    /// Connect a validated guest function, optionally with its explicit this object.
    pub fn connect_signal(
        &mut self,
        sender: ObjectId,
        signal: &str,
        receiver: Value,
        function: ExternalValue,
    ) -> Result<(), Error> {
        self.tx
            .change_signal_connection(sender, signal, receiver, function, true)
    }

    pub fn disconnect_signal(
        &mut self,
        sender: ObjectId,
        signal: &str,
        receiver: Value,
        function: ExternalValue,
    ) -> Result<(), Error> {
        self.tx
            .change_signal_connection(sender, signal, receiver, function, false)
    }

    pub fn is_method(&self, object: ObjectId, name: &str) -> bool {
        self.tx.model.script_method(object, name)
    }

    pub fn method_owner(&self, name: &str) -> Option<ObjectId> {
        if self.locals.contains_key(name) || self.id(name).is_some() {
            return None;
        }
        [self.owner, self.root, self.tx.model.root()]
            .into_iter()
            .find(|&object| self.is_method(object, name))
    }

    pub fn assign(&mut self, name: &str, value: Value) -> Result<(), Error> {
        self.tx.model.validate_handles(&value)?;
        if self.locals.contains_key(name) {
            self.locals.insert(name.to_owned(), value);
            return Ok(());
        }
        if private(name) || self.id(name).is_some() || matches!(name, "parent" | "this") {
            return Err(Error::new(format!("cannot assign to '{name}'")));
        }
        let key = self.tx.lookup_property(self.owner, self.root, name)?;
        self.write_key(key, value)
    }

    /// Install a validated guest function as the binding of a writable object
    /// property. An absent receiver selects the property in the lexical scope.
    pub fn install_binding(
        &mut self,
        object: Option<ObjectId>,
        name: &str,
        callback: ExternalValue,
    ) -> Result<(), Error> {
        if private(name) {
            return Err(Error::new("private QML member"));
        }
        let key = if let Some(object) = object {
            self.tx.scoped_key(object, name)?
        } else {
            if self.locals.contains_key(name)
                || self.id(name).is_some()
                || matches!(name, "this" | "parent")
            {
                return Err(Error::new(format!("cannot install a binding on '{name}'")));
            }
            self.tx.lookup_property(self.owner, self.root, name)?
        };
        self.tx.budget()?;
        let result = self.tx.install_guest_binding(key, callback);
        self.tx.host_result(result)
    }

    fn write_key(&mut self, key: SlotId, value: Value) -> Result<(), Error> {
        self.tx.budget()?;
        let result = self.tx.write(key, value).and_then(|()| self.tx.settle());
        self.tx.host_result(result)
    }

    pub fn get(&mut self, object: ObjectId, name: &str) -> Result<Value, Error> {
        self.read_member(&Value::Object(object), name)
    }
    pub fn set(&mut self, object: ObjectId, name: &str, value: Value) -> Result<(), Error> {
        self.write_member(&Value::Object(object), name, value)
    }

    pub fn call(&mut self, object: ObjectId, name: &str, args: Vec<Value>) -> Result<Value, Error> {
        if private(name) {
            return Err(Error::new("private QML member"));
        }
        let result = self
            .tx
            .invoke(object, name, args, self.reads)
            .and_then(|v| {
                // A binding may call a pure QML method during graph resolution.
                if !self.tx.in_binding() {
                    self.tx.settle()?;
                }
                Ok(v)
            });
        self.tx.host_result(result)
    }

    pub fn read_member(&mut self, base: &Value, name: &str) -> Result<Value, Error> {
        if private(name) {
            return Err(Error::new("private QML member"));
        }
        self.tx.budget()?;
        self.tx.model.validate_handles(base)?;
        if let Value::Object(object) = base {
            if name != "parent" && self.tx.scoped_key(*object, name).is_err() {
                return Ok(Value::Undefined);
            }
        }
        let result = self.tx.member(base.clone(), name, self.reads);
        self.tx.host_result(result)
    }
    pub fn write_member(&mut self, base: &Value, name: &str, value: Value) -> Result<(), Error> {
        if private(name) {
            return Err(Error::new("private QML member"));
        }
        self.tx.model.validate_handles(base)?;
        self.tx.model.validate_handles(&value)?;
        let result = match base {
            Value::Object(object) => {
                return self.write_key(self.tx.scoped_key(*object, name)?, value)
            }
            Value::ModelRow(row) => self.tx.write_role(*row, name, value),
            Value::DelegateRow(owner) => self.tx.write_delegate_role(*owner, name, value),
            Value::AnchorGroup(id) => {
                return self.write_key(self.tx.scoped_key(*id, &format!("anchors.{name}"))?, value)
            }
            Value::Record(record) => self.tx.set_record_field(record, name, value),
            Value::External(external) => {
                self.tx
                    .with_external(self.owner, external, self.reads, |engine, ctx| {
                        engine.set_external(ctx, external, name, value)
                    })
            }
            _ => Err(Error::new("value does not have writable members")),
        };
        self.tx.host_result(result)
    }

    pub fn record(&mut self, fields: BTreeMap<String, Value>) -> Result<Value, Error> {
        for v in fields.values() {
            self.tx.model.validate_handles(v)?;
        }
        Ok(self.tx.new_record(fields))
    }
    pub fn array(&mut self, items: Vec<Value>) -> Result<Value, Error> {
        for v in &items {
            self.tx.model.validate_handles(v)?;
        }
        Ok(self.tx.new_array(items))
    }
    pub fn record_keys(&self, record: &Record) -> Result<Vec<String>, Error> {
        Ok(self
            .tx
            .model
            .record_entries(record)?
            .keys()
            .cloned()
            .collect())
    }
    pub fn external<T: Any>(&self, id: u64, payload: Rc<T>) -> Value {
        Value::External(ExternalValue {
            instance: self.instance(),
            id,
            payload,
        })
    }
}

fn private(name: &str) -> bool {
    name.starts_with("$qml:")
}

impl Instance {
    pub fn script_instance(&self) -> ScriptInstance {
        ScriptInstance(self.identity)
    }
    /// Evaluate guest code from a Rust entry point under one QML transaction.
    pub fn evaluate_script(
        &mut self,
        owner: ObjectId,
        script: &ir::JavaScript,
    ) -> Result<Value, Error> {
        if self.node(owner).is_none() {
            return Err(Error::new("foreign or unknown script scope"));
        }
        self.transaction(|tx| tx.guest(owner, script, &mut Locals::new(), &mut BTreeSet::new()))
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        if let Some(engine) = &self.javascript.engine {
            engine.release_instance(self.script_instance());
        }
    }
}

impl Instance {
    pub(crate) fn script_method(&self, object: ObjectId, name: &str) -> bool {
        if private(name) {
            return false;
        }
        self.node(object).is_some_and(|node| {
            (self.dynamic.loaders.contains_key(&object) && name == "setSource")
                || (self.dynamic.contexts.contains_key(&object) && name == "destroy")
                || (self.is_component(object)
                    && matches!(name, "createObject" | "incubateObject" | "errorString"))
                || node.methods.contains_key(name)
                || node.native.contains_key(name)
                || node.signal(name).is_some()
                || self.workers.contains(object) && name == "sendMessage"
                || self.models.contains_key(&object) && list_model::is_method(name)
                || self.is_animation(object) && animations::METHODS.contains(&name)
                || self.timers.is_timer(object) && matches!(name, "start" | "stop" | "restart")
        })
    }
}

impl Transaction<'_> {
    pub(crate) fn host_result<T>(&mut self, result: Result<T, Error>) -> Result<T, Error> {
        // A guest may catch an exception from a host callback whose write or
        // downstream binding already touched state. Never commit that journal.
        if let Err(error) = &result {
            self.failed_host_call.get_or_insert_with(|| error.clone());
        }
        result
    }

    fn script_engine(&mut self) -> Result<Rc<dyn ScriptEngine>, Error> {
        let engine = self
            .model
            .javascript
            .engine
            .clone()
            .ok_or_else(|| Error::new("this component requires a JavaScript engine"))?;
        if !self.used_engine {
            engine.begin_transaction(self.model.script_instance())?;
            self.used_engine = true;
        }
        Ok(engine)
    }

    pub(crate) fn guest(
        &mut self,
        owner: ObjectId,
        script: &ir::JavaScript,
        locals: &mut Locals,
        reads: &mut BTreeSet<SlotId>,
    ) -> Result<Value, Error> {
        let engine = self.script_engine()?;
        let root = script
            .root
            .as_deref()
            .map(|id| self.scope_id(owner, id))
            .transpose()?
            .unwrap_or_else(|| self.model.context_root(owner));
        self.budget()?;
        let value = self
            .with_stack(4, |tx| {
                let mut context = ScriptContext {
                    tx,
                    owner,
                    root,
                    script: Some(script),
                    ids: Some(&script.ids),
                    locals,
                    reads,
                };
                engine.evaluate(&mut context, script)
            })
            .map_err(|error| {
                Error::new(format!(
                    "{}[{}..{}]: {error}",
                    script.origin, script.span.start, script.span.end
                ))
            })?;
        self.model.validate_handles(&value)?;
        Ok(value)
    }

    pub(crate) fn with_external<T>(
        &mut self,
        owner: ObjectId,
        external: &ExternalValue,
        reads: &mut BTreeSet<SlotId>,
        op: impl FnOnce(&dyn ScriptEngine, &mut ScriptContext<'_, '_>) -> Result<T, Error>,
    ) -> Result<T, Error> {
        if external.instance != self.model.script_instance() {
            return Err(Error::new("foreign JavaScript value"));
        }
        let engine = self.script_engine()?;
        self.with_receiver_context(owner, reads, |context| op(engine.as_ref(), context))
    }

    pub(crate) fn with_receiver_context<T>(
        &mut self,
        owner: ObjectId,
        reads: &mut BTreeSet<SlotId>,
        op: impl FnOnce(&mut ScriptContext<'_, '_>) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let root = self.model.context_root(owner);
        self.with_stack(4, |tx| {
            op(&mut ScriptContext {
                tx,
                owner,
                root,
                script: None,
                ids: None,
                locals: &mut Locals::new(),
                reads,
            })
        })
    }
}
