use super::DataErrorsChangedEventArgs;
use crate::events::Event;
use std::any::Any;
use std::rc::Rc;

/// Lets a model object report the validation errors of its properties, synchronously or
/// asynchronously (`System.ComponentModel.INotifyDataErrorInfo`).
///
/// The errors are untyped, as .NET's `IEnumerable` of errors is: a message is a `String`, and
/// anything else is the value the model reports. A reader picks the kinds it shows with
/// `downcast_ref` (`OfType<string>()`).
pub trait INotifyDataErrorInfo {
    /// Whether the object has validation errors (`HasErrors`).
    fn has_errors(&self) -> bool;

    /// The validation errors of the property named `property_name`, or the errors of the whole
    /// object when it is `None` or empty (`GetErrors(string?)`); empty when there are none.
    fn get_errors(&self, property_name: Option<&str>) -> Vec<Rc<dyn Any>>;

    /// The event raised when the validation errors of a property, or of the whole object, have
    /// changed (`ErrorsChanged`).
    fn errors_changed(&self) -> &Event<DataErrorsChangedEventArgs>;
}
