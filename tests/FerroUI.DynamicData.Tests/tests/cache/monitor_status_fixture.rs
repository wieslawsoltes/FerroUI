//! `MonitorStatusFixture` (`DynamicData.Tests/Cache/MonitorStatusFixture.cs`): the status of the
//! connection to a sequence: pending, loaded on the first value, errored on an error.

use ferroui_dynamic_data::cache::ObservableCacheSequenceEx;
use ferroui_dynamic_data::kernel::ConnectionStatus;
use ferroui_reactive::subjects::Subject;
use ferroui_reactive::ObservableExt;
use ferroui_system::{IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::Rc;

/// The error of the tests (`new Exception("Test")`).
#[derive(Debug)]
struct TestError;

impl fmt::Display for TestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Test")
    }
}

impl std::error::Error for TestError {}

#[test]
fn initiali_status_is_loadding() {
    let invoked = Rc::new(Cell::new(false));
    let status = Rc::new(Cell::new(ConnectionStatus::Pending));
    let (on_invoked, on_status) = (invoked.clone(), status.clone());
    let subscription = Subject::<i32>::new().as_observable().monitor_status().subscribe_next(move |s: ConnectionStatus| {
        on_invoked.set(true);
        on_status.set(s);
    });

    assert!(invoked.get());
    assert_eq!(status.get(), ConnectionStatus::Pending, "No status has been received");
    subscription.dispose();
}

#[test]
fn multiple_invokes_do_not_call_loaded_again() {
    let invoked = Rc::new(Cell::new(false));
    let invocations = Rc::new(Cell::new(0));
    let subject = Subject::<i32>::new();
    let (on_invoked, on_invocations) = (invoked.clone(), invocations.clone());
    let subscription = subject
        .as_observable()
        .monitor_status()
        .where_(|status: &ConnectionStatus| *status == ConnectionStatus::Loaded)
        .subscribe_next(move |_: ConnectionStatus| {
            on_invoked.set(true);
            on_invocations.set(on_invocations.get() + 1);
        });

    subject.on_next(1);
    subject.on_next(1);
    subject.on_next(1);

    assert!(invoked.get());
    assert_eq!(invocations.get(), 1, "Status should be ConnectionStatus.Loaded");
    subscription.dispose();
}

#[test]
fn set_to_error() {
    let invoked = Rc::new(Cell::new(false));
    let status = Rc::new(Cell::new(ConnectionStatus::Pending));
    let subject = Subject::<i32>::new();
    let exception: Rc<RefCell<Option<ObservableError>>> = Rc::new(RefCell::new(None));

    let (on_invoked, on_status, on_exception) = (invoked.clone(), status.clone(), exception.clone());
    let subscription = subject.as_observable().monitor_status().subscribe_next_error(
        move |s: ConnectionStatus| {
            on_invoked.set(true);
            on_status.set(s);
        },
        move |ex: ObservableError| {
            *on_exception.borrow_mut() = Some(ex);
        },
    );

    subject.on_error(Rc::new(TestError));
    subscription.dispose();

    assert!(invoked.get());
    assert_eq!(status.get(), ConnectionStatus::Errored, "Status should be ConnectionStatus.Faulted");
}

#[test]
fn set_to_loaded() {
    let invoked = Rc::new(Cell::new(false));
    let status = Rc::new(Cell::new(ConnectionStatus::Pending));
    let subject = Subject::<i32>::new();
    let (on_invoked, on_status) = (invoked.clone(), status.clone());
    let subscription = subject.as_observable().monitor_status().subscribe_next(move |s: ConnectionStatus| {
        on_invoked.set(true);
        on_status.set(s);
    });

    subject.on_next(1);
    assert!(invoked.get());
    assert_eq!(status.get(), ConnectionStatus::Loaded, "Status should be ConnectionStatus.Loaded");
    subscription.dispose();
}
