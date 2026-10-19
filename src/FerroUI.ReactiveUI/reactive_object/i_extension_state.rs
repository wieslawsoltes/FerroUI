//! `IExtensionState<TSender>` (`ReactiveUI.Core/ReactiveObject/IExtensionState.cs`).

use ferroui_system::{IDisposable, IObservable, ObservableError};
use std::rc::Rc;

/// The state of the change notifications of a reactive object (`IExtensionState<TSender>`): the
/// observables of its changes and errors, and the counts of suppression and delay.
///
/// Deviation (DEVIATIONS.md, ReactiveUI): the state is not generic over the sender and its
/// observables carry the names of the properties. A reactive object holds its state in its
/// [`ReactiveObject`](crate::ReactiveObject), which cannot hold the shared handle of the object
/// that holds it; the observables of a handle ([`IReactiveNotifyPropertyChanged`](crate::IReactiveNotifyPropertyChanged))
/// add the sender to each name.
pub trait IExtensionState {
    /// The names of the properties that are about to change (`Changing`).
    fn changing(&self) -> Rc<dyn IObservable<Rc<str>>>;

    /// The names of the properties that changed (`Changed`).
    fn changed(&self) -> Rc<dyn IObservable<Rc<str>>>;

    /// The errors of the subscribers of the changes (`ThrownExceptions`).
    fn thrown_exceptions(&self) -> Rc<dyn IObservable<ObservableError>>;

    /// Makes the raising of a change raise the `PropertyChanging` event of the object
    /// (`SubscribeChanging`). The state does it by itself once the event has a handler.
    fn subscribe_changing(&self);

    /// Raises the changing notifications of `property_name`, unless they are suppressed
    /// (`RaiseChanging`).
    fn raise_changing(&self, property_name: &str);

    /// Makes the raising of a change raise the `PropertyChanged` event of the object
    /// (`SubscribeChanged`). The state does it by itself once the event has a handler.
    fn subscribe_changed(&self);

    /// Raises the changed notifications of `property_name`, unless they are suppressed
    /// (`RaiseChanged`).
    fn raise_changed(&self, property_name: &str);

    /// Whether change notifications are sent (`NotificationsEnabled`).
    fn notifications_enabled(&self) -> bool;

    /// Suppresses change notifications until the result is disposed (`Suppress`).
    fn suppress(&self) -> Rc<dyn IDisposable>;

    /// Whether change notifications are delayed (`AreChangeNotificationsDelayed`).
    fn are_change_notifications_delayed(&self) -> bool;

    /// Delays change notifications until the result is disposed, then sends the last one of each
    /// property (`Delay`).
    fn delay(&self) -> Rc<dyn IDisposable>;
}
