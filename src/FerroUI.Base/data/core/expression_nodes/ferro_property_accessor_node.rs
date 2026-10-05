use super::{append_member, ExpressionNode, ISettableNode, NodeState};
use crate::data::core::plugins::{get_property_value, property_value_type, set_property_value};
use crate::data::core::{ValueType, ValueTypes};
use crate::data::{BindingError, BindingPriority};
use crate::reactive::IDisposable;
use crate::{BoxedValue, FerroProperty};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// A node that reads a registered property of an object of the class
/// hierarchy.
pub struct FerroPropertyAccessorNode {
    this: Weak<FerroPropertyAccessorNode>,
    state: NodeState,
    property: &'static FerroProperty,
    accepts_null: bool,
    subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

impl FerroPropertyAccessorNode {
    pub fn new(property: &'static FerroProperty, accepts_null: bool) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            state: NodeState::new(),
            property,
            accepts_null,
            subscription: RefCell::new(None),
        })
    }

    pub fn property(&self) -> &'static FerroProperty {
        self.property
    }
}

impl ExpressionNode for FerroPropertyAccessorNode {
    fn state(&self) -> &NodeState {
        &self.state
    }

    fn build_string(&self, builder: &mut String) {
        append_member(builder, self.property.name());
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

        if let Some(object) = ValueTypes::as_object(&**source) {
            let weak = self.this.clone();
            let id = self.property.id();
            let subscription = object.property_changed(move |e| {
                if e.property().id() == id {
                    if let Some(this) = weak.upgrade() {
                        if let Some(o) = this.state.source_object() {
                            this.state.set_value(get_property_value(&o, this.property), None);
                        }
                    }
                }
            });
            self.subscription.replace(Some(subscription));
            self.state.set_value(get_property_value(&object, self.property), None);
        }
    }

    fn unsubscribe(&self, _old_source: &BoxedValue) {
        let subscription = self.subscription.borrow_mut().take();
        if let Some(s) = subscription {
            s.dispose();
        }
    }

    fn as_settable(&self) -> Option<&dyn ISettableNode> {
        Some(self)
    }
}

impl ISettableNode for FerroPropertyAccessorNode {
    fn value_type(&self) -> Option<ValueType> {
        Some(property_value_type(self.property))
    }

    fn write_value_to_source(
        &self,
        value: Option<&BoxedValue>,
        _nodes: &[Rc<dyn ExpressionNode>],
    ) -> Result<bool, BindingError> {
        match self.state.source_object() {
            Some(o) => {
                set_property_value(&o, self.property, value, BindingPriority::LocalValue)?;
                Ok(true)
            }
            None => Ok(false),
        }
    }
}

impl Drop for FerroPropertyAccessorNode {
    fn drop(&mut self) {
        if let Some(s) = self.subscription.get_mut().take() {
            s.dispose();
        }
    }
}
