//! Mount a compiled QML component into FerroUI's retained control tree.
//!
//! Keep the returned [`QmlView`] alive for as long as the component is interactive.
//! The view owns a single instance and native subscriptions. Subscribers hold weak
//! handles back to the view, and dropping its last owner disconnects them.
mod anchors;
mod builtins;
mod clock;
mod composition;
mod positioner;
mod reflected;
mod tree;

use ferroui_base::{reactive::IDisposable, Ref};
use ferroui_controls::{Control, Controls};
use ferroui_qml::{
    ComponentSource, Error, Instance, ObjectId, ScriptEngine, TypeDefinition, TypeRegistry, Value,
};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::{Rc, Weak},
};

/// The visual counterpart of a registered QML object. Application properties are
/// also supplied to `apply`; ignore names that do not map to native state.
pub trait NativeObject {
    fn control(&self) -> Ref<Control>;
    /// Check additional constraints (e.g. native colors) without mutating a control.
    /// Run before a settled transaction commits. After validation, applying the
    /// same value must succeed.
    fn validate(&self, _property: &str, _value: &Value) -> Result<(), Error> {
        Ok(())
    }
    fn apply(&self, property: &str, value: &Value) -> Result<(), Error>;

    /// Native types with meaningful default or inherited values may skip unassigned
    /// properties, preserving FerroUI style precedence at mount time.
    fn apply_initial(&self, property: &str, value: &Value, _assigned: bool) -> Result<(), Error> {
        self.apply(property, value)
    }

    fn uses_item_anchors(&self) -> bool {
        false
    }
    fn anchor_host(&self) -> Option<Ref<ferroui_controls::Canvas>> {
        None
    }

    fn children_property(&self) -> Option<&str> {
        self.content().map(|(name, _)| name)
    }

    /// A list property whose objects become children of a FerroUI panel.
    fn content(&self) -> Option<(&str, Controls)> {
        None
    }

    /// Connect native input after the initial tree and all properties are applied.
    fn connect(&self, _endpoint: NativeEndpoint) -> Vec<Rc<dyn IDisposable>> {
        Vec::new()
    }
}

type Factory = Rc<dyn Fn() -> Box<dyn NativeObject>>;

/// A type registry with optional native factories. Pure model types need no factory.
pub struct ControlRegistry {
    types: TypeRegistry,
    factories: tree::Factories,
}

impl Default for ControlRegistry {
    fn default() -> Self {
        Self {
            types: TypeRegistry::new(),
            factories: BTreeMap::new(),
        }
    }
}

impl ControlRegistry {
    pub fn standard() -> Result<Self, Error> {
        let mut registry = Self::default();
        builtins::register(&mut registry)?;
        Ok(registry)
    }

    pub fn types_mut(&mut self) -> &mut TypeRegistry {
        &mut self.types
    }

    pub fn register(
        &mut self,
        module: &str,
        name: &str,
        definition: TypeDefinition,
        factory: impl Fn() -> Box<dyn NativeObject> + 'static,
    ) -> Result<(), Error> {
        self.types.register(module, name, definition)?;
        self.factories
            .entry(module.into())
            .or_default()
            .insert(name.into(), Rc::new(factory));
        Ok(())
    }
}

/// Target for native input. The endpoint neither retains the control tree nor
/// requires a host callback to borrow the component while setting a native value.
#[derive(Clone)]
pub struct NativeEndpoint {
    inner: Weak<Inner>,
    object: ObjectId,
}

impl NativeEndpoint {
    pub fn write(&self, name: &str, value: Value) {
        self.dispatch(|view| {
            // Applying a QML value to a native input also raises its native
            // change notification. An echo must preserve the original binding.
            let previous = view.get(self.object, name)?;
            let equal = previous == value
                || matches!((&previous, &value), (Value::Number(a), Value::Number(b)) if a.is_nan() && b.is_nan());
            if !equal {
                let outcome = view.0.instance.borrow_mut().set(self.object, name, value.clone());
                if let Err(error) = outcome {
                    // A failed handler or native validator rolls back the model.
                    // Restore the originating widget as well.
                    if let Some(node) = view.peer(self.object) {
                        node.apply(name, &previous)?;
                    }
                    return Err(error);
                }
                view.flush_from(Some((self.object, name, &value)))?;
            }
            Ok(())
        });
    }

    pub fn emit(&self, signal: &str, arguments: Vec<Value>) {
        self.dispatch(|view| view.emit(self.object, signal, arguments));
    }

    fn dispatch(&self, op: impl FnOnce(&QmlView) -> Result<(), Error>) {
        let Some(inner) = self.inner.upgrade() else {
            return;
        };
        let view = QmlView(inner);
        if let Err(error) = op(&view) {
            (view.0.on_error)(error);
        }
    }
}

struct Inner {
    clock: std::cell::OnceCell<clock::ViewClock>,
    manual_clock: Cell<bool>,
    deferred_posted: std::cell::Cell<bool>,
    instance: RefCell<Instance>,
    nodes: RefCell<tree::Peers>,
    factories: tree::Factories,
    revision: Cell<u64>,
    candidate: RefCell<Option<tree::Candidate>>,
    flushing: Cell<bool>,
    flush_again: Cell<bool>,
    root: Ref<Control>,
    subscriptions: RefCell<BTreeMap<ObjectId, tree::Subscriptions>>,
    on_error: Box<dyn Fn(Error)>,
}

impl Drop for Inner {
    fn drop(&mut self) {
        if let Some(clock) = self.clock.get() {
            clock.stop();
        }
        self.subscriptions.get_mut().clear();
    }
}

#[derive(Clone)]
pub struct QmlView(Rc<Inner>);

impl QmlView {
    pub fn new(
        document: &impl ComponentSource,
        registry: &ControlRegistry,
        initial: BTreeMap<String, Value>,
    ) -> Result<Self, Error> {
        Self::with_error_handler(document, registry, initial, |error| {
            eprintln!("QML event: {error}")
        })
    }

    pub fn with_error_handler(
        document: &impl ComponentSource,
        registry: &ControlRegistry,
        initial: BTreeMap<String, Value>,
        on_error: impl Fn(Error) + 'static,
    ) -> Result<Self, Error> {
        Self::mount(
            Instance::new(document, &registry.types, initial)?,
            registry,
            on_error,
        )
    }

    /// Mount a component that uses an external ECMAScript engine.
    pub fn with_script_engine(
        document: &impl ComponentSource,
        registry: &ControlRegistry,
        initial: BTreeMap<String, Value>,
        engine: Rc<dyn ScriptEngine>,
    ) -> Result<Self, Error> {
        Self::with_script_engine_and_error_handler(document, registry, initial, engine, |error| {
            eprintln!("QML event: {error}")
        })
    }

    pub fn with_script_engine_and_error_handler(
        document: &impl ComponentSource,
        registry: &ControlRegistry,
        initial: BTreeMap<String, Value>,
        engine: Rc<dyn ScriptEngine>,
        on_error: impl Fn(Error) + 'static,
    ) -> Result<Self, Error> {
        Self::mount(
            Instance::with_engine(document, &registry.types, initial, engine)?,
            registry,
            on_error,
        )
    }

    fn mount(
        mut instance: Instance,
        registry: &ControlRegistry,
        on_error: impl Fn(Error) + 'static,
    ) -> Result<Self, Error> {
        let nodes = tree::prepare(
            &instance,
            instance.objects(),
            &BTreeMap::new(),
            &registry.factories,
        )?;
        let revision = instance.structure_revision();
        let root = nodes
            .get(&instance.root())
            .ok_or_else(|| Error::new("component root has no native visual factory"))?
            .control();
        composition::reconcile(&instance, &nodes)?;
        instance.drain_changes();
        instance.drain_created();
        let view = Self(Rc::new(Inner {
            clock: std::cell::OnceCell::new(),
            manual_clock: Cell::new(false),
            deferred_posted: std::cell::Cell::new(false),
            instance: RefCell::new(instance),
            nodes: RefCell::new(nodes),
            factories: registry.factories.clone(),
            revision: Cell::new(revision),
            candidate: RefCell::default(),
            flushing: Cell::new(false),
            flush_again: Cell::new(false),
            root,
            subscriptions: RefCell::default(),
            on_error: Box::new(on_error),
        }));
        let weak = Rc::downgrade(&view.0);
        view.0
            .instance
            .borrow_mut()
            .set_validator(move |instance, changes| {
                let Some(inner) = weak.upgrade() else {
                    return Ok(());
                };
                inner.validate(instance, changes)
            });
        let peers = view.0.nodes.borrow().clone();
        for (object, node) in peers {
            view.connect_peer(object, &*node);
        }
        view.refresh_clock();
        view.post_deferred();
        Ok(view)
    }

    fn connect_peer(&self, object: ObjectId, peer: &dyn NativeObject) {
        let endpoint = NativeEndpoint {
            inner: Rc::downgrade(&self.0),
            object,
        };
        let subscriptions = peer.connect(endpoint);
        if !subscriptions.is_empty() {
            self.0
                .subscriptions
                .borrow_mut()
                .insert(object, tree::Subscriptions(subscriptions));
        }
    }

    fn peer(&self, object: ObjectId) -> Option<Rc<dyn NativeObject>> {
        self.0.nodes.borrow().get(&object).cloned()
    }

    pub fn root(&self) -> Ref<Control> {
        self.0.root.clone()
    }
    pub fn root_id(&self) -> ObjectId {
        self.0.instance.borrow().root()
    }
    pub fn id(&self, name: &str) -> Option<ObjectId> {
        self.0.instance.borrow().id(name)
    }
    pub fn control(&self, id: ObjectId) -> Option<Ref<Control>> {
        self.peer(id).map(|node| node.control())
    }
    pub fn get(&self, object: ObjectId, property: &str) -> Result<Value, Error> {
        self.0.instance.borrow().get(object, property)
    }

    pub fn set(&self, object: ObjectId, property: &str, value: Value) -> Result<(), Error> {
        self.0.instance.borrow_mut().set(object, property, value)?;
        self.flush()
    }

    /// Instantiate a QML Component and attach its native subtree after validation.
    pub fn create_object(
        &self,
        component: ObjectId,
        parent: Option<ObjectId>,
        initial: BTreeMap<String, Value>,
    ) -> Result<ObjectId, Error> {
        let object = self
            .0
            .instance
            .borrow_mut()
            .create_object(component, parent, initial)?;
        self.flush()?;
        Ok(object)
    }

    /// Destroy a dynamic object and disconnect input on its native descendants.
    pub fn destroy_object(&self, object: ObjectId) -> Result<(), Error> {
        self.0.instance.borrow_mut().destroy_object(object)?;
        self.flush()
    }

    pub fn emit(&self, object: ObjectId, signal: &str, arguments: Vec<Value>) -> Result<(), Error> {
        self.0
            .instance
            .borrow_mut()
            .emit(object, signal, arguments)?;
        self.flush()
    }

    pub fn call(
        &self,
        object: ObjectId,
        method: &str,
        arguments: Vec<Value>,
    ) -> Result<Value, Error> {
        let value = self
            .0
            .instance
            .borrow_mut()
            .call(object, method, arguments)?;
        self.flush()?;
        Ok(value)
    }

    fn flush(&self) -> Result<(), Error> {
        self.flush_from(None)
    }

    fn flush_from(&self, source: Option<(ObjectId, &str, &Value)>) -> Result<(), Error> {
        if self.0.flushing.replace(true) {
            self.0.flush_again.set(true);
            return Ok(());
        }
        struct FlushGuard<'a>(&'a Cell<bool>);
        impl Drop for FlushGuard<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        let _guard = FlushGuard(&self.0.flushing);
        let mut source = source;
        loop {
            self.0.flush_again.set(false);
            self.apply_changes(source.take())?;
            if !self.0.flush_again.get() {
                break;
            }
        }
        self.refresh_clock();
        self.post_deferred();
        Ok(())
    }

    fn apply_changes(&self, source: Option<(ObjectId, &str, &Value)>) -> Result<(), Error> {
        let changes = {
            let mut instance = self.0.instance.borrow_mut();
            instance.drain_created();
            instance.drain_changes()
        };
        let mut layout_changed = self.accept_topology();
        for change in changes {
            if let Some(node) = self.peer(change.object) {
                layout_changed |= tree::affects_layout(&*node, &change.property);
                // A native subscriber can synchronously update a later slot in this batch.
                if self.0.instance.borrow().type_name(change.object).is_none() {
                    continue;
                }
                let latest = self.get(change.object, &change.property)?;
                // Preserve the native source's priority (e.g. an inherited style)
                // unless a handler produced a different final value.
                if !source.is_some_and(|(id, name, value)| {
                    id == change.object && name == change.property && value == &latest
                }) {
                    node.apply(&change.property, &latest)?;
                }
            }
        }
        if layout_changed && self.0.revision.get() == self.0.instance.borrow().structure_revision()
        {
            let plan = composition::plan(&self.0.instance.borrow(), &self.0.nodes.borrow())?;
            composition::apply(plan);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
