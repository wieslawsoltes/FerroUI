use super::Event;

/// Notifies clients that a property value of a model object (an object that
/// is not part of the class hierarchy, such as a view model) has changed.
///
/// The event argument is the name of the property that changed; an empty
/// name means that all properties of the object changed.
pub trait INotifyPropertyChanged {
    /// The event raised when a property value changes.
    fn property_changed(&self) -> &Event<str>;
}
