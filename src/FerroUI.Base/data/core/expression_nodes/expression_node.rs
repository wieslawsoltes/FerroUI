use crate::data::core::plugins::IPropertyAccessor;
use crate::data::core::{BindingExpression, ValueType, ValueTypes, WeakValue};
use crate::data::{BindingError, BindingErrorType, BindingNotification};
use crate::{BoxedValue, FerroObject, FerroProperty, Ref};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// A source for an expression node: unset, or a (possibly null) value.
#[derive(Clone)]
pub enum NodeSource {
    Unset,
    Value(Option<BoxedValue>),
}

/// The state every expression node has: its place in the binding path, its
/// source and its current value.
pub struct NodeState {
    index: Cell<usize>,
    owner: RefCell<Option<Weak<BindingExpression>>>,
    source: RefCell<Option<WeakValue>>,
    value: RefCell<HeldValue>,
}

/// The value of a node, as the node holds it.
enum HeldValue {
    Value(Option<BoxedValue>),
    /// The value of a node that locates an element
    /// ([`NodeState::locating_an_element`]).
    Element(Option<WeakValue>),
}

impl Default for NodeState {
    fn default() -> Self {
        Self::new()
    }
}

impl NodeState {
    pub fn new() -> Self {
        Self {
            index: Cell::new(0),
            owner: RefCell::new(None),
            source: RefCell::new(None),
            value: RefCell::new(HeldValue::Value(Some(FerroProperty::unset_value()))),
        }
    }

    /// The state of a node whose value is an element it locates from the
    /// target of the binding: an element of a name scope, an ancestor, the
    /// templated parent.
    ///
    /// Such a node holds its value weakly (DEVIATIONS.md, Bindings). The
    /// element it finds is kept alive by the tree it was found in, and that
    /// tree most often owns the target of the binding, which owns the
    /// binding and its nodes: held strongly, as the managed original holds
    /// the value of every node, the element and the target keep each other
    /// alive. The value reads null once the element is gone.
    pub fn locating_an_element() -> Self {
        Self { value: RefCell::new(HeldValue::Element(Some(WeakValue::new(&FerroProperty::unset_value())))), ..Self::new() }
    }

    /// The index of the node in the binding path.
    pub fn index(&self) -> usize {
        self.index.get()
    }

    /// The owning binding expression.
    pub fn owner(&self) -> Option<Rc<BindingExpression>> {
        self.owner.borrow().as_ref().and_then(Weak::upgrade)
    }

    /// The source object from which the node reads its value.
    pub fn source(&self) -> Option<BoxedValue> {
        let source = self.source.borrow().clone();
        source.and_then(|s| s.upgrade())
    }

    /// The source viewed as an object of the class hierarchy.
    pub fn source_object(&self) -> Option<Ref<FerroObject>> {
        self.source().and_then(|s| ValueTypes::as_object(&*s))
    }

    /// The current value of the node. May be the unset marker.
    pub fn value(&self) -> Option<BoxedValue> {
        match &*self.value.borrow() {
            HeldValue::Value(value) => value.clone(),
            HeldValue::Element(value) => value.as_ref().and_then(WeakValue::upgrade),
        }
    }

    pub(crate) fn set_raw_value(&self, value: Option<BoxedValue>) {
        let mut held = self.value.borrow_mut();
        let old = match &*held {
            HeldValue::Value(_) => std::mem::replace(&mut *held, HeldValue::Value(value)),
            HeldValue::Element(_) => {
                std::mem::replace(&mut *held, HeldValue::Element(value.as_ref().map(WeakValue::new)))
            }
        };
        // The old value is dropped after the cell is released: dropping it
        // may run code that reads the node.
        drop(held);
        drop(old);
    }

    /// Sets the current value to the unset marker, notifying the owner.
    pub fn clear_value(&self) {
        self.set_value(Some(FerroProperty::unset_value()), None);
    }

    /// Called by a node which has a null-conditional operator applied to it
    /// when its source is null: the value of this and all subsequent nodes
    /// becomes null and the binding produces null rather than an error.
    pub fn short_circuit_null(&self) {
        self.set_raw_value(None);
        if let Some(owner) = self.owner() {
            owner.on_node_null_short_circuit(self.index());
        }
    }

    /// Notifies the owner of a data validation error.
    pub fn set_data_validation_error(&self, error: BindingError) {
        if let Some(owner) = self.owner() {
            owner.on_data_validation_error(error);
        }
    }

    /// Sets the current value to the unset marker and notifies the owner of
    /// the error.
    pub fn set_error(&self, message: &str) {
        self.set_raw_value(Some(FerroProperty::unset_value()));
        if let Some(owner) = self.owner() {
            owner.on_node_error(self.index() as isize, message);
        }
    }

    /// Sets the current value from a value that may be a boxed
    /// [`BindingNotification`].
    pub fn set_value_or_notification(&self, value_or_notification: Option<BoxedValue>) {
        let Some(notification) = value_or_notification.as_ref().and_then(|v| v.downcast_ref::<BindingNotification>())
        else {
            self.set_value(value_or_notification, None);
            return;
        };
        match notification.error_type() {
            BindingErrorType::Error => {
                let message = notification.error().map(|e| e.to_string()).unwrap_or_default();
                self.set_error(&message);
            }
            BindingErrorType::DataValidationError => {
                if notification.has_value() {
                    let inner = notification.value();
                    if inner.as_ref().is_some_and(|v| v.is::<BindingNotification>()) {
                        self.set_value_or_notification(inner);
                    } else {
                        self.set_value(inner, notification.error().as_ref());
                    }
                } else if let Some(error) = notification.error() {
                    self.set_data_validation_error(error);
                }
            }
            BindingErrorType::None => self.set_value_or_notification_inner(notification.value()),
        }
    }

    fn set_value_or_notification_inner(&self, value: Option<BoxedValue>) {
        if value.as_ref().is_some_and(|v| v.is::<BindingNotification>()) {
            self.set_value_or_notification(value);
        } else {
            self.set_value(value, None);
        }
    }

    /// Sets the current value, notifying the owner.
    pub fn set_value(&self, value: Option<BoxedValue>, data_validation_error: Option<&BindingError>) {
        debug_assert!(!value.as_ref().is_some_and(|v| v.is::<BindingNotification>()));
        self.set_raw_value(value.clone());
        if let Some(owner) = self.owner() {
            owner.on_node_value_changed(self.index(), value, data_validation_error);
        }
    }

    /// Validates that the source is non-null, raising a node error if it is
    /// not.
    pub fn validate_non_null_source(&self, source: Option<&BoxedValue>) -> bool {
        if source.is_none() {
            if let Some(owner) = self.owner() {
                owner.on_node_error(self.index() as isize - 1, "Value is null.");
            }
            self.set_raw_value(None);
            return false;
        }
        true
    }
}

/// A node of a binding path that can be written to.
pub trait ISettableNode {
    /// The type of the value accepted by the node, or `None` if the node is
    /// not currently settable.
    fn value_type(&self) -> Option<ValueType>;

    /// Tries to write the specified value to the source.
    fn write_value_to_source(
        &self,
        value: Option<&BoxedValue>,
        nodes: &[Rc<dyn ExpressionNode>],
    ) -> Result<bool, BindingError>;
}

/// A node that reads a property through a property accessor.
pub trait IPropertyAccessorNode {
    fn property_name(&self) -> &str;
    fn accessor(&self) -> Option<Rc<dyn IPropertyAccessor>>;
    fn enable_data_validation(&self);
}

/// A node that selects the source of a binding expression.
pub trait SourceNode {
    /// Selects the source for the binding expression based on the binding
    /// source (`None` when no source was given), target and anchor.
    fn select_source(
        &self,
        source: Option<&Option<BoxedValue>>,
        target: &Ref<FerroObject>,
        _anchor: Option<&Ref<FerroObject>>,
    ) -> Option<BoxedValue> {
        match source {
            Some(source) => source.clone(),
            None => Some(Rc::new(target.clone())),
        }
    }

    fn should_log_errors(&self, state: &NodeState, _target: &FerroObject) -> bool {
        state.value().is_some()
    }
}

/// A node in a binding path: reads a value from its source and passes it to
/// the next node (or, for the last node, to the binding expression).
pub trait ExpressionNode: Any {
    fn state(&self) -> &NodeState;

    /// Appends a string representation of the node to `builder`.
    fn build_string(&self, _builder: &mut String) {}

    /// Builds a string representation of the binding path up to this node.
    fn build_string_with_nodes(&self, builder: &mut String, nodes: &[Rc<dyn ExpressionNode>]) {
        let index = self.state().index();
        if index > 0 {
            nodes[index - 1].build_string_with_nodes(builder, nodes);
        }
        self.build_string(builder);
    }

    /// Subscribes to the new source and updates the current value.
    fn on_source_changed(&self, source: Option<&BoxedValue>, data_validation_error: Option<&BindingError>);

    /// Unsubscribes from the previous source.
    fn unsubscribe(&self, _old_source: &BoxedValue) {}

    fn as_settable(&self) -> Option<&dyn ISettableNode> {
        None
    }

    fn as_property_accessor_node(&self) -> Option<&dyn IPropertyAccessorNode> {
        None
    }

    fn as_source_node(&self) -> Option<&dyn SourceNode> {
        None
    }

    /// Whether the node reads a data context.
    fn is_data_context_node(&self) -> bool {
        false
    }
}

impl dyn ExpressionNode {
    /// Sets the owner binding. Panics if the node already has an owner.
    pub fn set_owner(&self, owner: Weak<BindingExpression>, index: usize) {
        let state = self.state();
        let mut slot = state.owner.borrow_mut();
        if slot.is_some() {
            panic!("The expression node already has an owner.");
        }
        *slot = Some(owner);
        state.index.set(index);
    }

    /// Sets the source from which the node reads its value and updates the
    /// current value, notifying the owner if the value changes.
    pub fn set_source(&self, source: NodeSource, data_validation_error: Option<&BindingError>) {
        let state = self.state();
        let old_source = state.source();
        let unchanged = match (&source, &old_source) {
            (NodeSource::Unset, None) => true,
            (NodeSource::Value(Some(new)), Some(old)) => Rc::ptr_eq(new, old) || (**new).value_eq(&**old),
            _ => false,
        };
        if unchanged {
            return;
        }
        if let Some(old) = &old_source {
            self.unsubscribe(old);
        }
        match source {
            NodeSource::Unset => {
                // If the source is unset then the value is unset. The source
                // change is deliberately not processed: errors must not be
                // raised for subsequent nodes in the binding chain.
                state.source.replace(None);
                state.set_raw_value(Some(FerroProperty::unset_value()));
            }
            NodeSource::Value(value) => {
                state.source.replace(value.as_ref().map(WeakValue::new));
                self.on_source_changed(value.as_ref(), data_validation_error);
            }
        }
    }

    /// Sets the value of the node to null without notifying the owner, and
    /// unsubscribes from the current source.
    pub(crate) fn propagate_null_short_circuit_value(&self) {
        self.set_source(NodeSource::Unset, None);
        self.state().set_raw_value(None);
    }
}

/// Appends a member name to a binding path description.
pub(crate) fn append_member(builder: &mut String, name: &str) {
    if !builder.is_empty() && !builder.ends_with('!') {
        builder.push('.');
    }
    builder.push_str(name);
}
