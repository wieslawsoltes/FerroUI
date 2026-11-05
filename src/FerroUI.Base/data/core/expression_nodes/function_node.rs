//! Not a port of an upstream file: the function step of WinUI's compiled bindings
//! (`{x:Bind Format(Name, 'text')}`, docs/porting/x-bind.md, step X2) as a node of a binding
//! path. The behaviour is the one of the code WinUI's XAML compiler generates for a function
//! (`BindPathStepCodeGenerators.cs` of microsoft-ui-xaml): the function is called with the
//! values of its arguments once they can all be read, and again when one of them changes.

use super::{append_member, ExpressionNode, ISettableNode, NodeState};
use crate::data::core::plugins::AsNotifyPropertyChanged;
use crate::data::core::{BindingExpression, BindingExpressionOptions, ValueType, WeakValue};
use crate::data::{
    BindingError, BindingErrorType, BindingMode, BindingNotification, CompiledBindingPath, PathBindBack,
    PathFunctionArgument, PathFunctionInvoke,
};
use crate::reactive::{AnonymousObserver, IDisposable, IObservable, IObserver};
use crate::{BoxedValue, UnsetValueType};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// The value of an argument of the function.
#[derive(Clone)]
enum ArgumentValue {
    /// The path has not produced a value yet.
    Pending,
    Value(Option<BoxedValue>),
    /// The path could not be read.
    Failed(String),
}

/// A function call: the owner is the source of the node (the value of the path before it, or
/// the root source for a function at the start of the path); the arguments are constants or
/// paths read from the root source of the expression, each through a binding expression of
/// its own that the node observes.
///
/// The node reads its arguments when its source is set. Unless the binding is `OneTime`, it
/// keeps observing the argument paths and calls the function again when one of them
/// changes, and when the owner reports a change of the property named as the function
/// (WinUI's generated code updates a function on a `PropertyChanged` with its name). An
/// argument that cannot be read is an error of the binding, and the function is not called.
///
/// With a `BindBack`, the node is the settable leaf of a `TwoWay` binding: writing the value
/// of the target calls the `BindBack` method instead of setting a property.
pub struct FunctionNode {
    this: Weak<FunctionNode>,
    state: NodeState,
    name: Box<str>,
    arguments: Rc<[PathFunctionArgument]>,
    invoke: PathFunctionInvoke,
    notifier: Option<AsNotifyPropertyChanged>,
    bind_back: Option<Rc<PathBindBack>>,
    values: RefCell<Vec<ArgumentValue>>,
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    /// The owner the change handler is registered with, and the token of the handler.
    owner_handler: RefCell<Option<(WeakValue, u64)>>,
    /// Set while the arguments are read: their first values do not call the function.
    reading: Cell<bool>,
}

impl FunctionNode {
    pub fn new(
        name: &str,
        arguments: Rc<[PathFunctionArgument]>,
        invoke: PathFunctionInvoke,
        notifier: Option<AsNotifyPropertyChanged>,
        bind_back: Option<Rc<PathBindBack>>,
    ) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            state: NodeState::new(),
            name: name.into(),
            arguments,
            invoke,
            notifier,
            bind_back,
            values: RefCell::new(Vec::new()),
            subscriptions: RefCell::new(Vec::new()),
            owner_handler: RefCell::new(None),
            reading: Cell::new(false),
        })
    }

    /// The name of the function.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The root source the argument paths are read from: the value of the first node of the
    /// expression (the root source of `{x:Bind}`), or the source of this node when it is the
    /// first.
    fn root_source(&self) -> Option<BoxedValue> {
        let owner = self.state.owner();
        let first = owner.as_ref().and_then(|owner| owner.nodes().first().cloned());
        let value = match first {
            Some(first) if self.state.index() > 0 => first.state().value(),
            _ => self.state.source(),
        };
        value.filter(|value| !value.is::<UnsetValueType>())
    }

    /// Whether the binding follows changes: every mode but `OneTime`.
    fn tracks_changes(&self) -> bool {
        !matches!(self.state.owner().map(|owner| owner.mode()), Some(BindingMode::OneTime))
    }

    fn read_arguments(&self, root: Option<&BoxedValue>, track: bool) {
        let values: Vec<ArgumentValue> = self
            .arguments
            .iter()
            .map(|argument| match argument {
                PathFunctionArgument::Value(value) => ArgumentValue::Value(value.clone()),
                PathFunctionArgument::Path(_) => ArgumentValue::Pending,
            })
            .collect();
        *self.values.borrow_mut() = values;

        self.reading.set(true);
        let mut subscriptions = Vec::new();
        for (index, argument) in self.arguments.iter().enumerate() {
            let PathFunctionArgument::Path(path) = argument else {
                continue;
            };
            let Some(root) = root else {
                self.set_argument(index, ArgumentValue::Failed("the root source is null".to_string()));
                continue;
            };
            let sink = path_expression(root, path).to_observable(None);
            let observer: Rc<dyn IObserver<Option<BoxedValue>>> =
                Rc::new(ArgumentObserver { node: self.this.clone(), index });
            // The first value arrives while subscribing.
            let subscription = sink.subscribe(observer);
            if track {
                subscriptions.push(subscription);
            } else {
                subscription.dispose();
            }
        }
        self.reading.set(false);
        *self.subscriptions.borrow_mut() = subscriptions;
    }

    fn set_argument(&self, index: usize, value: ArgumentValue) {
        if let Some(slot) = self.values.borrow_mut().get_mut(index) {
            *slot = value;
        }
    }

    fn on_argument_changed(&self, index: usize, value: Option<BoxedValue>) {
        let value = match read_value(value) {
            Ok(value) => ArgumentValue::Value(value),
            Err(message) => ArgumentValue::Failed(message),
        };
        self.set_argument(index, value);
        if !self.reading.get() {
            self.evaluate();
        }
    }

    /// Calls the function on the owner with the current values of the arguments.
    fn evaluate(&self) {
        let Some(owner) = self.state.source() else {
            return;
        };
        let values = self.values.borrow().clone();
        let mut arguments = Vec::with_capacity(values.len());
        for (index, value) in values.into_iter().enumerate() {
            match value {
                ArgumentValue::Value(value) => arguments.push(value),
                ArgumentValue::Pending => {
                    let message = format!("The argument {} of '{}' has no value.", index + 1, self.name);
                    return self.state.set_error(&message);
                }
                ArgumentValue::Failed(reason) => {
                    let message = format!("The argument {} of '{}' could not be read: {reason}", index + 1, self.name);
                    return self.state.set_error(&message);
                }
            }
        }
        match (self.invoke)(&owner, &arguments) {
            Ok(value) => self.state.set_value(value, None),
            Err(error) => self.state.set_error(&error.to_string()),
        }
    }

    /// Calls the function again when the owner reports a change of the property named as the
    /// function, or of all its properties.
    fn watch_owner(&self, owner: &BoxedValue) {
        let Some(notifier) = self.notifier else {
            return;
        };
        let Some(inpc) = notifier(&**owner) else {
            return;
        };
        let weak = self.this.clone();
        let name = self.name.clone();
        let token = inpc.property_changed().add(Rc::new(move |changed: &str| {
            if changed.is_empty() || changed == &*name {
                if let Some(this) = weak.upgrade() {
                    this.evaluate();
                }
            }
        }));
        *self.owner_handler.borrow_mut() = Some((WeakValue::new(owner), token));
    }

    /// Releases the argument expressions and the change handler of the owner.
    fn release(&self) {
        let subscriptions = std::mem::take(&mut *self.subscriptions.borrow_mut());
        for subscription in subscriptions {
            subscription.dispose();
        }
        let handler = self.owner_handler.borrow_mut().take();
        if let (Some((owner, token)), Some(notifier)) = (handler, self.notifier) {
            if let Some(owner) = owner.upgrade() {
                if let Some(inpc) = notifier(&*owner) {
                    inpc.property_changed().remove(token);
                }
            }
        }
    }
}

impl ExpressionNode for FunctionNode {
    fn state(&self) -> &NodeState {
        &self.state
    }

    fn build_string(&self, builder: &mut String) {
        append_member(builder, &format!("{}()", self.name));
    }

    fn on_source_changed(&self, source: Option<&BoxedValue>, _data_validation_error: Option<&BindingError>) {
        self.release();
        if !self.state.validate_non_null_source(source) {
            return;
        }
        let track = self.tracks_changes();
        let root = self.root_source();
        self.read_arguments(root.as_ref(), track);
        if let (true, Some(source)) = (track, source) {
            self.watch_owner(source);
        }
        self.evaluate();
    }

    fn unsubscribe(&self, _old_source: &BoxedValue) {
        self.release();
    }

    fn as_settable(&self) -> Option<&dyn ISettableNode> {
        Some(self)
    }
}

impl ISettableNode for FunctionNode {
    fn value_type(&self) -> Option<ValueType> {
        self.bind_back.as_ref().map(|bind_back| bind_back.value_type)
    }

    fn write_value_to_source(
        &self,
        value: Option<&BoxedValue>,
        _nodes: &[Rc<dyn ExpressionNode>],
    ) -> Result<bool, BindingError> {
        let Some(bind_back) = &self.bind_back else {
            return Ok(false);
        };
        let root = self
            .root_source()
            .ok_or_else(|| BindingError::message(format!("The BindBack of '{}' has no root source.", self.name)))?;
        let owner = match &bind_back.owner {
            None => root,
            Some(path) => read_once(&root, path)
                .map_err(|reason| {
                    BindingError::message(format!("The owner of the BindBack of '{}' could not be read: {reason}", self.name))
                })?
                .ok_or_else(|| BindingError::message(format!("The owner of the BindBack of '{}' is null.", self.name)))?,
        };
        (bind_back.invoke)(&owner, value)?;
        Ok(true)
    }
}

impl Drop for FunctionNode {
    fn drop(&mut self) {
        self.release();
    }
}

/// Observes the value of one argument path.
struct ArgumentObserver {
    node: Weak<FunctionNode>,
    index: usize,
}

impl IObserver<Option<BoxedValue>> for ArgumentObserver {
    fn on_next(&self, value: Option<BoxedValue>) {
        if let Some(node) = self.node.upgrade() {
            node.on_argument_changed(self.index, value);
        }
    }
}

/// The expression that reads `path` from `root`, unattached to a target.
fn path_expression(root: &BoxedValue, path: &CompiledBindingPath) -> Rc<BindingExpression> {
    let mut nodes = Vec::new();
    path.build_expression(&mut nodes);
    BindingExpression::new(Some(root.clone()), nodes, BindingExpressionOptions::default())
}

/// The value an observed expression produced: its value, or why it has none.
fn read_value(value: Option<BoxedValue>) -> Result<Option<BoxedValue>, String> {
    if let Some(notification) = value.as_ref().and_then(|value| value.downcast_ref::<BindingNotification>()) {
        if notification.error_type() == BindingErrorType::Error {
            return Err(notification
                .error()
                .map(|error| error.to_string())
                .unwrap_or_else(|| "the path could not be read".to_string()));
        }
        return match notification.has_value() {
            true => read_value(notification.value()),
            false => Err("the path has no value".to_string()),
        };
    }
    if value.as_ref().is_some_and(|value| value.is::<UnsetValueType>()) {
        return Err("the path has no value".to_string());
    }
    Ok(value)
}

/// Reads `path` from `root` once.
fn read_once(root: &BoxedValue, path: &CompiledBindingPath) -> Result<Option<BoxedValue>, String> {
    let sink = path_expression(root, path).to_observable(None);
    let read: Rc<RefCell<Option<Option<BoxedValue>>>> = Rc::new(RefCell::new(None));
    let captured = read.clone();
    let observer: Rc<dyn IObserver<Option<BoxedValue>>> =
        Rc::new(AnonymousObserver::new(move |value: Option<BoxedValue>| {
            *captured.borrow_mut() = Some(value);
        }));
    sink.subscribe(observer).dispose();
    let value = read.borrow_mut().take();
    match value {
        Some(value) => read_value(value),
        None => Err("the path has no value".to_string()),
    }
}
