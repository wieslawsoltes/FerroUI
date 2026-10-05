use super::{AccessorListener, IPropertyAccessor};
use crate::data::core::ValueType;
use crate::data::{BindingError, BindingNotification, BindingPriority};
use crate::reactive::IDisposable;
use crate::BoxedValue;
use std::rc::Rc;

/// An [`IPropertyAccessor`] that represents an error.
pub struct PropertyError {
    error: BoxedValue,
}

impl PropertyError {
    pub fn new(error: BindingNotification) -> Self {
        Self { error: Rc::new(error) }
    }
}

impl IDisposable for PropertyError {
    fn dispose(&self) {}
}

impl IPropertyAccessor for PropertyError {
    fn property_type(&self) -> Option<ValueType> {
        None
    }

    fn value(&self) -> Option<BoxedValue> {
        Some(self.error.clone())
    }

    fn set_value(&self, _value: Option<&BoxedValue>, _priority: BindingPriority) -> Result<bool, BindingError> {
        Ok(false)
    }

    fn subscribe(&self, listener: AccessorListener) {
        listener(Some(self.error.clone()));
    }

    fn unsubscribe(&self) {}
}
