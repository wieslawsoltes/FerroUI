use super::INotifyPropertyChanging;
use crate::events::Event;

/// Notifies clients that a property value of a model object (an object that
/// is not part of the class hierarchy, such as a view model) has changed.
///
/// The event argument is the name of the property that changed; an empty
/// name means that all properties of the object changed.
pub trait INotifyPropertyChanged {
    /// The event raised when a property value changes.
    fn property_changed(&self) -> &Event<str>;

    /// The object as [`INotifyPropertyChanging`], when it also raises the event of a property
    /// that is about to change; `None` by default.
    ///
    /// An observer that holds the object as `INotifyPropertyChanged` uses this to find the
    /// changing event, where .NET casts the object to the other interface.
    fn as_notify_property_changing(&self) -> Option<&dyn INotifyPropertyChanging> {
        None
    }
}
