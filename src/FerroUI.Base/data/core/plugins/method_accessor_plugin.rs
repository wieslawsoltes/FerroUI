use super::markup_members::{self, MethodLookup};
use super::{AccessorListener, IPropertyAccessor, IPropertyAccessorPlugin, PropertyAccessorBase, PropertyError};
use crate::data::core::expression_nodes::MethodCommand;
use crate::data::core::{ValueType, ValueTypes, WeakValue};
use crate::data::model::{ICommand, ModelMethod, ModelType, ModelTypes};
use crate::data::{BindingError, BindingErrorType, BindingNotification, BindingPriority};
use crate::metadata::MarkupDelegate;
use crate::reactive::IDisposable;
use crate::{AnyValue, BoxedValue};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// Reads a declared method of a model object.
///
/// A method declared in markup metadata is read as the managed original
/// reads it: as a delegate bound to the object
/// ([`MarkupDelegate`](crate::metadata::MarkupDelegate)), which the
/// converters of the binding turn into a command for a command-typed target
/// ([`MethodToCommandConverter`](crate::data::converters::MethodToCommandConverter)).
/// Neither keeps the object alive
/// ([`MarkupDelegate::for_method_of_source`]).
/// A method declared in binding metadata ([`ModelTypes`]) has no delegate
/// form: the accessor produces its command directly.
pub struct MethodAccessorPlugin;

impl IPropertyAccessorPlugin for MethodAccessorPlugin {
    fn match_(&self, obj: &dyn AnyValue, method_name: &str) -> bool {
        ModelTypes::find(obj).is_some_and(|m| m.find_method(method_name).is_some())
            || markup_members::has_method(obj, method_name)
    }

    fn start(&self, reference: &WeakValue, method_name: &str) -> Option<Rc<dyn IPropertyAccessor>> {
        let instance = reference.upgrade()?;
        if let Some(model) = ModelTypes::find(&*instance) {
            if let Some(method) = model.find_method(method_name) {
                return Some(Rc::new_cyclic(|this| Accessor {
                    this: this.clone(),
                    base: PropertyAccessorBase::new(),
                    reference: reference.clone(),
                    model,
                    method,
                    command: RefCell::new(None),
                    token: Cell::new(None),
                }));
            }
        }
        let error = match markup_members::find_best_command_method(&*instance, method_name) {
            MethodLookup::Method(declaring_type, method) => {
                let target = markup_members::instance_of(&instance);
                // The delegate has the source as the binding has it: weakly
                // (DEVIATIONS.md, Bindings).
                let delegate = MarkupDelegate::for_method_of_source(&target, declaring_type, method);
                return Some(Rc::new(DelegateAccessor {
                    base: PropertyAccessorBase::new(),
                    value: Rc::new(delegate),
                }));
            }
            MethodLookup::Error(error) => error,
            MethodLookup::None => format!(
                "Could not find CLR method '{}' on '{}'",
                method_name,
                ValueTypes::to_display_string(Some(&instance))
            ),
        };
        Some(Rc::new(PropertyError::new(BindingNotification::with_error(
            BindingError::message(error),
            BindingErrorType::Error,
        ))))
    }
}

/// The accessor of a method read as a delegate.
struct DelegateAccessor {
    base: PropertyAccessorBase,
    value: BoxedValue,
}

impl IDisposable for DelegateAccessor {
    fn dispose(&self) {
        if self.base.is_subscribed() {
            self.unsubscribe();
        }
    }
}

impl IPropertyAccessor for DelegateAccessor {
    fn property_type(&self) -> Option<ValueType> {
        Some(ValueType::of::<MarkupDelegate>())
    }

    fn value(&self) -> Option<BoxedValue> {
        Some(self.value.clone())
    }

    fn set_value(&self, _value: Option<&BoxedValue>, _priority: BindingPriority) -> Result<bool, BindingError> {
        Ok(false)
    }

    fn subscribe(&self, listener: AccessorListener) {
        self.base.set_listener(listener);
        self.base.publish_value(Some(self.value.clone()));
    }

    fn unsubscribe(&self) {
        self.base.clear_listener();
    }
}

struct Accessor {
    this: Weak<Accessor>,
    base: PropertyAccessorBase,
    reference: WeakValue,
    model: Rc<ModelType>,
    method: Rc<ModelMethod>,
    command: RefCell<Option<Rc<MethodCommand>>>,
    token: Cell<Option<u64>>,
}

impl Accessor {
    fn remove_handler(&self) {
        if let Some(token) = self.token.take() {
            if let Some(target) = self.reference.upgrade() {
                if let Some(inpc) = self.model.as_notify_property_changed(&*target) {
                    inpc.property_changed().remove(token);
                }
            }
        }
    }
}

impl IDisposable for Accessor {
    fn dispose(&self) {
        if self.base.is_subscribed() {
            self.unsubscribe();
        }
    }
}

impl IPropertyAccessor for Accessor {
    fn property_type(&self) -> Option<ValueType> {
        Some(ValueType::of::<Rc<dyn ICommand>>())
    }

    fn value(&self) -> Option<BoxedValue> {
        self.command.borrow().as_ref().map(|c| {
            let command: Rc<dyn ICommand> = c.clone();
            Rc::new(command) as BoxedValue
        })
    }

    fn set_value(&self, _value: Option<&BoxedValue>, _priority: BindingPriority) -> Result<bool, BindingError> {
        Ok(false)
    }

    fn subscribe(&self, listener: AccessorListener) {
        self.base.set_listener(listener);
        let command =
            MethodCommand::new(self.reference.clone(), self.method.execute.clone(), self.method.can_execute.clone());
        *self.command.borrow_mut() = Some(command);
        if let Some(target) = self.reference.upgrade() {
            if let Some(inpc) = self.model.as_notify_property_changed(&*target) {
                let weak = self.this.clone();
                let token = inpc.property_changed().add(Rc::new(move |name: &str| {
                    if let Some(this) = weak.upgrade() {
                        if name.is_empty() || this.method.depends_on.iter().any(|d| &**d == name) {
                            let command = this.command.borrow().clone();
                            if let Some(command) = command {
                                command.raise_can_execute_changed();
                            }
                        }
                    }
                }));
                self.token.set(Some(token));
            }
        }
        let value = self.value();
        self.base.publish_value(value);
    }

    fn unsubscribe(&self) {
        self.remove_handler();
        self.base.clear_listener();
    }
}

impl Drop for Accessor {
    fn drop(&mut self) {
        self.remove_handler();
    }
}
