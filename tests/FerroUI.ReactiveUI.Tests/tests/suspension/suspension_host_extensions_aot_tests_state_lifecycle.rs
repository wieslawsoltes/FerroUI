//! `SuspensionHostExtensionsAotTests.StateLifecycle`
//! (`ReactiveUI.Tests/Suspension/SuspensionHostExtensionsAotTests.StateLifecycle.cs`).

use super::suspension_host_extensions_aot_tests::{
    created_state, same, silent_host, test_state, TestAppState, TestSuspensionDriver, FACTORY_STATE_VALUE, LOADED_STATE_VALUE, SAMPLE_STATE_VALUE,
};
use ferroui_reactive::concurrency::ImmediateScheduler;
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::subjects::Subject;
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::*;
use ferroui_splat::AppLocator;
use ferroui_system::{IDisposable, IObserver};
use std::cell::Cell;
use std::rc::Rc;

#[test]
fn setup_default_suspend_resume_typed_should_persist_loaded_state_when_persist_occurs_before_get_app_state() {
    const LOADED_STATE_VALUE_FOR_PERSIST_BEFORE_GET_APP_STATE: i32 = 987;
    let loaded_state = test_state(LOADED_STATE_VALUE_FOR_PERSIST_BEFORE_GET_APP_STATE);
    let create_new_app_state_call_count = Rc::new(Cell::new(0));
    let host = silent_host();
    let count = create_new_app_state_call_count.clone();
    host.set_create_new_app_state_typed(Some(Rc::new(move || {
        count.set(count.get() + 1);
        Rc::new(TestAppState::default())
    })));
    let driver = TestSuspensionDriver::loading(loaded_state.clone());
    let persist_subject = Subject::<Rc<dyn IDisposable>>::new();
    host.set_should_persist_state(persist_subject.as_observable().observe_on(ImmediateScheduler::instance()));
    let disposable = host.setup_default_suspend_resume_typed_with(driver.as_driver());

    persist_subject.on_next(Disposable::empty());

    assert_eq!(driver.load_state_call_count.get(), 1);
    assert_eq!(create_new_app_state_call_count.get(), 0);
    assert!(same(&host.app_state_value(), &Some(loaded_state.clone())));
    assert_eq!(driver.save_state_call_count.get(), 1);
    assert!(same(&driver.last_saved_state(), &Some(loaded_state)));
    disposable.dispose();
    host.dispose();
}

#[test]
fn setup_default_suspend_resume_typed_should_dispose_persist_token_after_save() {
    const APP_STATE_VALUE_FOR_DISPOSE_PERSIST_TOKEN: i32 = 111;
    let app_state = test_state(APP_STATE_VALUE_FOR_DISPOSE_PERSIST_TOKEN);
    let persist_token_disposed = Rc::new(Cell::new(false));
    let host = silent_host();
    host.set_app_state_value(Some(app_state));
    let driver = TestSuspensionDriver::<TestAppState>::new();
    let persist_subject = Subject::<Rc<dyn IDisposable>>::new();
    host.set_should_persist_state(persist_subject.as_observable().observe_on(ImmediateScheduler::instance()));
    let disposable = host.setup_default_suspend_resume_typed_with(driver.as_driver());

    let disposed = persist_token_disposed.clone();
    persist_subject.on_next(Disposable::create(move || disposed.set(true)));

    assert_eq!(driver.save_state_call_count.get(), 1);
    assert!(persist_token_disposed.get());
    disposable.dispose();
    host.dispose();
}

#[test]
fn setup_default_suspend_resume_typed_should_invalidate_state_calls_driver_invalidate_state() {
    let host = silent_host();
    let driver = TestSuspensionDriver::<TestAppState>::new();
    let invalidate_subject = Subject::<()>::new();
    host.set_should_invalidate_state(invalidate_subject.as_observable().observe_on(ImmediateScheduler::instance()));
    let disposable = host.setup_default_suspend_resume_typed_with(driver.as_driver());

    invalidate_subject.on_next(());

    assert_eq!(driver.invalidate_state_call_count.get(), 1);
    disposable.dispose();
    host.dispose();
}

#[test]
fn setup_default_suspend_resume_typed_is_launching_new_triggers_state_load() {
    let host = silent_host();
    host.set_create_new_app_state_typed(Some(Rc::new(|| Rc::new(TestAppState::default()))));
    let driver = TestSuspensionDriver::loading(test_state(LOADED_STATE_VALUE));
    let launch_subject = Subject::<()>::new();
    let resume_subject = Subject::<()>::new();
    host.set_is_launching_new(launch_subject.as_observable().observe_on(ImmediateScheduler::instance()));
    host.set_is_resuming(resume_subject.as_observable().observe_on(ImmediateScheduler::instance()));
    let disposable = host.setup_default_suspend_resume_typed_with(driver.as_driver());

    launch_subject.on_next(());

    assert_eq!(host.app_state_value().map(|state| state.value), Some(LOADED_STATE_VALUE));
    disposable.dispose();
    host.dispose();
}

#[test]
fn setup_default_suspend_resume_typed_with_null_driver_logs_error_and_returns_empty_disposable() {
    let host = silent_host();
    AppLocator::unregister_all::<dyn ISuspensionDriverOf<TestAppState>>();

    let disposable = host.setup_default_suspend_resume_typed();

    disposable.dispose();
    host.dispose();
}

#[test]
fn ensure_load_app_state_typed_with_existing_state_does_not_load() {
    let existing_state = test_state(FACTORY_STATE_VALUE);
    let host = silent_host();
    host.set_app_state_value(Some(existing_state.clone()));
    let driver = TestSuspensionDriver::<TestAppState>::new();
    let disposable = host.setup_default_suspend_resume_typed_with(driver.as_driver());

    let state = host.get_app_state_typed();

    assert_eq!(driver.load_state_call_count.get(), 0);
    assert!(same(&state, &Some(existing_state)));
    disposable.dispose();
    host.dispose();
}

#[test]
fn ensure_load_app_state_typed_load_state_throws_creates_new_app_state() {
    let host = silent_host();
    host.set_create_new_app_state_typed(Some(Rc::new(created_state)));
    let driver = TestSuspensionDriver::<TestAppState>::throwing_on_load();
    let disposable = host.setup_default_suspend_resume_typed_with(driver.as_driver());

    let state = host.get_app_state_typed();

    assert_eq!(state.map(|state| state.value), Some(created_state().value));
    assert_eq!(driver.load_state_call_count.get(), 1);
    disposable.dispose();
    host.dispose();
}

#[test]
fn ensure_load_app_state_typed_with_null_create_new_app_state_sets_state_to_null() {
    let host = silent_host();
    host.set_create_new_app_state_typed(None);
    let driver = TestSuspensionDriver::<TestAppState>::throwing_on_load();
    let disposable = host.setup_default_suspend_resume_typed_with(driver.as_driver());

    let state = host.get_app_state_typed();

    assert!(state.is_none());
    disposable.dispose();
    host.dispose();
}

#[test]
fn ensure_load_app_state_typed_locator_drivers_removed_after_setup_loads_through_setup_driver() {
    let host = silent_host();
    host.set_create_new_app_state_typed(Some(Rc::new(|| Rc::new(TestAppState::default()))));
    let driver = TestSuspensionDriver::loading(test_state(SAMPLE_STATE_VALUE));
    let disposable = host.setup_default_suspend_resume_typed_with(driver.as_driver());
    AppLocator::unregister_all::<dyn ISuspensionDriverOf<TestAppState>>();

    let state = host.get_app_state_typed();

    assert!(same(&state, &driver.state_to_load()));
    assert_eq!(driver.load_state_call_count.get(), 1);
    disposable.dispose();
    host.dispose();
}

#[test]
fn ensure_load_app_state_typed_only_loads_once() {
    let host = silent_host();
    host.set_create_new_app_state_typed(Some(Rc::new(|| Rc::new(TestAppState::default()))));
    let driver = TestSuspensionDriver::loading(test_state(SAMPLE_STATE_VALUE));
    let disposable = host.setup_default_suspend_resume_typed_with(driver.as_driver());

    let state1 = host.get_app_state_typed();
    let state2 = host.get_app_state_typed();

    assert_eq!(driver.load_state_call_count.get(), 1);
    assert!(same(&state1, &state2));
    disposable.dispose();
    host.dispose();
}
