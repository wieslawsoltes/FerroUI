//! `ReactivePropertyChangingEventArgs<TSender>` (`ReactiveUI.Core/Interfaces/ReactivePropertyChangingEventArgs.cs`).

use super::IReactivePropertyChangedEventArgs;

/// The arguments of a property of a reactive object that is about to change
/// (`ReactivePropertyChangingEventArgs<TSender>`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReactivePropertyChangingEventArgs<S> {
    /// The object whose property is changing (`Sender`).
    pub sender: S,
    /// The name of the property; empty when every property is changing (`PropertyName`).
    pub property_name: String,
}

impl<S> ReactivePropertyChangingEventArgs<S> {
    /// The arguments of a change of `property_name` of `sender` that is about to happen.
    pub fn new(sender: S, property_name: impl Into<String>) -> Self {
        Self { sender, property_name: property_name.into() }
    }
}

impl<S> IReactivePropertyChangedEventArgs<S> for ReactivePropertyChangingEventArgs<S> {
    fn property_name(&self) -> &str {
        &self.property_name
    }

    fn sender(&self) -> &S {
        &self.sender
    }
}
