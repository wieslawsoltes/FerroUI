//! `SuspensionHost` (`ReactiveUI.Shared/Suspension/SuspensionHost.cs`).

use crate::{AppStateFactory, IReactiveObject, ISuspensionHost, ReactiveObject};
use ferroui_reactive::subjects::ReplaySubject;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::any::Any;
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

/// The message of the observables of a host that no platform helper set up.
pub(crate) const SUSPENSION_HOST_NOT_SET_UP: &str = "Your App class needs to use AutoSuspendHelper";

/// The error of the observables of a host that no platform helper set up (upstream's
/// `InvalidOperationException`).
#[derive(Debug)]
pub(crate) struct SuspensionHostNotSetUpError;

impl fmt::Display for SuspensionHostNotSetUpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(SUSPENSION_HOST_NOT_SET_UP)
    }
}

impl std::error::Error for SuspensionHostNotSetUpError {}

/// The observable of a host that no platform helper set up: it fails on subscription.
pub(crate) fn not_set_up<T: 'static>() -> Rc<dyn IObservable<T>> {
    Observable::throw(Rc::new(SuspensionHostNotSetUpError) as ObservableError)
}

/// One settable observable of a host: the latest observable set, which a subscriber follows even
/// when another is set later (a `ReplaySignal<IObservable<T>>` of one, read through a switch).
pub(crate) struct SwitchedObservable<T> {
    subject: Rc<ReplaySubject<Rc<dyn IObservable<T>>>>,
}

impl<T: 'static> SwitchedObservable<T> {
    /// The observable, set to `initial`.
    pub(crate) fn new(initial: Rc<dyn IObservable<T>>) -> Self {
        let subject = ReplaySubject::with_buffer_size(1);
        subject.on_next(initial);
        Self { subject }
    }

    /// The values of the latest observable set (`new SwitchSignal<T>(subject)`).
    pub(crate) fn get(&self) -> Rc<dyn IObservable<T>> {
        self.subject.as_observable().switch()
    }

    /// Sets the observable.
    pub(crate) fn set(&self, value: Rc<dyn IObservable<T>>) {
        self.subject.on_next(value);
    }

    /// Disposes the subject.
    pub(crate) fn dispose(&self) {
        self.subject.dispose();
    }
}

/// The state of the suspension of an application, which a platform helper sets up
/// (`SuspensionHost`): the default [`ISuspensionHost`] of [`RxSuspension`](crate::RxSuspension).
///
/// The platform pushes its events into the settable observables; view models subscribe through
/// [`ISuspensionHost`]. Each observable replays the latest one set, so an observable set after a
/// subscription is followed by it. Until a helper sets them, the observables fail on
/// subscription.
///
/// ```
/// use ferroui_reactive::Observable;
/// use ferroui_reactiveui::{ISuspensionHost, SuspensionHost};
/// use std::rc::Rc;
///
/// let host = SuspensionHost::new();
/// host.set_is_launching_new(Observable::return_(()));
/// host.set_create_new_app_state(Some(Rc::new(|| Rc::new(String::from("shell")) as Rc<dyn std::any::Any>)));
/// let factory = host.create_new_app_state().unwrap();
/// host.set_app_state(Some(factory()));
/// assert!(host.app_state().is_some());
/// ```
// Deviation (DEVIATIONS.md, ReactiveUI): internal upstream (its tests see it); public here.
pub struct SuspensionHost {
    reactive: ReactiveObject,
    is_launching_new: SwitchedObservable<()>,
    is_resuming: SwitchedObservable<()>,
    is_unpausing: SwitchedObservable<()>,
    should_persist_state: SwitchedObservable<Rc<dyn IDisposable>>,
    should_invalidate_state: SwitchedObservable<()>,
    create_new_app_state: RefCell<Option<AppStateFactory>>,
    app_state: RefCell<Option<Rc<dyn Any>>>,
}

crate::reactive_object!(SuspensionHost);

impl fmt::Debug for SuspensionHost {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SuspensionHost").field("has_app_state", &self.app_state.borrow().is_some()).finish()
    }
}

impl SuspensionHost {
    /// A host whose observables fail on subscription until a helper sets them
    /// (`SuspensionHost()`).
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            reactive: ReactiveObject::new(),
            is_launching_new: SwitchedObservable::new(not_set_up()),
            is_resuming: SwitchedObservable::new(not_set_up()),
            is_unpausing: SwitchedObservable::new(not_set_up()),
            should_persist_state: SwitchedObservable::new(not_set_up()),
            should_invalidate_state: SwitchedObservable::new(not_set_up()),
            create_new_app_state: RefCell::new(None),
            app_state: RefCell::new(None),
        })
    }
}

impl ISuspensionHost for SuspensionHost {
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

    fn create_new_app_state(&self) -> Option<AppStateFactory> {
        self.create_new_app_state.borrow().clone()
    }

    fn set_create_new_app_state(&self, value: Option<AppStateFactory>) {
        *self.create_new_app_state.borrow_mut() = value;
    }

    fn app_state(&self) -> Option<Rc<dyn Any>> {
        self.app_state.borrow().clone()
    }

    /// Sets the state, raising `AppState` when it is another object (`RaiseAndSetIfChanged`).
    fn set_app_state(&self, value: Option<Rc<dyn Any>>) {
        if same_object(&self.app_state.borrow(), &value) {
            return;
        }
        self.raise_property_changing("AppState");
        *self.app_state.borrow_mut() = value;
        self.raise_property_changed("AppState");
    }
}

impl IDisposable for SuspensionHost {
    /// Disposes the subjects of the observables (`Dispose()`).
    fn dispose(&self) {
        self.is_launching_new.dispose();
        self.is_resuming.dispose();
        self.is_unpausing.dispose();
        self.should_persist_state.dispose();
        self.should_invalidate_state.dispose();
    }
}

/// Whether two optional shared objects are the same object (reference equality).
pub(crate) fn same_object<T: ?Sized>(first: &Option<Rc<T>>, second: &Option<Rc<T>>) -> bool {
    match (first, second) {
        (Some(first), Some(second)) => std::ptr::addr_eq(Rc::as_ptr(first), Rc::as_ptr(second)),
        (None, None) => true,
        _ => false,
    }
}
