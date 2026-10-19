//! `IReactiveNotifyPropertyChanged<TSender>` (`ReactiveUI.Core/Interfaces/IReactiveNotifyPropertyChanged.cs`).

use super::ReactivePropertyChangedEventArgs;
use ferroui_system::{IDisposable, IObservable};
use std::rc::Rc;

/// An object whose property changes are observables as well as events
/// (`IReactiveNotifyPropertyChanged<TSender>`).
///
/// Every reactive object implements it through its shared handle: for `vm: Rc<T>` with
/// `T: IReactiveObject`, `vm.changing()` and `vm.changed()` are observables whose arguments carry
/// `vm` as the sender. The trait is also exported as
/// [`ReactiveObjectRcExt`](crate::ReactiveObjectRcExt).
///
/// Deviation (DEVIATIONS.md, ReactiveUI): the arguments are the concrete
/// [`ReactivePropertyChangedEventArgs`] rather than the interface; the sender is the shared handle
/// of the object.
pub trait IReactiveNotifyPropertyChanged<S> {
    /// Fires when a property is about to change, before its value is stored (`Changing`). The
    /// notifications follow the suppression and the delay of the object.
    fn changing(&self) -> Rc<dyn IObservable<ReactivePropertyChangedEventArgs<S>>>;

    /// Fires when a property has changed (`Changed`).
    fn changed(&self) -> Rc<dyn IObservable<ReactivePropertyChangedEventArgs<S>>>;

    /// Stops both the events and the observables of the changes until the result is disposed
    /// (`SuppressChangeNotifications`); the calls nest.
    fn suppress_change_notifications(&self) -> Rc<dyn IDisposable>;
}
