use super::Event;
use crate::BoxedValue;

/// Lets a model object report validation errors for its properties
/// (synchronously or asynchronously).
pub trait INotifyDataErrorInfo {
    /// Whether the object has validation errors.
    fn has_errors(&self) -> bool;

    /// The validation errors of the property named `property_name`, or the
    /// object-level errors when it is `None` or empty. Errors are untyped:
    /// strings and boxed [`BindingError`](crate::data::BindingError) values
    /// are shown as they are.
    fn get_errors(&self, property_name: Option<&str>) -> Vec<BoxedValue>;

    /// The event raised when the validation errors of a property (the event
    /// argument; empty for the whole object) have changed.
    fn errors_changed(&self) -> &Event<str>;
}
