//! `ReactivePropertyChangedEventArgs<TSender>` (`ReactiveUI.Core/Interfaces/ReactivePropertyChangedEventArgs.cs`).

use super::IReactivePropertyChangedEventArgs;

/// The arguments of a change of a property of a reactive object
/// (`ReactivePropertyChangedEventArgs<TSender>`): the object that raised it and the name of the
/// property. The `Changed` and `Changing` observables of a reactive object deliver these.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReactivePropertyChangedEventArgs<S> {
    /// The object whose property changed (`Sender`).
    pub sender: S,
    /// The name of the property; empty when every property changed (`PropertyName`).
    pub property_name: String,
}

impl<S> ReactivePropertyChangedEventArgs<S> {
    /// The arguments of a change of `property_name` of `sender`.
    pub fn new(sender: S, property_name: impl Into<String>) -> Self {
        Self { sender, property_name: property_name.into() }
    }
}

impl<S> IReactivePropertyChangedEventArgs<S> for ReactivePropertyChangedEventArgs<S> {
    fn property_name(&self) -> &str {
        &self.property_name
    }

    fn sender(&self) -> &S {
        &self.sender
    }
}
