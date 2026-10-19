//! `SuspensionHostExtensions` (`ReactiveUI.Shared/Suspension/SuspensionHostExtensions.cs`).

use crate::{ISuspensionDriver, ISuspensionDriverOf, ISuspensionHost, ISuspensionHostOf};
use ferroui_reactive::disposables::{CompositeDisposable, Disposable};
use ferroui_reactive::Observer;
use ferroui_splat::{AppLocator, IFullLogger, ILogManager};
use ferroui_system::threading::tasks::block_on;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::rc::{Rc, Weak};
use std::task::{Context, Poll, Waker};

/// A load of the state armed by the setup of a host, run once (`PendingLoad`).
#[derive(Default)]
struct PendingLoad {
    load: RefCell<Option<Rc<dyn Fn()>>>,
}

impl PendingLoad {
    /// Arms the load, replacing one armed before (`Arm`).
    fn arm(&self, load: Rc<dyn Fn()>) {
        *self.load.borrow_mut() = Some(load);
    }

    /// Runs the armed load, once (`Run`).
    fn run(&self) {
        let load = self.load.borrow_mut().take();
        if let Some(load) = load {
            load();
        }
    }
}

/// The pending load of a host, with the test that the host is alive.
struct PendingEntry {
    alive: Box<dyn Fn() -> bool>,
    load: Rc<PendingLoad>,
}

ferroui_system::ferro_thread_local! {
    /// The pending loads, by the address of their host (`_pendingLoads`, a
    /// `ConditionalWeakTable<ISuspensionHost, PendingLoad>`).
    static PENDING_LOADS: RefCell<HashMap<usize, PendingEntry>> = RefCell::new(HashMap::new());
}

/// The address of a host, which identifies it.
fn address_of<H: ?Sized>(host: &Rc<H>) -> usize {
    Rc::as_ptr(host) as *const () as usize
}

/// The pending load of `host`, created the first time (`_pendingLoads.GetValue`); the entries of
/// dropped hosts are removed.
fn pending_load_of<H: ?Sized + 'static>(host: &Rc<H>) -> Rc<PendingLoad> {
    let key = address_of(host);
    let weak: Weak<H> = Rc::downgrade(host);
    PENDING_LOADS.with(|loads| {
        let mut loads = loads.borrow_mut();
        loads.retain(|_, entry| (entry.alive)());
        loads
            .entry(key)
            .or_insert_with(|| PendingEntry { alive: Box::new(move || weak.strong_count() > 0), load: Rc::new(PendingLoad::default()) })
            .load
            .clone()
    })
}

/// Runs the pending load of `host`, if its setup armed one (`RunPendingLoad`).
fn run_pending_load<H: ?Sized>(host: &Rc<H>) {
    let key = address_of(host);
    let load = PENDING_LOADS.with(|loads| loads.borrow().get(&key).filter(|entry| (entry.alive)()).map(|entry| entry.load.clone()));
    if let Some(load) = load {
        load.run();
    }
}

/// The logger of the type of a host, when the locator has a log manager (`item.Log()`).
fn logger_of(type_name: &'static str) -> Option<Rc<dyn IFullLogger>> {
    AppLocator::get_service::<dyn ILogManager>().map(|log_manager| log_manager.get_logger(type_name))
}

/// The state of the application of a suspension host, and the default handling of its suspension
/// (the extension members of `ISuspensionHost` in `SuspensionHostExtensions`), on its shared
/// handle.
pub trait SuspensionHostExtensions {
    /// The state of the application as a `T`, loaded first when the setup armed a load and it has
    /// not run yet (`GetAppState<T>()`); `None` when there is no state.
    ///
    /// Panics when the state is of another type (`InvalidCastException`).
    fn get_app_state<T: 'static>(&self) -> Option<Rc<T>>;

    /// The state of the application as a `T`: the current one on subscription, then each one set;
    /// a state that is `None` or of another type is skipped (`ObserveAppState<T>()`).
    // Deviation (DEVIATIONS.md, ReactiveUI): upstream observes `AppState` with
    // `WhenAnyUnsafe(x => x.AppState)`, by reflection; here through the host's `PropertyChanged`.
    fn observe_app_state<T: 'static>(&self) -> Rc<dyn IObservable<Rc<T>>>;

    /// Sets up the default suspension and resumption with the driver registered in `AppLocator`
    /// (`SetupDefaultSuspendResume()`); see
    /// [`setup_default_suspend_resume_with`](Self::setup_default_suspend_resume_with).
    fn setup_default_suspend_resume(&self) -> Rc<dyn IDisposable>;

    /// Sets up the default suspension and resumption with `driver`, or the one registered in
    /// `AppLocator` as `dyn ISuspensionDriver` when `None` (`SetupDefaultSuspendResume(ISuspensionDriver?)`):
    /// the state is loaded once, when the application launches or resumes, or before it is first
    /// read or saved, and created anew when there is none or the load fails; it is saved when the
    /// host says it should persist, and invalidated when it says so. Without a driver, an error is
    /// logged and nothing is set up.
    fn setup_default_suspend_resume_with(&self, driver: Option<Rc<dyn ISuspensionDriver>>) -> Rc<dyn IDisposable>;
}

impl<H: ISuspensionHost + ?Sized + 'static> SuspensionHostExtensions for Rc<H> {
    fn get_app_state<T: 'static>(&self) -> Option<Rc<T>> {
        run_pending_load(self);
        let state = self.app_state()?;
        match state.downcast::<T>() {
            Ok(state) => Some(state),
            Err(_) => panic!("The state of the application is not a {}", std::any::type_name::<T>()),
        }
    }

    fn observe_app_state<T: 'static>(&self) -> Rc<dyn IObservable<Rc<T>>> {
        Rc::new(AppStateObservable::<H, T> { host: Rc::downgrade(self), _state: std::marker::PhantomData })
    }

    fn setup_default_suspend_resume(&self) -> Rc<dyn IDisposable> {
        self.setup_default_suspend_resume_with(None)
    }

    fn setup_default_suspend_resume_with(&self, driver: Option<Rc<dyn ISuspensionDriver>>) -> Rc<dyn IDisposable> {
        let type_name = std::any::type_name::<H>();
        let Some(driver) = driver.or_else(AppLocator::get_service::<dyn ISuspensionDriver>) else {
            if let Some(logger) = logger_of(type_name) {
                logger.error("Could not find a valid driver and therefore cannot setup Suspend/Resume.");
            }
            return Disposable::empty();
        };
        let pending_load = pending_load_of(self);
        let host = Rc::downgrade(self);
        let load_driver = driver.clone();
        pending_load.arm(Rc::new(move || {
            if let Some(host) = host.upgrade() {
                ensure_load_app_state(&*host, &*load_driver, type_name);
            }
        }));
        let invalidate_driver = driver.clone();
        let persist_driver = driver;
        let persist_load = pending_load.clone();
        let host = Rc::downgrade(self);
        setup_handlers(
            &**self,
            type_name,
            pending_load,
            move || invalidate_driver.invalidate_state(),
            move || {
                // Materialize the state (the one-time load) before saving, so that a shutdown that
                // races ahead of the load on resume or launch persists the real state.
                persist_load.run();
                let state = host.upgrade().and_then(|host| host.app_state());
                persist_driver.save_state(state)
            },
        )
    }
}

/// The typed state of a typed suspension host, and the default handling of its suspension with a
/// typed driver (the extension members of `ISuspensionHost<TAppState>` in
/// `SuspensionHostExtensions`), on its shared handle.
// Deviation (DEVIATIONS.md, ReactiveUI): upstream names these as the untyped members
// (`GetAppState()`, `ObserveAppState()`, `SetupDefaultSuspendResume(JsonTypeInfo<TAppState>)`),
// told apart by their type arguments; here they end in `_typed`, and the `JsonTypeInfo` is the
// type of the driver ([`ISuspensionDriverOf<T>`]).
pub trait SuspensionHostOfExtensions<T: 'static> {
    /// The state, loaded first when the setup armed a load and it has not run yet
    /// (`GetAppState()`).
    fn get_app_state_typed(&self) -> Option<Rc<T>>;

    /// The state: the current one on subscription when there is one, then each one set; `None`
    /// is skipped (`ObserveAppState()`).
    fn observe_app_state_typed(&self) -> Rc<dyn IObservable<Rc<T>>>;

    /// Sets up the default suspension and resumption with the typed driver registered in
    /// `AppLocator` as `dyn ISuspensionDriverOf<T>` (`SetupDefaultSuspendResume(JsonTypeInfo<TAppState>)`).
    fn setup_default_suspend_resume_typed(&self) -> Rc<dyn IDisposable>;

    /// Sets up the default suspension and resumption with `driver`, or the registered one when
    /// `None` (`SetupDefaultSuspendResume(JsonTypeInfo<TAppState>, ISuspensionDriver?)`); see
    /// [`SuspensionHostExtensions::setup_default_suspend_resume_with`].
    fn setup_default_suspend_resume_typed_with(&self, driver: Option<Rc<dyn ISuspensionDriverOf<T>>>) -> Rc<dyn IDisposable>;
}

impl<T: 'static, H: ISuspensionHostOf<T> + ?Sized + 'static> SuspensionHostOfExtensions<T> for Rc<H> {
    fn get_app_state_typed(&self) -> Option<Rc<T>> {
        run_pending_load(self);
        self.app_state_value()
    }

    fn observe_app_state_typed(&self) -> Rc<dyn IObservable<Rc<T>>> {
        Rc::new(AppStateValueObservable::<H, T> { host: Rc::downgrade(self), _state: std::marker::PhantomData })
    }

    fn setup_default_suspend_resume_typed(&self) -> Rc<dyn IDisposable> {
        self.setup_default_suspend_resume_typed_with(None)
    }

    fn setup_default_suspend_resume_typed_with(&self, driver: Option<Rc<dyn ISuspensionDriverOf<T>>>) -> Rc<dyn IDisposable> {
        let type_name = std::any::type_name::<H>();
        let Some(driver) = driver.or_else(AppLocator::get_service::<dyn ISuspensionDriverOf<T>>) else {
            if let Some(logger) = logger_of(type_name) {
                logger.error("Could not find a valid driver and therefore cannot setup Suspend/Resume.");
            }
            return Disposable::empty();
        };
        let pending_load = pending_load_of(self);
        let host = Rc::downgrade(self);
        let load_driver = driver.clone();
        pending_load.arm(Rc::new(move || {
            if let Some(host) = host.upgrade() {
                ensure_load_app_state_typed(&*host, &*load_driver, type_name);
            }
        }));
        let invalidate_driver = driver.clone();
        let persist_driver = driver;
        let persist_load = pending_load.clone();
        let host = Rc::downgrade(self);
        setup_handlers(
            &**self,
            type_name,
            pending_load,
            move || invalidate_driver.invalidate_state(),
            move || {
                persist_load.run();
                let state = host.upgrade().and_then(|host| host.app_state_value());
                persist_driver.save_state_of(state)
            },
        )
    }
}

/// Subscribes the handlers of the default suspension and resumption to the observables of `host`.
fn setup_handlers<H: ISuspensionHost + ?Sized>(
    host: &H,
    type_name: &'static str,
    pending_load: Rc<PendingLoad>,
    invalidate: impl Fn() -> Rc<dyn IObservable<()>> + 'static,
    persist: impl Fn() -> Rc<dyn IObservable<()>> + 'static,
) -> Rc<dyn IDisposable> {
    let ret = CompositeDisposable::new();
    ret.add(host.should_invalidate_state().subscribe(Rc::new(DriverOperationObserver {
        type_name,
        operation: Box::new(move |_: ()| invalidate()),
        on_finally: Rc::new(|_: ()| {}),
        success_message: "Invalidated app state",
        error_message: "Tried to invalidate app state",
    })));
    ret.add(host.should_persist_state().subscribe(Rc::new(DriverOperationObserver {
        type_name,
        operation: Box::new(move |_: Rc<dyn IDisposable>| persist()),
        on_finally: Rc::new(|token: Rc<dyn IDisposable>| token.dispose()),
        success_message: "Persisted application state",
        error_message: "Tried to persist app state",
    })));
    let resume_load = pending_load.clone();
    ret.add(host.is_resuming().subscribe(Observer::create_all(move |_: ()| resume_load.run(), |_| {}, || {})));
    ret.add(host.is_launching_new().subscribe(Observer::create_all(move |_: ()| pending_load.run(), |_| {}, || {})));
    ret
}

/// Loads the state when the host has none: the persisted one, or a new one when there is none or
/// the load fails (`EnsureLoadAppState`).
fn ensure_load_app_state<H: ISuspensionHost + ?Sized>(item: &H, driver: &dyn ISuspensionDriver, type_name: &'static str) {
    if item.app_state().is_some() {
        return;
    }
    match wait_for_result(driver.load_state()) {
        // Fall back to a new state when the driver has none persisted.
        Ok(state) => item.set_app_state(state.flatten().or_else(|| item.create_new_app_state().map(|create| create()))),
        Err(error) => {
            if let Some(logger) = logger_of(type_name) {
                logger.warn_exception(&*error, "Failed to restore app state from storage, creating from scratch");
            }
            item.set_app_state(item.create_new_app_state().map(|create| create()));
        }
    }
}

/// [`ensure_load_app_state`] for a typed host and driver (`EnsureLoadAppState<TAppState>`).
fn ensure_load_app_state_typed<T: 'static, H: ISuspensionHostOf<T> + ?Sized>(item: &H, driver: &dyn ISuspensionDriverOf<T>, type_name: &'static str) {
    if item.app_state_value().is_some() {
        return;
    }
    match wait_for_result(driver.load_state_of()) {
        Ok(state) => item.set_app_state_value(state.flatten().or_else(|| item.create_new_app_state_typed().map(|create| create()))),
        Err(error) => {
            if let Some(logger) = logger_of(type_name) {
                logger.warn_exception(&*error, "Failed to restore app state from storage, creating from scratch");
            }
            item.set_app_state_value(item.create_new_app_state_typed().map(|create| create()));
        }
    }
}

/// What a source of one result has delivered.
struct WaitState<T> {
    value: RefCell<Option<T>>,
    error: RefCell<Option<ObservableError>>,
    done: Cell<bool>,
    waker: RefCell<Option<Waker>>,
}

impl<T> WaitState<T> {
    fn finish(&self) {
        self.done.set(true);
        let waker = self.waker.borrow_mut().take();
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

/// Records the last value of the source and its end (`BlockingObserver<T>`).
struct BlockingObserver<T> {
    state: Rc<WaitState<T>>,
}

impl<T> IObserver<T> for BlockingObserver<T> {
    fn on_next(&self, value: T) {
        *self.state.value.borrow_mut() = Some(value);
    }

    fn on_error(&self, error: ObservableError) {
        *self.state.error.borrow_mut() = Some(error);
        self.state.finish();
    }

    fn on_completed(&self) {
        self.state.finish();
    }
}

/// Completes when the source ends.
struct WaitFuture<T> {
    state: Rc<WaitState<T>>,
}

impl<T> Future for WaitFuture<T> {
    type Output = Result<(), ObservableError>;

    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        if self.state.done.get() {
            return Poll::Ready(Ok(()));
        }
        *self.state.waker.borrow_mut() = Some(context.waker().clone());
        Poll::Pending
    }
}

/// The last value of `source` once it ends, or its error (`WaitForResult<T>`).
// Deviation (DEVIATIONS.md, ReactiveUI): upstream blocks the thread until another thread ends the
// load; on one thread, a load that does not end at once runs the tasks of the thread
// (`block_on`) until it ends.
fn wait_for_result<T: 'static>(source: Rc<dyn IObservable<T>>) -> Result<Option<T>, ObservableError> {
    let state = Rc::new(WaitState { value: RefCell::new(None), error: RefCell::new(None), done: Cell::new(false), waker: RefCell::new(None) });
    let subscription = source.subscribe(Rc::new(BlockingObserver { state: state.clone() }));
    if !state.done.get() {
        block_on(WaitFuture { state: state.clone() })?;
    }
    subscription.dispose();
    if let Some(error) = state.error.borrow_mut().take() {
        return Err(error);
    }
    let value = state.value.borrow_mut().take();
    Ok(value)
}

/// Runs a driver operation for each value of an observable of the host, logs its outcome, and
/// runs `on_finally` with the value once the operation ends (`DriverOperationObserver<TArg>`).
struct DriverOperationObserver<A> {
    type_name: &'static str,
    operation: Box<dyn Fn(A) -> Rc<dyn IObservable<()>>>,
    on_finally: Rc<dyn Fn(A)>,
    success_message: &'static str,
    error_message: &'static str,
}

impl<A: Clone + 'static> IObserver<A> for DriverOperationObserver<A> {
    fn on_next(&self, value: A) {
        let operation = (self.operation)(value.clone());
        let _ = operation.subscribe(Rc::new(DriverResultObserver {
            type_name: self.type_name,
            argument: RefCell::new(Some(value)),
            on_finally: self.on_finally.clone(),
            success_message: self.success_message,
            error_message: self.error_message,
        }));
    }

    fn on_error(&self, error: ObservableError) {
        if let Some(logger) = logger_of(self.type_name) {
            logger.warn_exception(&*error, self.error_message);
        }
    }

    fn on_completed(&self) {}
}

/// Logs the outcome of one driver operation and runs `on_finally` once it ends
/// (`DriverOperationObserver<TArg>.ResultObserver`).
struct DriverResultObserver<A> {
    type_name: &'static str,
    /// The value of the operation, until the operation ends (`_finished`).
    argument: RefCell<Option<A>>,
    on_finally: Rc<dyn Fn(A)>,
    success_message: &'static str,
    error_message: &'static str,
}

impl<A> DriverResultObserver<A> {
    fn finish(&self) {
        let argument = self.argument.borrow_mut().take();
        if let Some(argument) = argument {
            (self.on_finally)(argument);
        }
    }
}

impl<A> IObserver<()> for DriverResultObserver<A> {
    fn on_next(&self, _value: ()) {
        if let Some(logger) = logger_of(self.type_name) {
            logger.info(self.success_message);
        }
    }

    fn on_error(&self, error: ObservableError) {
        if let Some(logger) = logger_of(self.type_name) {
            logger.warn_exception(&*error, self.error_message);
        }
        self.finish();
    }

    fn on_completed(&self) {
        self.finish();
    }
}

/// The state of an untyped host as a `T`, now and after each change of `AppState`
/// (`NonNullCastObservable<TResult>` over `WhenAny(x => x.AppState)`).
struct AppStateObservable<H: ?Sized, T> {
    host: Weak<H>,
    _state: std::marker::PhantomData<fn() -> T>,
}

impl<H: ISuspensionHost + ?Sized + 'static, T: 'static> IObservable<Rc<T>> for AppStateObservable<H, T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<Rc<T>>>) -> Rc<dyn IDisposable> {
        let Some(host) = self.host.upgrade() else {
            return Disposable::empty();
        };
        let emit = {
            let observer = observer.clone();
            move |state: Option<Rc<dyn Any>>| {
                if let Some(state) = state.and_then(|state| state.downcast::<T>().ok()) {
                    observer.on_next(state);
                }
            }
        };
        emit(host.app_state());
        let weak = self.host.clone();
        let token = host.property_changed().add(Rc::new(move |name: &str| {
            if name != "AppState" {
                return;
            }
            if let Some(host) = weak.upgrade() {
                emit(host.app_state());
            }
        }));
        let weak = self.host.clone();
        Disposable::create(move || {
            if let Some(host) = weak.upgrade() {
                host.property_changed().remove(token);
            }
        })
    }
}

/// The state of a typed host when there is one, then each state set that is not `None`
/// (`AppStateValueObservable<TAppState>`).
struct AppStateValueObservable<H: ?Sized, T> {
    host: Weak<H>,
    _state: std::marker::PhantomData<fn() -> T>,
}

impl<T: 'static, H: ISuspensionHostOf<T> + ?Sized + 'static> IObservable<Rc<T>> for AppStateValueObservable<H, T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<Rc<T>>>) -> Rc<dyn IDisposable> {
        let Some(host) = self.host.upgrade() else {
            return Disposable::empty();
        };
        if let Some(current) = host.app_state_value() {
            observer.on_next(current);
        }
        host.app_state_value_changed().subscribe(Observer::create_all(
            {
                let observer = observer.clone();
                move |state: Option<Rc<T>>| {
                    if let Some(state) = state {
                        observer.on_next(state);
                    }
                }
            },
            {
                let observer = observer.clone();
                move |error| observer.on_error(error)
            },
            move || observer.on_completed(),
        ))
    }
}
