use crate::events::Event;

/// Notifies clients that a property value of a model object is changing
/// (`System.ComponentModel.INotifyPropertyChanging`).
///
/// The event argument is the name of the property that is about to change;
/// an empty name means that all properties of the object are changing. The
/// event is raised before the new value is stored, so a handler reads the
/// old value.
///
/// An object that raises both events also overrides
/// [`INotifyPropertyChanged::as_notify_property_changing`](super::INotifyPropertyChanged::as_notify_property_changing),
/// which is how an observer that holds the object as `INotifyPropertyChanged`
/// finds this event without reflection.
pub trait INotifyPropertyChanging {
    /// The event raised when a property value is about to change.
    fn property_changing(&self) -> &Event<str>;
}
