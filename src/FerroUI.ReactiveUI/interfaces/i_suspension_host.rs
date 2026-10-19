//! `ISuspensionHost` (`ReactiveUI.Shared/Interfaces/ISuspensionHost.cs`).

use crate::IReactiveObject;
use ferroui_system::{IDisposable, IObservable};
use std::any::Any;
use std::rc::Rc;

/// The function that creates a new state of the application (`Func<object>`).
pub type AppStateFactory = Rc<dyn Fn() -> Rc<dyn Any>>;

/// The events of the life of the process that the host operating system publishes, in one form
/// (`ISuspensionHost`): subscribe to them to save and restore the state of the application when
/// it is suspended and resumed.
///
/// The observables abstract the terms of each platform ("launching", "activated", "closing") so
/// that shared code persists its state without knowing the user-interface toolkit. Most
/// applications call
/// [`setup_default_suspend_resume`](crate::SuspensionHostExtensions::setup_default_suspend_resume)
/// at startup; the observables are settable so that a host can plug in its own monitoring.
///
/// The state of the application is an object the application defines, type-erased here
/// ([`app_state`](Self::app_state)); [`create_new_app_state`](Self::create_new_app_state) makes a
/// fresh one on the first launch or after a crash.
// Deviation (DEVIATIONS.md, ReactiveUI): the properties with a getter and a setter are pairs of
// methods (`is_launching_new` and `set_is_launching_new`); `object?` is `Option<Rc<dyn Any>>`.
pub trait ISuspensionHost: IReactiveObject {
    /// Ticks when the application is launched anew, after a crash or the first time: the
    /// application creates its state from scratch (`IsLaunchingNew`).
    fn is_launching_new(&self) -> Rc<dyn IObservable<()>>;

    /// Sets [`is_launching_new`](Self::is_launching_new).
    fn set_is_launching_new(&self, value: Rc<dyn IObservable<()>>);

    /// Ticks when the application resumes from the suspended state: it was running, but its
    /// process was destroyed (`IsResuming`).
    fn is_resuming(&self) -> Rc<dyn IObservable<()>>;

    /// Sets [`is_resuming`](Self::is_resuming).
    fn set_is_resuming(&self, value: Rc<dyn IObservable<()>>);

    /// Ticks when the application is activated; the process may not have been running before
    /// (`IsUnpausing`).
    fn is_unpausing(&self) -> Rc<dyn IObservable<()>>;

    /// Sets [`is_unpausing`](Self::is_unpausing).
    fn set_is_unpausing(&self, value: Rc<dyn IObservable<()>>);

    /// Ticks when the application should persist its state; the value is disposed once the state
    /// is persisted (`ShouldPersistState`).
    fn should_persist_state(&self) -> Rc<dyn IObservable<Rc<dyn IDisposable>>>;

    /// Sets [`should_persist_state`](Self::should_persist_state).
    fn set_should_persist_state(&self, value: Rc<dyn IObservable<Rc<dyn IDisposable>>>);

    /// Ticks when the saved state should be deleted, usually after a crash
    /// (`ShouldInvalidateState`).
    fn should_invalidate_state(&self) -> Rc<dyn IObservable<()>>;

    /// Sets [`should_invalidate_state`](Self::should_invalidate_state).
    fn set_should_invalidate_state(&self, value: Rc<dyn IObservable<()>>);

    /// The function that creates a new state of the application (`CreateNewAppState`).
    fn create_new_app_state(&self) -> Option<AppStateFactory>;

    /// Sets [`create_new_app_state`](Self::create_new_app_state).
    fn set_create_new_app_state(&self, value: Option<AppStateFactory>);

    /// The current state of the application (`AppState`); see
    /// [`get_app_state`](crate::SuspensionHostExtensions::get_app_state) for a typed view.
    fn app_state(&self) -> Option<Rc<dyn Any>>;

    /// Sets [`app_state`](Self::app_state).
    fn set_app_state(&self, value: Option<Rc<dyn Any>>);
}
