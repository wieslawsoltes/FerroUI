//! `DummySuspensionDriverTests` (`ReactiveUI.Tests/Suspension/DummySuspensionDriverTests.cs`).
//!
//! Upstream asserts that each method returns an observable that is not null; the observables are
//! subscribed here, which shows that they exist and complete.

use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::*;
use ferroui_system::IObservable;
use std::any::Any;
use std::cell::Cell;
use std::rc::Rc;

/// Whether `source` completes when it is subscribed.
fn completes<T: 'static>(source: Rc<dyn IObservable<T>>) -> bool {
    let completed = Rc::new(Cell::new(false));
    let seen = completed.clone();
    let _ = source.subscribe_next_completed(|_| {}, move || seen.set(true));
    completed.get()
}

#[test]
fn dummy_suspension_driver_invalidate_state_returns_observable() {
    let driver = DummySuspensionDriver::new();
    assert!(completes(driver.invalidate_state()));
}

#[test]
fn dummy_suspension_driver_load_state_returns_observable() {
    let driver = DummySuspensionDriver::new();
    assert!(completes(driver.load_state()));
}

#[test]
fn dummy_suspension_driver_save_state_null_state_returns_observable() {
    let driver = DummySuspensionDriver::new();
    assert!(completes(driver.save_state(None)));
}

#[test]
fn dummy_suspension_driver_save_state_returns_observable() {
    let driver = DummySuspensionDriver::new();
    let state: Rc<dyn Any> = Rc::new(String::from("test"));
    assert!(completes(driver.save_state(Some(state))));
}
