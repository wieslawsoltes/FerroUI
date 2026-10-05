use super::{AccessorListener, IPropertyAccessor, PropertyAccessorBase};
use crate::data::BindingNotification;
use crate::BoxedValue;
use std::rc::Rc;

/// The state shared by data validators: property accessors returned from an
/// [`IDataValidationPlugin`](super::IDataValidationPlugin) that wrap an inner
/// accessor and convert the values received from it into
/// [`BindingNotification`]s.
pub struct DataValidationBase {
    pub base: PropertyAccessorBase,
    pub inner: Rc<dyn IPropertyAccessor>,
}

impl DataValidationBase {
    pub fn new(inner: Rc<dyn IPropertyAccessor>) -> Self {
        Self { base: PropertyAccessorBase::new(), inner }
    }

    /// Begins listening to the inner accessor.
    pub fn subscribe_core(&self, inner_value_changed: AccessorListener) {
        self.inner.subscribe(inner_value_changed);
    }

    /// Stops listening to the inner accessor.
    pub fn unsubscribe_core(&self) {
        self.inner.dispose();
    }

    /// The default handling of a value from the inner accessor: wraps it in a
    /// notification unless it already is one, and publishes it.
    pub fn inner_value_changed(&self, value: Option<BoxedValue>) {
        let notification = wrap_notification(value);
        self.base.publish_value(Some(notification));
    }
}

/// Wraps a value into a boxed notification unless it already is one.
pub(crate) fn wrap_notification(value: Option<BoxedValue>) -> BoxedValue {
    match value {
        Some(v) if v.is::<BindingNotification>() => v,
        other => Rc::new(BindingNotification::new(other)),
    }
}
