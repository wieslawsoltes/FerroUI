use crate::data::core::{ValueType, WeakValue};
use crate::data::{BindingError, BindingPriority};
use crate::reactive::IDisposable;
use crate::{AnyValue, BoxedValue};
use std::rc::Rc;

/// Called by a property accessor with each new value. The value may be a
/// boxed [`BindingNotification`](crate::data::BindingNotification).
pub type AccessorListener = Rc<dyn Fn(Option<BoxedValue>)>;

/// Defines an accessor to a property on an object returned by an
/// [`IPropertyAccessorPlugin`].
pub trait IPropertyAccessor: IDisposable {
    /// The type of the property, or `None` if the accessor is in an error
    /// state.
    fn property_type(&self) -> Option<ValueType>;

    /// The current value of the property.
    fn value(&self) -> Option<BoxedValue>;

    /// Sets the property value. Returns `Ok(false)` if the property cannot be
    /// written; an `Err` is the equivalent of the setter throwing.
    fn set_value(&self, value: Option<&BoxedValue>, priority: BindingPriority) -> Result<bool, BindingError>;

    /// Subscribes to the value of the member. The listener is invoked with
    /// the current value and with each new value.
    fn subscribe(&self, listener: AccessorListener);

    /// Stops the subscription.
    fn unsubscribe(&self);
}

/// Defines how a member is read, written and observed by a binding.
pub trait IPropertyAccessorPlugin {
    /// Checks whether this plugin can handle accessing the properties of the
    /// specified object.
    fn match_(&self, obj: &dyn AnyValue, property_name: &str) -> bool;

    /// Starts monitoring the value of a property on an object.
    fn start(&self, reference: &WeakValue, property_name: &str) -> Option<Rc<dyn IPropertyAccessor>>;
}

/// Defines how data validation is observed by a binding.
pub trait IDataValidationPlugin {
    /// Checks whether this plugin can handle data validation on the specified
    /// object.
    fn match_(&self, reference: &WeakValue, member_name: &str) -> bool;

    /// Starts monitoring the data validation state of a property on an
    /// object, wrapping `inner`.
    fn start(&self, reference: &WeakValue, property_name: &str, inner: Rc<dyn IPropertyAccessor>)
        -> Rc<dyn IPropertyAccessor>;
}

/// Handles compare by identity (reference equality), so that they can be
/// held in property and untyped values.
impl PartialEq for dyn IPropertyAccessor {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
