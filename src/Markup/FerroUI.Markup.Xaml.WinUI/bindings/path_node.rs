//! A lifecycle gate before the ordinary binding engine's source/target conversion pipeline.

use super::{WinUIBindingDiagnostic, WinUIBindingScope, WinUIBindings};
use ferroui_base::controls::INameScope;
use ferroui_base::data::core::expression_nodes::{
    CastTarget, ExpressionNode, ISettableNode, NodeSource, NodeState,
};
use ferroui_base::data::core::plugins::{BindingPlugins, IPropertyAccessor};
use ferroui_base::data::core::{UntypedBindingExpression, ValueType, ValueTypes, WeakValue};
use ferroui_base::data::model::ModelTypes;
use ferroui_base::data::{
    BindingError, BindingErrorType, BindingMode, BindingNotification, BindingPriority,
};
use ferroui_base::metadata::MarkupType;
use ferroui_base::{BoxedValue, FerroPropertyRegistry, StyledElement};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

pub(super) struct RootGate {
    state: NodeState,
    group: Weak<WinUIBindings>,
    scope: WinUIBindingScope,
    data_type: Option<ValueType>,
    path: String,
    line: i32,
    column: i32,
    enabled: Cell<bool>,
    error: RefCell<Option<String>>,
    nodes: RefCell<Vec<Weak<dyn ExpressionNode>>>,
}

impl RootGate {
    pub fn new(
        group: &Rc<WinUIBindings>,
        scope: WinUIBindingScope,
        data_type: Option<ValueType>,
        path: String,
        line: i32,
        column: i32,
    ) -> Rc<Self> {
        Rc::new(Self {
            state: NodeState::locating_an_element(),
            group: Rc::downgrade(group),
            scope,
            data_type,
            path,
            line,
            column,
            enabled: Cell::new(group.is_initialized()),
            error: RefCell::new(None),
            nodes: RefCell::new(Vec::new()),
        })
    }

    pub fn set_nodes(&self, nodes: &[Rc<dyn ExpressionNode>]) {
        *self.nodes.borrow_mut() = nodes.iter().map(Rc::downgrade).collect();
    }

    pub fn refresh(&self) {
        self.enabled.set(true);
        self.clear_path_sources();
        if self
            .state
            .owner()
            .is_some_and(|owner| owner.base().is_running())
        {
            self.publish_source();
        }
    }

    pub fn pause(&self) {
        self.enabled.set(false);
        self.clear_path_sources();
    }

    fn clear_path_sources(&self) {
        let nodes: Vec<_> = self
            .nodes
            .borrow()
            .iter()
            .filter_map(Weak::upgrade)
            .collect();
        // Unset releases listeners without publishing a new target value.
        for node in nodes {
            node.set_source(NodeSource::Unset, None);
        }
    }

    fn publish_source(&self) {
        let Some(group) = self.group.upgrade().filter(|group| group.is_initialized()) else {
            return;
        };
        if !self.enabled.get() {
            return;
        }
        self.error.borrow_mut().take();
        let Some(root) = group.root.upgrade() else {
            self.fail("The declaring root has been released");
            return;
        };
        let source = match self.scope {
            WinUIBindingScope::Root => Some(Rc::new(root) as BoxedValue),
            WinUIBindingScope::Template => {
                let data = root
                    .cast::<StyledElement>()
                    .and_then(|root| root.data_context());
                match (self.data_type, data) {
                    (Some(type_), Some(data)) if CastTarget::Value(type_).is_instance(&data) => {
                        Some(data)
                    }
                    (Some(_), None) => {
                        self.fail("The typed template item is null; initialize after assigning DataContext");
                        return;
                    }
                    (Some(type_), Some(_)) => {
                        self.fail(&format!(
                            "The template item does not match declared type '{}'",
                            type_.name()
                        ));
                        return;
                    }
                    (None, _) => {
                        self.fail("A template binding requires x:DataType");
                        return;
                    }
                }
            }
        };
        self.state.set_value(source, None);
    }

    fn fail(&self, message: &str) {
        self.record_error(message.to_string());
        self.state.set_error(message);
    }

    pub fn record_error(&self, message: String) {
        *self.error.borrow_mut() = Some(message);
    }

    pub fn clear_error(&self) {
        self.error.borrow_mut().take();
    }

    pub fn diagnostic(&self) -> Option<WinUIBindingDiagnostic> {
        let message = self.error.borrow().clone().or_else(|| {
            self.state
                .owner()
                .filter(|owner| owner.error_type() != BindingErrorType::None)
                .map(|_| {
                    "The path could not be evaluated or converted to the target property type"
                        .into()
                })
        })?;
        Some(WinUIBindingDiagnostic {
            path: self.path.clone(),
            message,
            line: self.line,
            column: self.column,
        })
    }
}

impl ExpressionNode for RootGate {
    fn state(&self) -> &NodeState {
        &self.state
    }
    fn on_source_changed(&self, _: Option<&BoxedValue>, _: Option<&BindingError>) {
        self.publish_source();
    }
    // The binding engine's OneTime reset point is a source-scope change. This private node
    // publishes only at an explicit Update/Initialize or a typed template item replacement.
    fn is_data_context_node(&self) -> bool {
        true
    }
}

/// Property nodes use the existing registered property/model accessors. Unlike an ordinary
/// OneTime Binding, WinUI OneTime reads without subscribing at all. Located UI/model objects
/// are held weakly between reads to avoid root -> target -> binding -> root ownership cycles.
pub(super) struct PathNode {
    this: Weak<Self>,
    state: NodeState,
    gate: Weak<RootGate>,
    name: String,
    mode: BindingMode,
    first_scope: Option<WinUIBindingScope>,
    name_scope: Option<Weak<dyn INameScope>>,
    leaf: bool,
    accessor: RefCell<Option<Rc<dyn IPropertyAccessor>>>,
}

impl PathNode {
    pub fn new(
        gate: &Rc<RootGate>,
        name: &str,
        mode: BindingMode,
        first_scope: Option<WinUIBindingScope>,
        name_scope: Option<Weak<dyn INameScope>>,
        leaf: bool,
    ) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            state: NodeState::locating_an_element(),
            gate: Rc::downgrade(gate),
            name: name.into(),
            mode,
            first_scope,
            name_scope,
            leaf,
            accessor: RefCell::new(None),
        })
    }

    fn named_element(&self) -> Option<BoxedValue> {
        self.first_scope?;
        let scope = self.name_scope.as_ref()?.upgrade()?;
        scope
            .find(&self.name)
            .map(|object| Rc::new(object) as BoxedValue)
    }

    fn publish_named(&self, value: BoxedValue) {
        if self.leaf && self.mode == BindingMode::TwoWay {
            self.fail("A named-element field cannot be replaced by a TwoWay binding".into());
        } else {
            self.publish(Some(value));
        }
    }

    fn publish(&self, value: Option<BoxedValue>) {
        let error = value
            .as_ref()
            .and_then(|value| value.downcast_ref::<BindingNotification>())
            .filter(|notification| notification.error_type() != BindingErrorType::None)
            .and_then(|notification| notification.error())
            .map(|error| error.to_string());
        if let Some(gate) = self.gate.upgrade() {
            if let Some(error) = error {
                gate.record_error(error);
            } else if self.leaf {
                gate.clear_error();
            }
        }
        self.state.set_value_or_notification(value);
    }

    fn fail(&self, message: String) {
        if let Some(gate) = self.gate.upgrade() {
            gate.record_error(message.clone());
        }
        self.state.set_error(&message);
    }
}

impl ExpressionNode for PathNode {
    fn state(&self) -> &NodeState {
        &self.state
    }
    fn build_string(&self, builder: &mut String) {
        if !builder.is_empty() {
            builder.push('.');
        }
        builder.push_str(&self.name);
    }

    fn on_source_changed(&self, source: Option<&BoxedValue>, _: Option<&BindingError>) {
        // WinUI's file root favors generated named-element fields. A typed DataTemplate
        // favors members of its item type, with a named-element fallback.
        let named = self.named_element();
        if self.first_scope == Some(WinUIBindingScope::Root) {
            if let Some(named) = named {
                self.publish_named(named);
                return;
            }
        }
        let Some(source) = source else {
            self.fail(format!("Cannot read '{}' from a null source", self.name));
            return;
        };
        let Some((readable, writable)) = property_capabilities(source, &self.name) else {
            if let Some(named) = self.named_element() {
                self.publish_named(named);
                return;
            }
            self.fail(format!("No declared property '{}' exists on this source; functions and methods are unsupported", self.name));
            return;
        };
        if !readable || (self.leaf && self.mode == BindingMode::TwoWay && !writable) {
            self.fail(format!(
                "Property '{}' does not declare the required {} accessor",
                self.name,
                if readable { "write" } else { "read" }
            ));
            return;
        }
        let reference = WeakValue::new(source);
        let accessor = BindingPlugins::property_accessors()
            .into_iter()
            .find(|plugin| plugin.match_(&**source, &self.name))
            .and_then(|plugin| plugin.start(&reference, &self.name));
        let Some(accessor) = accessor else {
            self.fail(format!(
                "No registered readable member '{}' exists on '{}'",
                self.name,
                ValueTypes::as_object(&**source).map_or_else(
                    || ValueTypes::type_full_name(ValueType::of_value(&**source)),
                    |object| object.get_type().name().to_string()
                )
            ));
            return;
        };
        if self.mode == BindingMode::OneTime {
            let value = accessor.value();
            accessor.dispose();
            self.publish(value);
        } else {
            self.accessor.replace(Some(accessor.clone()));
            let weak = self.this.clone();
            accessor.subscribe(Rc::new(move |value| {
                if let Some(node) = weak.upgrade() {
                    node.publish(value);
                }
            }));
        }
    }

    fn unsubscribe(&self, _: &BoxedValue) {
        let accessor = self.accessor.borrow_mut().take();
        if let Some(accessor) = accessor {
            accessor.dispose();
        }
    }
    fn as_settable(&self) -> Option<&dyn ISettableNode> {
        Some(self)
    }
}

impl ISettableNode for PathNode {
    fn value_type(&self) -> Option<ValueType> {
        self.accessor
            .borrow()
            .as_ref()
            .and_then(|a| a.property_type())
    }
    fn write_value_to_source(
        &self,
        value: Option<&BoxedValue>,
        _: &[Rc<dyn ExpressionNode>],
    ) -> Result<bool, BindingError> {
        let accessor = self.accessor.borrow().clone();
        let result = match accessor {
            Some(accessor) => accessor.set_value(value, BindingPriority::LocalValue),
            None => Ok(false),
        };
        if let Some(gate) = self.gate.upgrade() {
            match &result {
                Err(error) => gate.record_error(error.to_string()),
                Ok(false) if gate.enabled.get() => {
                    gate.record_error(format!("Property '{}' could not be written", self.name))
                }
                _ => {}
            }
        }
        result
    }
}

/// Methods that an ordinary binding can turn into commands are deliberately not properties
/// of this subset. Metadata also lets TwoWay reject a read-only source before the first edit.
fn property_capabilities(source: &BoxedValue, name: &str) -> Option<(bool, bool)> {
    let object = ValueTypes::as_object(&**source);
    if let Some(property) = object
        .as_ref()
        .and_then(|object| FerroPropertyRegistry::instance().find_registered_for(object, name))
    {
        return Some((true, !property.is_read_only()));
    }
    if let Some(property) = ModelTypes::find(&**source).and_then(|model| model.find_property(name))
    {
        return Some((property.can_get(), property.can_set()));
    }
    let mut metadata = object
        .as_ref()
        .and_then(|object| object.get_type().markup())
        .or_else(|| MarkupType::find_by_handle(ValueType::of_value(&**source).id()));
    for _ in 0..64 {
        let current = metadata?;
        if let Some(property) = current.find_property(name) {
            return Some((property.get.is_some(), property.set.is_some()));
        }
        metadata = current.base_type();
    }
    None
}

impl Drop for PathNode {
    fn drop(&mut self) {
        if let Some(accessor) = self.accessor.get_mut().take() {
            accessor.dispose();
        }
    }
}
