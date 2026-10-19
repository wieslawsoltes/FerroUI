//! `SuspensionHostExtensionsTests` (`ReactiveUI.Tests/SuspensionHostExtensionsTests.cs`).
//!
//! Upstream saves and restores the drivers registered in the locator around the tests that
//! unregister them; the locator is per thread here, and each test runs on a thread of its own.

use ferroui_reactive::concurrency::ImmediateScheduler;
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::subjects::Subject;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_reactiveui::*;
use ferroui_splat::AppLocator;
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::Rc;

/// An empty state of the application (`DummyAppState`).
#[derive(Debug, Default)]
struct DummyAppState;

fn dummy_state() -> Rc<dyn Any> {
    Rc::new(DummyAppState)
}

/// The error of a load that fails.
#[derive(Debug)]
struct LoadFailed;

impl fmt::Display for LoadFailed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Failed to load state")
    }
}

impl std::error::Error for LoadFailed {}

/// A driver that counts its calls (`TestSuspensionDriver`).
#[derive(Default)]
struct TestSuspensionDriver {
    invalidate_state_call_count: Cell<i32>,
    last_saved_state: RefCell<Option<Rc<dyn Any>>>,
    load_state_call_count: Cell<i32>,
    save_state_call_count: Cell<i32>,
    should_throw_on_load: Cell<bool>,
    state_to_load: RefCell<Option<Rc<dyn Any>>>,
}

impl TestSuspensionDriver {
    fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    fn loading(state: Rc<dyn Any>) -> Rc<Self> {
        let driver = Self::new();
        *driver.state_to_load.borrow_mut() = Some(state);
        driver
    }

    fn throwing_on_load() -> Rc<Self> {
        let driver = Self::new();
        driver.should_throw_on_load.set(true);
        driver
    }

    fn state_to_load(&self) -> Option<Rc<dyn Any>> {
        self.state_to_load.borrow().clone()
    }
}

impl ISuspensionDriver for TestSuspensionDriver {
    fn load_state(&self) -> Rc<dyn IObservable<Option<Rc<dyn Any>>>> {
        self.load_state_call_count.set(self.load_state_call_count.get() + 1);
        if self.should_throw_on_load.get() {
            return Observable::throw(Rc::new(LoadFailed));
        }
        Observable::return_(Some(self.state_to_load().unwrap_or_else(dummy_state)))
    }

    fn save_state(&self, state: Option<Rc<dyn Any>>) -> Rc<dyn IObservable<()>> {
        self.save_state_call_count.set(self.save_state_call_count.get() + 1);
        *self.last_saved_state.borrow_mut() = state;
        Observable::return_(())
    }

    fn invalidate_state(&self) -> Rc<dyn IObservable<()>> {
        self.invalidate_state_call_count.set(self.invalidate_state_call_count.get() + 1);
        Observable::return_(())
    }
}

fn as_driver(driver: &Rc<TestSuspensionDriver>) -> Option<Rc<dyn ISuspensionDriver>> {
    Some(driver.clone())
}

/// A host whose observables are silent, unless the test sets them.
fn silent_host() -> Rc<SuspensionHost> {
    let host = SuspensionHost::new();
    host.set_is_launching_new(Observable::never());
    host.set_is_resuming(Observable::never());
    host.set_should_persist_state(Observable::never());
    host.set_should_invalidate_state(Observable::never());
    host
}

fn same(first: &Option<Rc<dyn Any>>, second: &Option<Rc<dyn Any>>) -> bool {
    match (first, second) {
        (Some(first), Some(second)) => Rc::ptr_eq(first, second),
        (None, None) => true,
        _ => false,
    }
}

fn erased<T: 'static>(state: &Option<Rc<T>>) -> Option<Rc<dyn Any>> {
    state.clone().map(|state| state as Rc<dyn Any>)
}

#[test]
fn dummy_suspension_driver_invalidate_state_returns_unit_observable() {
    let driver = DummySuspensionDriver::new();
    let was_called = Rc::new(Cell::new(false));
    let seen = was_called.clone();
    let subscription = driver.invalidate_state().observe_on(ImmediateScheduler::instance()).subscribe_next(move |_| seen.set(true));
    assert!(was_called.get());
    subscription.dispose();
}

#[test]
fn dummy_suspension_driver_load_state_returns_default_observable() {
    let driver = DummySuspensionDriver::new();
    let result: Rc<RefCell<Option<Option<Rc<dyn Any>>>>> = Rc::new(RefCell::new(None));
    let seen = result.clone();
    let subscription = driver.load_state().observe_on(ImmediateScheduler::instance()).subscribe_next(move |state| *seen.borrow_mut() = Some(state));
    assert!(result.borrow().as_ref().is_some_and(Option::is_none));
    subscription.dispose();
}

#[test]
fn dummy_suspension_driver_save_state_returns_unit_observable() {
    let driver = DummySuspensionDriver::new();
    let was_called = Rc::new(Cell::new(false));
    let seen = was_called.clone();
    let subscription = driver.save_state(Some(dummy_state())).observe_on(ImmediateScheduler::instance()).subscribe_next(move |_| seen.set(true));
    assert!(was_called.get());
    subscription.dispose();
}

#[test]
fn ensure_load_app_state_locator_drivers_removed_after_setup_loads_through_setup_driver() {
    let host = silent_host();
    host.set_create_new_app_state(Some(Rc::new(dummy_state)));
    let driver = TestSuspensionDriver::loading(dummy_state());

    // Set up with a driver
    let disposable = host.setup_default_suspend_resume_with(as_driver(&driver));
    AppLocator::unregister_all::<dyn ISuspensionDriver>();

    let state = host.get_app_state::<DummyAppState>();
    assert!(same(&erased(&state), &driver.state_to_load()));
    assert_eq!(driver.load_state_call_count.get(), 1);
    disposable.dispose();
}

#[test]
fn ensure_load_app_state_load_state_throws_creates_new_app_state() {
    let host = silent_host();
    host.set_create_new_app_state(Some(Rc::new(dummy_state)));
    let driver = TestSuspensionDriver::throwing_on_load();
    let disposable = host.setup_default_suspend_resume_with(as_driver(&driver));

    let state = host.get_app_state::<DummyAppState>();

    assert!(state.is_some());
    assert_eq!(driver.load_state_call_count.get(), 1);
    disposable.dispose();
}

#[test]
fn ensure_load_app_state_with_existing_app_state_does_not_load() {
    let existing_state = dummy_state();
    let host = silent_host();
    host.set_app_state(Some(existing_state.clone()));
    let driver = TestSuspensionDriver::new();
    let disposable = host.setup_default_suspend_resume_with(as_driver(&driver));

    let state = host.get_app_state::<DummyAppState>();

    assert_eq!(driver.load_state_call_count.get(), 0);
    assert!(same(&erased(&state), &Some(existing_state)));
    disposable.dispose();
}

#[test]
fn ensure_load_app_state_with_null_create_new_app_state_sets_app_state_to_null() {
    let host = silent_host();
    host.set_create_new_app_state(None);
    let driver = TestSuspensionDriver::throwing_on_load();
    let disposable = host.setup_default_suspend_resume_with(as_driver(&driver));

    let state = host.get_app_state::<DummyAppState>();

    assert!(state.is_none());
    disposable.dispose();
}

#[test]
fn get_app_state_only_loads_once() {
    let host = silent_host();
    host.set_create_new_app_state(Some(Rc::new(dummy_state)));
    let driver = TestSuspensionDriver::loading(dummy_state());
    let disposable = host.setup_default_suspend_resume_with(as_driver(&driver));

    let state1 = host.get_app_state::<DummyAppState>();
    let state2 = host.get_app_state::<DummyAppState>();

    assert_eq!(driver.load_state_call_count.get(), 1);
    assert!(same(&erased(&state1), &erased(&state2)));
    disposable.dispose();
}

#[test]
fn get_app_state_triggers_load_on_first_call() {
    let host = silent_host();
    host.set_create_new_app_state(Some(Rc::new(dummy_state)));
    let driver = TestSuspensionDriver::loading(dummy_state());
    let disposable = host.setup_default_suspend_resume_with(as_driver(&driver));

    let state = host.get_app_state::<DummyAppState>();

    assert!(state.is_some());
    assert_eq!(driver.load_state_call_count.get(), 1);
    disposable.dispose();
}

#[test]
fn get_app_state_returns() {
    let fixture = SuspensionHost::new();
    fixture.set_app_state(Some(dummy_state()));

    let result = fixture.get_app_state::<DummyAppState>();

    assert!(same(&erased(&result), &fixture.app_state()));
}

// Not ported: GetAppStateThrowsForNullHost - argument validation

#[test]
fn null_app_state_does_not_throw_exception() {
    let fixture = SuspensionHost::new();
    let disposable = fixture.setup_default_suspend_resume();
    disposable.dispose();
}

// Not ported: NullSuspensionHostThrowsException - argument validation

#[test]
fn observe_app_state_does_not_throw_exception() {
    let fixture = SuspensionHost::new();
    let subscription = fixture.observe_app_state::<DummyAppState>().subscribe_ignore();
    subscription.dispose();
}

#[test]
fn observe_app_state_emits_values() {
    let fixture = SuspensionHost::new();
    let received_states = Rc::new(RefCell::new(Vec::new()));
    let log = received_states.clone();
    let subscription = fixture.observe_app_state::<DummyAppState>().observe_on(ImmediateScheduler::instance()).subscribe_next(move |state| log.borrow_mut().push(state));

    let state1 = Rc::new(DummyAppState);
    fixture.set_app_state(Some(state1.clone()));
    let state2 = Rc::new(DummyAppState);
    fixture.set_app_state(Some(state2.clone()));

    let received_states = received_states.borrow();
    assert_eq!(received_states.len(), 2);
    assert!(Rc::ptr_eq(&received_states[0], &state1));
    assert!(Rc::ptr_eq(&received_states[1], &state2));
    subscription.dispose();
    fixture.dispose();
}

#[test]
fn observe_app_state_filters_null_values() {
    let fixture = SuspensionHost::new();
    let received_states = Rc::new(RefCell::new(Vec::new()));
    let log = received_states.clone();
    let subscription = fixture.observe_app_state::<DummyAppState>().observe_on(ImmediateScheduler::instance()).subscribe_next(move |state| log.borrow_mut().push(state));

    fixture.set_app_state(None);
    let state = Rc::new(DummyAppState);
    fixture.set_app_state(Some(state.clone()));
    fixture.set_app_state(None);

    let received_states = received_states.borrow();
    assert_eq!(received_states.len(), 1);
    assert!(Rc::ptr_eq(&received_states[0], &state));
    subscription.dispose();
    fixture.dispose();
}

// Not ported: ObserveAppStateThrowsForNullHost - argument validation

#[test]
fn setup_default_suspend_resume_is_resuming_or_is_launching_new_triggers_state_load() {
    let host = SuspensionHost::new();
    host.set_create_new_app_state(Some(Rc::new(dummy_state)));
    host.set_should_persist_state(Observable::never());
    host.set_should_invalidate_state(Observable::never());
    let driver = TestSuspensionDriver::new();
    let launch_subject = Subject::<()>::new();
    let resume_subject = Subject::<()>::new();
    host.set_is_launching_new(launch_subject.as_observable().observe_on(ImmediateScheduler::instance()));
    host.set_is_resuming(resume_subject.as_observable().observe_on(ImmediateScheduler::instance()));
    let disposable = host.setup_default_suspend_resume_with(as_driver(&driver));

    launch_subject.on_next(());

    assert!(host.app_state().is_some());
    disposable.dispose();
}

#[test]
fn setup_default_suspend_resume_should_invalidate_state_calls_driver_invalidate_state() {
    let host = SuspensionHost::new();
    host.set_is_launching_new(Observable::never());
    host.set_is_resuming(Observable::never());
    host.set_should_persist_state(Observable::never());
    let driver = TestSuspensionDriver::new();
    let invalidate_subject = Subject::<()>::new();
    host.set_should_invalidate_state(invalidate_subject.as_observable().observe_on(ImmediateScheduler::instance()));
    let disposable = host.setup_default_suspend_resume_with(as_driver(&driver));

    invalidate_subject.on_next(());

    assert_eq!(driver.invalidate_state_call_count.get(), 1);
    disposable.dispose();
}

#[test]
fn setup_default_suspend_resume_should_persist_state_calls_driver_save_state() {
    let app_state = dummy_state();
    let host = SuspensionHost::new();
    host.set_app_state(Some(app_state.clone()));
    host.set_is_launching_new(Observable::never());
    host.set_is_resuming(Observable::never());
    host.set_should_invalidate_state(Observable::never());
    let driver = TestSuspensionDriver::new();
    let persist_subject = Subject::<Rc<dyn IDisposable>>::new();
    host.set_should_persist_state(persist_subject.as_observable().observe_on(ImmediateScheduler::instance()));
    let disposable = host.setup_default_suspend_resume_with(as_driver(&driver));

    persist_subject.on_next(Disposable::empty());

    assert_eq!(driver.save_state_call_count.get(), 1);
    assert!(same(&driver.last_saved_state.borrow(), &Some(app_state)));
    disposable.dispose();
}

#[test]
fn setup_default_suspend_resume_with_null_driver_logs_error() {
    let host = silent_host();
    AppLocator::unregister_all::<dyn ISuspensionDriver>();

    // The setup without a driver logs an error and returns an empty disposable.
    let disposable = host.setup_default_suspend_resume();
    disposable.dispose();
}

#[test]
fn setup_default_suspend_resume_with_provided_driver_uses_provided_driver() {
    let host = silent_host();
    host.set_create_new_app_state(Some(Rc::new(dummy_state)));
    let driver = TestSuspensionDriver::loading(dummy_state());

    let disposable = host.setup_default_suspend_resume_with(as_driver(&driver));

    disposable.dispose();
}

#[test]
fn setup_default_suspend_resume_two_hosts_with_different_drivers_each_host_uses_its_own_driver() {
    let first_persist = Subject::<Rc<dyn IDisposable>>::new();
    let first_host = SuspensionHost::new();
    first_host.set_is_launching_new(Observable::never());
    first_host.set_is_resuming(Observable::never());
    first_host.set_should_invalidate_state(Observable::never());
    first_host.set_should_persist_state(first_persist.as_observable().observe_on(ImmediateScheduler::instance()));
    let second_persist = Subject::<Rc<dyn IDisposable>>::new();
    let second_host = SuspensionHost::new();
    second_host.set_is_launching_new(Observable::never());
    second_host.set_is_resuming(Observable::never());
    second_host.set_should_invalidate_state(Observable::never());
    second_host.set_should_persist_state(second_persist.as_observable().observe_on(ImmediateScheduler::instance()));
    let first_driver = TestSuspensionDriver::loading(dummy_state());
    let second_driver = TestSuspensionDriver::loading(dummy_state());
    let first_setup = first_host.setup_default_suspend_resume_with(as_driver(&first_driver));
    let second_setup = second_host.setup_default_suspend_resume_with(as_driver(&second_driver));

    let first_state = first_host.get_app_state::<DummyAppState>();
    let second_state = second_host.get_app_state::<DummyAppState>();
    first_persist.on_next(Disposable::empty());
    second_persist.on_next(Disposable::empty());

    assert!(same(&erased(&first_state), &first_driver.state_to_load()));
    assert!(same(&erased(&second_state), &second_driver.state_to_load()));
    assert_eq!(first_driver.load_state_call_count.get(), 1);
    assert_eq!(second_driver.load_state_call_count.get(), 1);
    assert_eq!(first_driver.save_state_call_count.get(), 1);
    assert_eq!(second_driver.save_state_call_count.get(), 1);
    assert!(same(&first_driver.last_saved_state.borrow(), &first_driver.state_to_load()));
    assert!(same(&second_driver.last_saved_state.borrow(), &second_driver.state_to_load()));
    first_setup.dispose();
    second_setup.dispose();
}
