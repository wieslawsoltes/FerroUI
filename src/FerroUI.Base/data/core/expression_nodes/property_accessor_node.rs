use super::{append_member, ExpressionNode, IPropertyAccessorNode, ISettableNode, NodeState};
use crate::data::core::plugins::{BindingPlugins, IPropertyAccessor, IPropertyAccessorPlugin};
use crate::data::core::{ValueType, ValueTypes, WeakValue};
use crate::data::{BindingError, BindingPriority};
use crate::BoxedValue;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// A node that reads a property through a property accessor plugin: a fixed
/// plugin (compiled binding paths) or, when none is given, the first
/// registered plugin that matches the source (string binding paths).
pub struct PropertyAccessorNode {
    this: Weak<PropertyAccessorNode>,
    state: NodeState,
    property_name: Box<str>,
    plugin: Option<Rc<dyn IPropertyAccessorPlugin>>,
    accepts_null: bool,
    accessor: RefCell<Option<Rc<dyn IPropertyAccessor>>>,
    enable_data_validation: Cell<bool>,
}

impl PropertyAccessorNode {
    /// Creates a node that uses `plugin` to access the property.
    pub fn new(property_name: &str, plugin: Rc<dyn IPropertyAccessorPlugin>, accepts_null: bool) -> Rc<Self> {
        Self::create(property_name, Some(plugin), accepts_null)
    }

    /// Creates a node that selects the plugin from the registered plugins
    /// each time its source changes.
    pub fn new_dynamic(property_name: &str, accepts_null: bool) -> Rc<Self> {
        Self::create(property_name, None, accepts_null)
    }

    fn create(property_name: &str, plugin: Option<Rc<dyn IPropertyAccessorPlugin>>, accepts_null: bool) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            state: NodeState::new(),
            property_name: property_name.into(),
            plugin,
            accepts_null,
            accessor: RefCell::new(None),
            enable_data_validation: Cell::new(false),
        })
    }

    fn start(&self, source: &BoxedValue, reference: &WeakValue) -> Result<Option<Rc<dyn IPropertyAccessor>>, String> {
        match &self.plugin {
            Some(plugin) => Ok(plugin.start(reference, &self.property_name)),
            None => {
                let plugin = BindingPlugins::property_accessors()
                    .into_iter()
                    .find(|p| p.match_(&**source, &self.property_name));
                match plugin.and_then(|p| p.start(reference, &self.property_name)) {
                    Some(accessor) => Ok(Some(accessor)),
                    // The type reported is the run-time type of the source:
                    // the class of an object, the value type otherwise.
                    None => Err(format!(
                        "Could not find a matching property accessor for '{}' on '{}'.",
                        self.property_name,
                        match ValueTypes::as_object(&**source) {
                            Some(o) => o.get_type().name().to_string(),
                            None => ValueTypes::type_full_name(ValueType::of_value(&**source)),
                        }
                    )),
                }
            }
        }
    }
}

impl ExpressionNode for PropertyAccessorNode {
    fn state(&self) -> &NodeState {
        &self.state
    }

    fn build_string(&self, builder: &mut String) {
        append_member(builder, &self.property_name);
    }

    fn on_source_changed(&self, source: Option<&BoxedValue>, _data_validation_error: Option<&BindingError>) {
        let Some(source) = source else {
            if self.accepts_null {
                self.state.short_circuit_null();
            } else {
                self.state.validate_non_null_source(source);
            }
            return;
        };

        let reference = WeakValue::new(source);
        match self.start(source, &reference) {
            Ok(Some(mut accessor)) => {
                if self.enable_data_validation.get() {
                    for validator in BindingPlugins::data_validators() {
                        if validator.match_(&reference, &self.property_name) {
                            accessor = validator.start(&reference, &self.property_name, accessor);
                        }
                    }
                }
                self.accessor.replace(Some(accessor.clone()));
                let weak = self.this.clone();
                accessor.subscribe(Rc::new(move |value| {
                    if let Some(this) = weak.upgrade() {
                        this.state.set_value_or_notification(value);
                    }
                }));
            }
            Ok(None) => self.state.clear_value(),
            Err(message) => self.state.set_error(&message),
        }
    }

    fn unsubscribe(&self, _old_source: &BoxedValue) {
        let accessor = self.accessor.borrow_mut().take();
        if let Some(accessor) = accessor {
            accessor.dispose();
        }
    }

    fn as_settable(&self) -> Option<&dyn ISettableNode> {
        Some(self)
    }

    fn as_property_accessor_node(&self) -> Option<&dyn IPropertyAccessorNode> {
        Some(self)
    }
}

impl IPropertyAccessorNode for PropertyAccessorNode {
    fn property_name(&self) -> &str {
        &self.property_name
    }

    fn accessor(&self) -> Option<Rc<dyn IPropertyAccessor>> {
        self.accessor.borrow().clone()
    }

    fn enable_data_validation(&self) {
        self.enable_data_validation.set(true);
    }
}

impl ISettableNode for PropertyAccessorNode {
    fn value_type(&self) -> Option<ValueType> {
        self.accessor().and_then(|a| a.property_type())
    }

    fn write_value_to_source(
        &self,
        value: Option<&BoxedValue>,
        _nodes: &[Rc<dyn ExpressionNode>],
    ) -> Result<bool, BindingError> {
        match self.accessor() {
            Some(accessor) if accessor.property_type().is_some() => {
                accessor.set_value(value, BindingPriority::LocalValue)
            }
            _ => Ok(false),
        }
    }
}

impl Drop for PropertyAccessorNode {
    fn drop(&mut self) {
        if let Some(accessor) = self.accessor.get_mut().take() {
            accessor.dispose();
        }
    }
}
