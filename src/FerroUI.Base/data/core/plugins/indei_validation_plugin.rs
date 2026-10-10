use super::{AccessorListener, DataValidationBase, IDataValidationPlugin, IPropertyAccessor};
use crate::data::core::{ValueType, WeakValue};
use crate::data::model::ModelTypes;
use crate::data::{
    AggregateException, BindingError, BindingErrorType, BindingNotification, BindingPriority,
    DataValidationException,
};
use crate::reactive::IDisposable;
use crate::BoxedValue;
use std::cell::Cell;
use std::rc::{Rc, Weak};

/// Validates properties on model objects that implement
/// [`INotifyDataErrorInfo`](crate::data::model::INotifyDataErrorInfo).
pub struct IndeiValidationPlugin;

impl IDataValidationPlugin for IndeiValidationPlugin {
    fn match_(&self, reference: &WeakValue, _member_name: &str) -> bool {
        instance(reference).is_some_and(|t| ModelTypes::find(&*t).is_some_and(|m| m.as_notify_data_error_info(&*t).is_some()))
    }

    fn start(
        &self,
        reference: &WeakValue,
        property_name: &str,
        inner: Rc<dyn IPropertyAccessor>,
    ) -> Rc<dyn IPropertyAccessor> {
        Rc::new_cyclic(|this| Validator {
            this: this.clone(),
            validation: DataValidationBase::new(inner),
            reference: reference.clone(),
            name: property_name.into(),
            token: Cell::new(None),
        })
    }
}

/// The object the reference of a binding refers to, as the object whose contracts are
/// looked up: the object form of a reference type held as its handle (a view model a
/// binding read from a property typed with the handle), the value itself otherwise.
fn instance(reference: &WeakValue) -> Option<BoxedValue> {
    reference.upgrade().map(|target| super::markup_members::instance_of(&target))
}

struct Validator {
    this: Weak<Validator>,
    validation: DataValidationBase,
    reference: WeakValue,
    name: Box<str>,
    token: Cell<Option<u64>>,
}

impl Validator {
    fn create_binding_notification(&self, value: Option<BoxedValue>) -> BoxedValue {
        if let Some(target) = instance(&self.reference) {
            if let Some(model) = ModelTypes::find(&*target) {
                if let Some(indei) = model.as_notify_data_error_info(&*target) {
                    let errors = indei.get_errors(Some(&self.name));
                    if !errors.is_empty() {
                        return Rc::new(BindingNotification::with_error_and_fallback(
                            generate_exception(errors),
                            BindingErrorType::DataValidationError,
                            value,
                        ));
                    }
                }
            }
        }
        Rc::new(BindingNotification::new(value))
    }

    fn remove_handler(&self) {
        if let Some(token) = self.token.take() {
            if let Some(target) = instance(&self.reference) {
                if let Some(model) = ModelTypes::find(&*target) {
                    if let Some(indei) = model.as_notify_data_error_info(&*target) {
                        indei.errors_changed().remove(token);
                    }
                }
            }
        }
    }
}

fn generate_exception(mut errors: Vec<BoxedValue>) -> BindingError {
    if errors.len() == 1 {
        BindingError::new(DataValidationException::new(errors.pop()))
    } else {
        BindingError::new(AggregateException::new(
            errors.into_iter().map(|e| DataValidationException::new(Some(e))).collect(),
        ))
    }
}

impl IDisposable for Validator {
    fn dispose(&self) {
        if self.validation.base.is_subscribed() {
            self.unsubscribe();
        }
    }
}

impl IPropertyAccessor for Validator {
    fn property_type(&self) -> Option<ValueType> {
        self.validation.inner.property_type()
    }

    fn value(&self) -> Option<BoxedValue> {
        self.validation.inner.value()
    }

    fn set_value(&self, value: Option<&BoxedValue>, priority: BindingPriority) -> Result<bool, BindingError> {
        self.validation.inner.set_value(value, priority)
    }

    fn subscribe(&self, listener: AccessorListener) {
        self.validation.base.set_listener(listener);
        if let Some(target) = instance(&self.reference) {
            if let Some(model) = ModelTypes::find(&*target) {
                if let Some(indei) = model.as_notify_data_error_info(&*target) {
                    let weak = self.this.clone();
                    let token = indei.errors_changed().add(Rc::new(move |name: &str| {
                        if let Some(this) = weak.upgrade() {
                            if name.is_empty() || name == &*this.name {
                                let value = this.value();
                                let notification = this.create_binding_notification(value);
                                this.validation.base.publish_value(Some(notification));
                            }
                        }
                    }));
                    self.token.set(Some(token));
                }
            }
        }
        let weak = self.this.clone();
        self.validation.subscribe_core(Rc::new(move |value| {
            if let Some(this) = weak.upgrade() {
                let notification = this.create_binding_notification(value);
                this.validation.base.publish_value(Some(notification));
            }
        }));
    }

    fn unsubscribe(&self) {
        self.remove_handler();
        self.validation.unsubscribe_core();
        self.validation.base.clear_listener();
    }
}

impl Drop for Validator {
    fn drop(&mut self) {
        self.remove_handler();
    }
}
