//! Mount a compiled QML component into FerroUI's retained control tree.
//!
//! Keep the returned [`QmlView`] alive for as long as the component is interactive.
//! The view owns a single instance and native subscriptions. Subscribers hold weak
//! handles back to the view, and dropping its last owner disconnects them.
mod anchors;
mod builtins;
mod clock;
mod composition;
mod reflected;

use ferroui_base::{reactive::IDisposable, Ref};
use ferroui_controls::{Control, Controls};
use ferroui_qml::{
    ComponentSource, Error, Instance, ObjectId, TypeDefinition, TypeRegistry, Value,
};
use std::{
    cell::RefCell,
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

type Factory = Box<dyn Fn() -> Box<dyn NativeObject>>;

/// A type registry with optional native factories. Pure model types need no factory.
pub struct ControlRegistry {
    types: TypeRegistry,
    factories: BTreeMap<String, BTreeMap<String, Factory>>,
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
            .insert(name.into(), Box::new(factory));
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
                    if let Some(node) = view.0.nodes.get(&self.object) {
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
    instance: RefCell<Instance>,
    nodes: BTreeMap<ObjectId, Box<dyn NativeObject>>,
    root: Ref<Control>,
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    on_error: Box<dyn Fn(Error)>,
}

impl Drop for Inner {
    fn drop(&mut self) {
        if let Some(clock) = self.clock.get() {
            clock.stop();
        }
        for subscription in self.subscriptions.get_mut().drain(..) {
            subscription.dispose();
        }
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
        let mut instance = Instance::new(document, &registry.types, initial)?;
        let mut pending = vec![instance.root()];
        let mut nodes = BTreeMap::new();
        while let Some(object) = pending.pop() {
            pending.extend_from_slice(instance.children(object));
            let factory = instance
                .module(object)
                .and_then(|module| registry.factories.get(module))
                .and_then(|types| types.get(instance.type_name(object)?));
            if let Some(factory) = factory {
                let node = factory();
                for name in instance.property_names(object) {
                    let value = instance.get(object, &name)?;
                    node.validate(&name, &value)?;
                    node.apply_initial(&name, &value, instance.is_assigned(object, &name))?;
                }
                nodes.insert(object, node);
            }
        }
        let root = nodes
            .get(&instance.root())
            .ok_or_else(|| Error::new("component root has no native visual factory"))?
            .control();
        composition::reconcile(&instance, &nodes)?;
        instance.drain_changes();
        let view = Self(Rc::new(Inner {
            clock: std::cell::OnceCell::new(),
            instance: RefCell::new(instance),
            nodes,
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
                let mut layout_changed = false;
                for change in changes {
                    if let Some(node) = inner.nodes.get(&change.object) {
                        node.validate(&change.property, &change.value)?;
                        layout_changed |= node.children_property()
                            == Some(change.property.as_str())
                            || (node.uses_item_anchors()
                                && change.property.starts_with("anchors."));
                    }
                }
                if layout_changed {
                    composition::plan(instance, &inner.nodes)?;
                }
                Ok(())
            });
        for (&object, node) in &view.0.nodes {
            let endpoint = NativeEndpoint {
                inner: Rc::downgrade(&view.0),
                object,
            };
            view.0
                .subscriptions
                .borrow_mut()
                .extend(node.connect(endpoint));
        }
        view.install_clock();
        Ok(view)
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
        self.0.nodes.get(&id).map(|node| node.control())
    }
    pub fn get(&self, object: ObjectId, property: &str) -> Result<Value, Error> {
        self.0.instance.borrow().get(object, property)
    }

    pub fn set(&self, object: ObjectId, property: &str, value: Value) -> Result<(), Error> {
        self.0.instance.borrow_mut().set(object, property, value)?;
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
        let changes = self.0.instance.borrow_mut().drain_changes();
        let mut layout_changed = false;
        for change in changes {
            if let Some(node) = self.0.nodes.get(&change.object) {
                layout_changed |= node.children_property() == Some(change.property.as_str())
                    || (node.uses_item_anchors() && change.property.starts_with("anchors."));
                // A native subscriber can synchronously update a later slot in this batch.
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
        if layout_changed {
            let plan = composition::plan(&self.0.instance.borrow(), &self.0.nodes)?;
            composition::apply(plan);
        }
        self.refresh_clock();
        Ok(())
    }
}

#[cfg(test)]
mod tests;
