//! `SuspensionHostExtensionsAotTests` (`ReactiveUI.Tests/Suspension/SuspensionHostExtensionsAotTests.cs`):
//! the typed suspension host with a typed driver.
//!
//! Upstream passes the serialization metadata of the state (`JsonTypeInfo<TestAppState>`); in the
//! port the type of the state is the type of the driver ([`ISuspensionDriverOf<T>`]).

use ferroui_reactive::concurrency::ImmediateScheduler;
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::subjects::Subject;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_reactiveui::*;
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::Rc;

pub(super) const SAMPLE_STATE_VALUE: i32 = 42;
pub(super) const FACTORY_STATE_VALUE: i32 = 99;
pub(super) const LOADED_STATE_VALUE: i32 = 123;
pub(super) const CREATED_STATE_VALUE: i32 = 999;
const SECOND_STATE_VALUE: i32 = 2;
const EXPECTED_EMISSION_COUNT: usize = 2;

/// The state of the tests (`TestAppState`).
#[derive(Debug, Default)]
pub(super) struct TestAppState {
    pub(super) value: i32,
}

pub(super) fn test_state(value: i32) -> Rc<TestAppState> {
    Rc::new(TestAppState { value })
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

/// A typed driver that counts its calls (`TestSuspensionDriver<T>`).
pub(super) struct TestSuspensionDriver<T> {
    pub(super) invalidate_state_call_count: Cell<i32>,
    pub(super) last_saved_state: RefCell<Option<Rc<T>>>,
    pub(super) load_state_call_count: Cell<i32>,
    pub(super) save_state_call_count: Cell<i32>,
    pub(super) should_throw_on_load: Cell<bool>,
    pub(super) state_to_load: RefCell<Option<Rc<T>>>,
}

impl<T: 'static> TestSuspensionDriver<T> {
    pub(super) fn new() -> Rc<Self> {
        Rc::new(Self {
            invalidate_state_call_count: Cell::new(0),
            last_saved_state: RefCell::new(None),
            load_state_call_count: Cell::new(0),
            save_state_call_count: Cell::new(0),
            should_throw_on_load: Cell::new(false),
            state_to_load: RefCell::new(None),
        })
    }

    pub(super) fn loading(state: Rc<T>) -> Rc<Self> {
        let driver = Self::new();
        *driver.state_to_load.borrow_mut() = Some(state);
        driver
    }

    pub(super) fn throwing_on_load() -> Rc<Self> {
        let driver = Self::new();
        driver.should_throw_on_load.set(true);
        driver
    }

    pub(super) fn state_to_load(&self) -> Option<Rc<T>> {
        self.state_to_load.borrow().clone()
    }

    pub(super) fn last_saved_state(&self) -> Option<Rc<T>> {
        self.last_saved_state.borrow().clone()
    }

    pub(super) fn as_driver(self: &Rc<Self>) -> Option<Rc<dyn ISuspensionDriverOf<T>>> {
        Some(self.clone())
    }
}

impl<T: 'static> ISuspensionDriver for TestSuspensionDriver<T> {
    fn load_state(&self) -> Rc<dyn IObservable<Option<Rc<dyn Any>>>> {
        self.load_state_call_count.set(self.load_state_call_count.get() + 1);
        if self.should_throw_on_load.get() {
            return Observable::throw(Rc::new(LoadFailed));
        }
        Observable::return_(self.state_to_load().map(|state| state as Rc<dyn Any>))
    }

    fn save_state(&self, state: Option<Rc<dyn Any>>) -> Rc<dyn IObservable<()>> {
        self.save_state_call_count.set(self.save_state_call_count.get() + 1);
        if let Some(state) = state.and_then(|state| state.downcast::<T>().ok()) {
            *self.last_saved_state.borrow_mut() = Some(state);
        }
        Observable::return_(())
    }

    fn invalidate_state(&self) -> Rc<dyn IObservable<()>> {
        self.invalidate_state_call_count.set(self.invalidate_state_call_count.get() + 1);
        Observable::return_(())
    }
}

impl<T: 'static> ISuspensionDriverOf<T> for TestSuspensionDriver<T> {
    fn load_state_of(&self) -> Rc<dyn IObservable<Option<Rc<T>>>> {
        self.load_state_call_count.set(self.load_state_call_count.get() + 1);
        if self.should_throw_on_load.get() {
            return Observable::throw(Rc::new(LoadFailed));
        }
        Observable::return_(self.state_to_load())
    }

    fn save_state_of(&self, state: Option<Rc<T>>) -> Rc<dyn IObservable<()>> {
        self.save_state_call_count.set(self.save_state_call_count.get() + 1);
        if let Some(state) = state {
            *self.last_saved_state.borrow_mut() = Some(state);
        }
        Observable::return_(())
    }
}

/// A typed host whose observables are silent, unless the test sets them.
pub(super) fn silent_host() -> Rc<SuspensionHostOf<TestAppState>> {
    let host = SuspensionHostOf::<TestAppState>::new();
    host.set_is_launching_new(Observable::never());
    host.set_is_resuming(Observable::never());
    host.set_should_persist_state(Observable::never());
    host.set_should_invalidate_state(Observable::never());
    host
}

pub(super) fn same<T>(first: &Option<Rc<T>>, second: &Option<Rc<T>>) -> bool {
    match (first, second) {
        (Some(first), Some(second)) => Rc::ptr_eq(first, second),
        (None, None) => true,
        _ => false,
    }
}

#[test]
fn get_app_state_typed_returns_current_state() {
    let state = test_state(SAMPLE_STATE_VALUE);
    let host = silent_host();
    host.set_app_state_value(Some(state.clone()));

    let result = host.get_app_state_typed();

    assert!(same(&result, &Some(state)));
    host.dispose();
}

// Not ported: GetAppState_Typed_ThrowsForNullHost - argument validation

#[test]
fn get_app_state_typed_triggers_ensure_load_app_state() {
    let host = silent_host();
    host.set_create_new_app_state_typed(Some(Rc::new(|| test_state(FACTORY_STATE_VALUE))));
    let driver = TestSuspensionDriver::loading(test_state(LOADED_STATE_VALUE));
    let disposable = host.setup_default_suspend_resume_typed_with(driver.as_driver());

    let state = host.get_app_state_typed();

    assert_eq!(state.map(|state| state.value), Some(LOADED_STATE_VALUE));
    assert_eq!(driver.load_state_call_count.get(), 1);
    disposable.dispose();
    host.dispose();
}

#[test]
fn get_app_state_typed_when_no_persisted_state_creates_and_stores_new_app_state() {
    let created_state = test_state(FACTORY_STATE_VALUE);
    let create_new_app_state_call_count = Rc::new(Cell::new(0));
    let host = silent_host();
    let (created, count) = (created_state.clone(), create_new_app_state_call_count.clone());
    host.set_create_new_app_state_typed(Some(Rc::new(move || {
        count.set(count.get() + 1);
        created.clone()
    })));
    let driver = TestSuspensionDriver::<TestAppState>::new();
    let disposable = host.setup_default_suspend_resume_typed_with(driver.as_driver());

    let state = host.get_app_state_typed();

    assert!(same(&state, &Some(created_state.clone())));
    assert!(same(&host.app_state_value(), &Some(created_state)));
    assert_eq!(create_new_app_state_call_count.get(), 1);
    assert_eq!(driver.load_state_call_count.get(), 1);
    disposable.dispose();
    host.dispose();
}

#[test]
fn observe_app_state_typed_emits_current_value_immediately() {
    let state = test_state(SAMPLE_STATE_VALUE);
    let host = silent_host();
    host.set_app_state_value(Some(state.clone()));
    let received_states = Rc::new(RefCell::new(Vec::new()));
    let log = received_states.clone();

    let subscription = host.observe_app_state_typed().observe_on(ImmediateScheduler::instance()).subscribe_next(move |state| log.borrow_mut().push(state));

    let received_states = received_states.borrow();
    assert_eq!(received_states.len(), 1);
    assert!(Rc::ptr_eq(&received_states[0], &state));
    subscription.dispose();
    host.dispose();
}

#[test]
fn observe_app_state_typed_emits_subsequent_changes() {
    let host = silent_host();
    let received_states = Rc::new(RefCell::new(Vec::new()));
    let log = received_states.clone();
    let subscription = host.observe_app_state_typed().observe_on(ImmediateScheduler::instance()).subscribe_next(move |state| log.borrow_mut().push(state));

    let state1 = test_state(1);
    host.set_app_state_value(Some(state1.clone()));
    let state2 = test_state(SECOND_STATE_VALUE);
    host.set_app_state_value(Some(state2.clone()));

    let received_states = received_states.borrow();
    assert_eq!(received_states.len(), EXPECTED_EMISSION_COUNT);
    assert!(Rc::ptr_eq(&received_states[0], &state1));
    assert!(Rc::ptr_eq(&received_states[1], &state2));
    subscription.dispose();
    host.dispose();
}

#[test]
fn observe_app_state_typed_filters_null_values() {
    let host = silent_host();
    let received_states = Rc::new(RefCell::new(Vec::new()));
    let log = received_states.clone();
    let subscription = host.observe_app_state_typed().observe_on(ImmediateScheduler::instance()).subscribe_next(move |state| log.borrow_mut().push(state));

    host.set_app_state_value(None);
    let state = test_state(SAMPLE_STATE_VALUE);
    host.set_app_state_value(Some(state.clone()));
    host.set_app_state_value(None);

    let received_states = received_states.borrow();
    assert_eq!(received_states.len(), 1);
    assert!(Rc::ptr_eq(&received_states[0], &state));
    subscription.dispose();
    host.dispose();
}

// Not ported: ObserveAppState_Typed_ThrowsForNullHost - argument validation
// Not ported: SetupDefaultSuspendResume_Typed_ThrowsForNullHost - argument validation
// Not ported: SetupDefaultSuspendResume_Typed_ThrowsForNullTypeInfo - argument validation (the type information is the type of the driver)

#[test]
fn setup_default_suspend_resume_typed_with_provided_driver_uses_provided_driver() {
    let host = silent_host();
    host.set_create_new_app_state_typed(Some(Rc::new(|| Rc::new(TestAppState::default()))));
    let driver = TestSuspensionDriver::loading(test_state(SAMPLE_STATE_VALUE));

    let disposable = host.setup_default_suspend_resume_typed_with(driver.as_driver());

    disposable.dispose();
    host.dispose();
}

#[test]
fn setup_default_suspend_resume_typed_should_persist_state_calls_driver_save_state() {
    let app_state = test_state(FACTORY_STATE_VALUE);
    let host = silent_host();
    host.set_app_state_value(Some(app_state.clone()));
    let driver = TestSuspensionDriver::<TestAppState>::new();
    let persist_subject = Subject::<Rc<dyn IDisposable>>::new();
    host.set_should_persist_state(persist_subject.as_observable().observe_on(ImmediateScheduler::instance()));
    let disposable = host.setup_default_suspend_resume_typed_with(driver.as_driver());

    persist_subject.on_next(Disposable::empty());

    assert_eq!(driver.save_state_call_count.get(), 1);
    assert!(same(&driver.last_saved_state(), &Some(app_state)));
    disposable.dispose();
    host.dispose();
}

#[test]
fn setup_default_suspend_resume_typed_should_persist_created_state_when_no_persisted_state_and_persist_occurs_before_get_app_state() {
    const CREATED_STATE_VALUE_FOR_NO_PERSISTED_STATE: i32 = 321;
    let created_state = test_state(CREATED_STATE_VALUE_FOR_NO_PERSISTED_STATE);
    let create_new_app_state_call_count = Rc::new(Cell::new(0));
    let persist_token_disposed = Rc::new(Cell::new(false));
    let host = silent_host();
    let (created, count) = (created_state.clone(), create_new_app_state_call_count.clone());
    host.set_create_new_app_state_typed(Some(Rc::new(move || {
        count.set(count.get() + 1);
        created.clone()
    })));
    let driver = TestSuspensionDriver::<TestAppState>::new();
    let persist_subject = Subject::<Rc<dyn IDisposable>>::new();
    host.set_should_persist_state(persist_subject.as_observable().observe_on(ImmediateScheduler::instance()));
    let disposable = host.setup_default_suspend_resume_typed_with(driver.as_driver());

    let disposed = persist_token_disposed.clone();
    persist_subject.on_next(Disposable::create(move || disposed.set(true)));

    assert_eq!(driver.load_state_call_count.get(), 1);
    assert_eq!(create_new_app_state_call_count.get(), 1);
    assert!(same(&host.app_state_value(), &Some(created_state.clone())));
    assert_eq!(driver.save_state_call_count.get(), 1);
    assert!(same(&driver.last_saved_state(), &Some(created_state)));
    assert!(persist_token_disposed.get());
    disposable.dispose();
    host.dispose();
}

#[test]
fn setup_default_suspend_resume_typed_should_persist_created_state_when_launch_signal_was_raised_before_setup() {
    const CREATED_STATE_VALUE_FOR_LAUNCH_BEFORE_SETUP: i32 = 654;
    let created_state = test_state(CREATED_STATE_VALUE_FOR_LAUNCH_BEFORE_SETUP);
    let create_new_app_state_call_count = Rc::new(Cell::new(0));
    let host = SuspensionHostOf::<TestAppState>::new();
    let (created, count) = (created_state.clone(), create_new_app_state_call_count.clone());
    host.set_create_new_app_state_typed(Some(Rc::new(move || {
        count.set(count.get() + 1);
        created.clone()
    })));
    host.set_should_invalidate_state(Observable::never());
    let launch_subject = Subject::<()>::new();
    let resume_subject = Subject::<()>::new();
    let persist_subject = Subject::<Rc<dyn IDisposable>>::new();
    host.set_is_launching_new(launch_subject.as_observable().observe_on(ImmediateScheduler::instance()));
    host.set_is_resuming(resume_subject.as_observable().observe_on(ImmediateScheduler::instance()));
    host.set_should_persist_state(persist_subject.as_observable().observe_on(ImmediateScheduler::instance()));
    launch_subject.on_next(());
    let driver = TestSuspensionDriver::<TestAppState>::new();
    let disposable = host.setup_default_suspend_resume_typed_with(driver.as_driver());

    persist_subject.on_next(Disposable::empty());

    assert_eq!(driver.load_state_call_count.get(), 1);
    assert_eq!(create_new_app_state_call_count.get(), 1);
    assert!(same(&host.app_state_value(), &Some(created_state.clone())));
    assert_eq!(driver.save_state_call_count.get(), 1);
    assert!(same(&driver.last_saved_state(), &Some(created_state)));
    disposable.dispose();
    host.dispose();
}

#[test]
fn setup_default_suspend_resume_typed_two_hosts_with_different_drivers_each_host_uses_its_own_driver() {
    let first_persist = Subject::<Rc<dyn IDisposable>>::new();
    let first_host = silent_host();
    first_host.set_should_persist_state(first_persist.as_observable().observe_on(ImmediateScheduler::instance()));
    let second_persist = Subject::<Rc<dyn IDisposable>>::new();
    let second_host = silent_host();
    second_host.set_should_persist_state(second_persist.as_observable().observe_on(ImmediateScheduler::instance()));
    let first_driver = TestSuspensionDriver::loading(test_state(SAMPLE_STATE_VALUE));
    let second_driver = TestSuspensionDriver::loading(test_state(LOADED_STATE_VALUE));
    let first_setup = first_host.setup_default_suspend_resume_typed_with(first_driver.as_driver());
    let second_setup = second_host.setup_default_suspend_resume_typed_with(second_driver.as_driver());

    let first_state = first_host.get_app_state_typed();
    let second_state = second_host.get_app_state_typed();
    first_persist.on_next(Disposable::empty());
    second_persist.on_next(Disposable::empty());

    assert!(same(&first_state, &first_driver.state_to_load()));
    assert!(same(&second_state, &second_driver.state_to_load()));
    assert_eq!(first_driver.load_state_call_count.get(), 1);
    assert_eq!(second_driver.load_state_call_count.get(), 1);
    assert_eq!(first_driver.save_state_call_count.get(), 1);
    assert_eq!(second_driver.save_state_call_count.get(), 1);
    assert!(same(&first_driver.last_saved_state(), &first_driver.state_to_load()));
    assert!(same(&second_driver.last_saved_state(), &second_driver.state_to_load()));
    first_setup.dispose();
    second_setup.dispose();
}

/// The states created by the tests carry the value upstream reads (`CreatedStateValue`).
pub(super) fn created_state() -> Rc<TestAppState> {
    test_state(CREATED_STATE_VALUE)
}
