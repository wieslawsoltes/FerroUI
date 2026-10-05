use super::{AccessorListener, DataValidationBase, IDataValidationPlugin, IPropertyAccessor};
use crate::data::core::{ValueType, WeakValue};
use crate::data::{BindingError, BindingErrorType, BindingNotification, BindingPriority};
use crate::reactive::IDisposable;
use crate::BoxedValue;
use std::rc::{Rc, Weak};

/// Validates properties that report errors by failing in their setter.
pub struct ExceptionValidationPlugin;

impl IDataValidationPlugin for ExceptionValidationPlugin {
    fn match_(&self, _reference: &WeakValue, _member_name: &str) -> bool {
        true
    }

    fn start(
        &self,
        _reference: &WeakValue,
        _property_name: &str,
        inner: Rc<dyn IPropertyAccessor>,
    ) -> Rc<dyn IPropertyAccessor> {
        Rc::new_cyclic(|this| Validator { this: this.clone(), validation: DataValidationBase::new(inner) })
    }
}

struct Validator {
    this: Weak<Validator>,
    validation: DataValidationBase,
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
        match self.validation.inner.set_value(value, priority) {
            Ok(result) => Ok(result),
            Err(error) => {
                self.validation.base.publish_value(Some(Rc::new(BindingNotification::with_error(
                    error,
                    BindingErrorType::DataValidationError,
                ))));
                Ok(false)
            }
        }
    }

    fn subscribe(&self, listener: AccessorListener) {
        self.validation.base.set_listener(listener);
        let weak = self.this.clone();
        self.validation.subscribe_core(Rc::new(move |value| {
            if let Some(this) = weak.upgrade() {
                this.validation.inner_value_changed(value);
            }
        }));
    }

    fn unsubscribe(&self) {
        self.validation.unsubscribe_core();
        self.validation.base.clear_listener();
    }
}
