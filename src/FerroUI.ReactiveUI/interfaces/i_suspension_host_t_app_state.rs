//! `ISuspensionHost<TAppState>` (`ReactiveUI.Shared/Interfaces/ISuspensionHost{TAppState}.cs`).

use crate::ISuspensionHost;
use ferroui_system::IObservable;
use std::rc::Rc;

/// The function that creates a new state of the type `T` (`Func<TAppState>`).
pub type TypedAppStateFactory<T> = Rc<dyn Fn() -> Rc<T>>;

/// A suspension host whose state of the application has the type `T` (`ISuspensionHost<TAppState>`).
///
/// It keeps the observables of [`ISuspensionHost`] and adds the typed state; the untyped
/// [`app_state`](ISuspensionHost::app_state) of an implementation is a view of the typed one.
/// The state is shared (`Rc<T>`), as the class of upstream is a reference.
// Deviation (DEVIATIONS.md, ReactiveUI): named `ISuspensionHostOf<T>`, since Rust has no
// overloading on the number of type parameters; the state is an `Rc<T>`, upstream's class.
pub trait ISuspensionHostOf<T: 'static>: ISuspensionHost {
    /// The function that creates a new state (`CreateNewAppStateTyped`).
    fn create_new_app_state_typed(&self) -> Option<TypedAppStateFactory<T>>;

    /// Sets [`create_new_app_state_typed`](Self::create_new_app_state_typed).
    fn set_create_new_app_state_typed(&self, value: Option<TypedAppStateFactory<T>>);

    /// The current state (`AppStateValue`).
    fn app_state_value(&self) -> Option<Rc<T>>;

    /// Sets [`app_state_value`](Self::app_state_value).
    fn set_app_state_value(&self, value: Option<Rc<T>>);

    /// Ticks with the state each time [`app_state_value`](Self::app_state_value) is set; it does
    /// not replay the current one (`AppStateValueChanged`).
    fn app_state_value_changed(&self) -> Rc<dyn IObservable<Option<Rc<T>>>>;
}
