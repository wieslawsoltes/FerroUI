//! `ISuspensionDriver` (`ReactiveUI.Shared/Interfaces/ISuspensionDriver.cs`).

use ferroui_system::IObservable;
use std::any::Any;
use std::rc::Rc;

/// A driver that loads and saves the state of the application to persistent storage
/// (`ISuspensionDriver`).
///
/// The driver owns the serialization: the state reaches it as the object itself, and it is free
/// to write it in any format. [`DummySuspensionDriver`](crate::DummySuspensionDriver) stores
/// nothing.
// Deviation (DEVIATIONS.md, ReactiveUI): upstream's `SaveState<T>(T)` and `LoadState()` serialize
// by reflection (`System.Text.Json`); here the state is the type-erased object of the host
// (`Option<Rc<dyn Any>>`, upstream's `object?`) and the serializer belongs to the driver. The
// overloads that take a `JsonTypeInfo<T>` are the typed trait [`ISuspensionDriverOf<T>`].
pub trait ISuspensionDriver {
    /// The persisted state of the application, or `None` when there is none (`LoadState()`).
    fn load_state(&self) -> Rc<dyn IObservable<Option<Rc<dyn Any>>>>;

    /// Saves the state of the application; the observable completes once it is saved
    /// (`SaveState<T>(T)`).
    fn save_state(&self, state: Option<Rc<dyn Any>>) -> Rc<dyn IObservable<()>>;

    /// Invalidates the persisted state, for example by deleting it; the observable completes once
    /// it is invalidated (`InvalidateState()`).
    fn invalidate_state(&self) -> Rc<dyn IObservable<()>>;
}

/// A driver that loads and saves a state of the type `T` (the overloads of `ISuspensionDriver`
/// that take a `JsonTypeInfo<T>`), which the typed host
/// [`SuspensionHostOf<T>`](crate::SuspensionHostOf) uses.
// Deviation (DEVIATIONS.md, ReactiveUI): upstream passes the source-generated serialization
// metadata of `T` (`JsonTypeInfo<T>`) to each call; here the type is a parameter of the trait and
// the driver knows how to serialize it. A driver registered in `AppLocator` for a typed host is
// registered as `dyn ISuspensionDriverOf<T>`.
pub trait ISuspensionDriverOf<T: 'static>: ISuspensionDriver {
    /// The persisted state, or `None` when there is none (`LoadState<T>(JsonTypeInfo<T>)`).
    fn load_state_of(&self) -> Rc<dyn IObservable<Option<Rc<T>>>>;

    /// Saves the state; the observable completes once it is saved
    /// (`SaveState<T>(T, JsonTypeInfo<T>)`).
    fn save_state_of(&self, state: Option<Rc<T>>) -> Rc<dyn IObservable<()>>;
}
