//! `IReactivePropertyChangedEventArgs<TSender>` (`ReactiveUI.Core/Interfaces/IReactivePropertyChangedEventArgs.cs`).

/// The arguments of a change of a property of a reactive object: the property and the object
/// (`IReactivePropertyChangedEventArgs<TSender>`).
pub trait IReactivePropertyChangedEventArgs<S> {
    /// The name of the property that changed or is changing; empty when every property did
    /// (`PropertyName`, where upstream's `null` is every property).
    fn property_name(&self) -> &str;

    /// The object whose property changed (`Sender`).
    fn sender(&self) -> &S;
}
