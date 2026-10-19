//! `SuspensionHost<TAppState>` (`ReactiveUI.Shared/Suspension/SuspensionHost{TAppState}.cs`).

use super::suspension_host::{not_set_up, same_object, SwitchedObservable};
use crate::{AppStateFactory, IReactiveObject, ISuspensionHost, ISuspensionHostOf, ReactiveObject, TypedAppStateFactory};
use ferroui_reactive::subjects::Subject;
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::any::Any;
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

/// A suspension host whose state of the application has the type `T` (`SuspensionHost<TAppState>`).
///
/// It has the observables of [`SuspensionHost`](crate::SuspensionHost) and `IsContinuing`, and
/// a typed state with the observable of its changes. Through [`ISuspensionHost`] the state is
/// seen untyped: setting an untyped state of another type panics (`InvalidCastException`).
// Deviation (DEVIATIONS.md, ReactiveUI): named `SuspensionHostOf<T>` (Rust has no overloading on
// the number of type parameters); the state is an `Rc<T>`.
pub struct SuspensionHostOf<T: 'static> {
    reactive: ReactiveObject,
    is_launching_new: SwitchedObservable<()>,
    is_resuming: SwitchedObservable<()>,
    is_unpausing: SwitchedObservable<()>,
    should_persist_state: SwitchedObservable<Rc<dyn IDisposable>>,
    should_invalidate_state: SwitchedObservable<()>,
    is_continuing: SwitchedObservable<()>,
    app_state_value_changed: Rc<Subject<Option<Rc<T>>>>,
    create_new_app_state_typed: RefCell<Option<TypedAppStateFactory<T>>>,
    app_state: RefCell<Option<Rc<T>>>,
}

impl<T: 'static> fmt::Debug for SuspensionHostOf<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SuspensionHostOf").field("has_app_state_value", &self.app_state.borrow().is_some()).finish()
    }
}

impl<T: 'static> ferroui_system::component_model::INotifyPropertyChanged for SuspensionHostOf<T> {
    fn property_changed(&self) -> &ferroui_system::events::Event<str> {
        self.reactive.property_changed()
    }

    fn as_notify_property_changing(&self) -> Option<&dyn ferroui_system::component_model::INotifyPropertyChanging> {
        Some(self)
    }
}

impl<T: 'static> ferroui_system::component_model::INotifyPropertyChanging for SuspensionHostOf<T> {
    fn property_changing(&self) -> &ferroui_system::events::Event<str> {
        self.reactive.property_changing()
    }
}

impl<T: 'static> ferroui_splat::IEnableLogger for SuspensionHostOf<T> {}

impl<T: 'static> IReactiveObject for SuspensionHostOf<T> {
    fn reactive_object(&self) -> &ReactiveObject {
        &self.reactive
    }
}

impl<T: 'static> crate::IHandleObservableErrors for SuspensionHostOf<T> {
    fn thrown_exceptions(&self) -> Rc<dyn IObservable<ferroui_system::ObservableError>> {
        self.reactive.thrown_exceptions()
    }
}

impl<T: 'static> SuspensionHostOf<T> {
    /// A host whose observables fail on subscription until a helper sets them
    /// (`SuspensionHost<TAppState>()`).
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            reactive: ReactiveObject::new(),
            is_launching_new: SwitchedObservable::new(not_set_up()),
            is_resuming: SwitchedObservable::new(not_set_up()),
            is_unpausing: SwitchedObservable::new(not_set_up()),
            should_persist_state: SwitchedObservable::new(not_set_up()),
            should_invalidate_state: SwitchedObservable::new(not_set_up()),
            is_continuing: SwitchedObservable::new(not_set_up()),
            app_state_value_changed: Subject::new(),
            create_new_app_state_typed: RefCell::new(None),
            app_state: RefCell::new(None),
        })
    }

    /// Ticks when the application continues an activity handed over from another device
    /// (`IsContinuing`).
    pub fn is_continuing(&self) -> Rc<dyn IObservable<()>> {
        self.is_continuing.get()
    }

    /// Sets [`is_continuing`](Self::is_continuing).
    pub fn set_is_continuing(&self, value: Rc<dyn IObservable<()>>) {
        self.is_continuing.set(value);
    }
}

impl<T: 'static> ISuspensionHost for SuspensionHostOf<T> {
    fn is_launching_new(&self) -> Rc<dyn IObservable<()>> {
        self.is_launching_new.get()
    }

    fn set_is_launching_new(&self, value: Rc<dyn IObservable<()>>) {
        self.is_launching_new.set(value);
    }

    fn is_resuming(&self) -> Rc<dyn IObservable<()>> {
        self.is_resuming.get()
    }

    fn set_is_resuming(&self, value: Rc<dyn IObservable<()>>) {
        self.is_resuming.set(value);
    }

    fn is_unpausing(&self) -> Rc<dyn IObservable<()>> {
        self.is_unpausing.get()
    }

    fn set_is_unpausing(&self, value: Rc<dyn IObservable<()>>) {
        self.is_unpausing.set(value);
    }

    fn should_persist_state(&self) -> Rc<dyn IObservable<Rc<dyn IDisposable>>> {
        self.should_persist_state.get()
    }

    fn set_should_persist_state(&self, value: Rc<dyn IObservable<Rc<dyn IDisposable>>>) {
        self.should_persist_state.set(value);
    }

    fn should_invalidate_state(&self) -> Rc<dyn IObservable<()>> {
        self.should_invalidate_state.get()
    }

    fn set_should_invalidate_state(&self, value: Rc<dyn IObservable<()>>) {
        self.should_invalidate_state.set(value);
    }

    /// The typed factory, seen untyped (`ISuspensionHost.CreateNewAppState`).
    fn create_new_app_state(&self) -> Option<AppStateFactory> {
        let typed = self.create_new_app_state_typed.borrow().clone()?;
        Some(Rc::new(move || typed() as Rc<dyn Any>))
    }

    /// Sets the typed factory from an untyped one; the typed factory panics when the untyped one
    /// creates a state of another type (`InvalidCastException`).
    fn set_create_new_app_state(&self, value: Option<AppStateFactory>) {
        let typed = value.map(|factory| {
            let typed: TypedAppStateFactory<T> = Rc::new(move || match factory().downcast::<T>() {
                Ok(state) => state,
                Err(_) => panic!("Created app state is not assignable to {}.", std::any::type_name::<T>()),
            });
            typed
        });
        self.set_create_new_app_state_typed(typed);
    }

    /// The typed state, seen untyped (`ISuspensionHost.AppState`).
    fn app_state(&self) -> Option<Rc<dyn Any>> {
        self.app_state.borrow().clone().map(|state| state as Rc<dyn Any>)
    }

    /// Sets the typed state from an untyped one; panics when it has another type
    /// (`InvalidCastException`).
    fn set_app_state(&self, value: Option<Rc<dyn Any>>) {
        let typed = value.map(|state| match state.downcast::<T>() {
            Ok(state) => state,
            Err(_) => panic!("AppState is not assignable to {}.", std::any::type_name::<T>()),
        });
        self.set_app_state_value(typed);
    }
}

impl<T: 'static> ISuspensionHostOf<T> for SuspensionHostOf<T> {
    fn create_new_app_state_typed(&self) -> Option<TypedAppStateFactory<T>> {
        self.create_new_app_state_typed.borrow().clone()
    }

    /// Sets the factory, raising `CreateNewAppStateTyped` when it is another function
    /// (`RaiseAndSetIfChanged`).
    fn set_create_new_app_state_typed(&self, value: Option<TypedAppStateFactory<T>>) {
        if same_object(&self.create_new_app_state_typed.borrow(), &value) {
            return;
        }
        self.raise_property_changing("CreateNewAppStateTyped");
        *self.create_new_app_state_typed.borrow_mut() = value;
        self.raise_property_changed("CreateNewAppStateTyped");
    }

    fn app_state_value(&self) -> Option<Rc<T>> {
        self.app_state.borrow().clone()
    }

    /// Sets the state, raising `AppStateValue` when it is another object, and ticks
    /// [`app_state_value_changed`](ISuspensionHostOf::app_state_value_changed) in any case.
    fn set_app_state_value(&self, value: Option<Rc<T>>) {
        if !same_object(&self.app_state.borrow(), &value) {
            self.raise_property_changing("AppStateValue");
            *self.app_state.borrow_mut() = value.clone();
            self.raise_property_changed("AppStateValue");
        }
        self.app_state_value_changed.on_next(value);
    }

    fn app_state_value_changed(&self) -> Rc<dyn IObservable<Option<Rc<T>>>> {
        self.app_state_value_changed.as_observable()
    }
}

impl<T: 'static> IDisposable for SuspensionHostOf<T> {
    /// Disposes the subjects of the observables (`Dispose()`).
    fn dispose(&self) {
        self.is_launching_new.dispose();
        self.is_resuming.dispose();
        self.is_unpausing.dispose();
        self.is_continuing.dispose();
        self.should_persist_state.dispose();
        self.should_invalidate_state.dispose();
        self.app_state_value_changed.dispose();
    }
}
