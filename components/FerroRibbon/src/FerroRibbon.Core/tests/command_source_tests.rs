//! Direct ports of the three relay/async CommandTests in upstream ModelTests.cs.
use ferroribbon_core::commands::*;
#[path = "support/command_executor.rs"]
pub mod executor;
use executor::{Executor, channel};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[test]
fn relay_command_respects_can_execute() {
    let count = Rc::new(Cell::new(0));
    let enabled = Rc::new(Cell::new(false));
    let c = count.clone();
    let e = enabled.clone();
    let command = RibbonRelayCommand::new_with_can_execute(
        move || c.set(c.get() + 1),
        Some(Rc::new(move || e.get())),
    );
    command.execute(None);
    assert_eq!(count.get(), 0);
    enabled.set(true);
    command.execute(None);
    assert_eq!(count.get(), 1);
}

#[test]
fn typed_command_passes_parameter() {
    let received = Rc::new(RefCell::new(None));
    let r = received.clone();
    RibbonRelayCommandOf::<Option<String>>::new(move |s| *r.borrow_mut() = s, None)
        .execute(Some(&"hello".to_owned()));
    assert_eq!(received.borrow().as_deref(), Some("hello"));
}

#[test]
fn async_command_disables_while_running_and_reports_errors() {
    let mut executor = Executor::new();
    let (completion, c) = channel();
    let command = RibbonAsyncCommand::new(executor.spawner(), move || c.clone());
    let run = command.execute_async(None);
    assert!(!command.can_execute(None));
    completion.send(Ok(())).unwrap();
    executor.wait(run).unwrap();
    assert!(command.can_execute(None));
    // A diverging factory needs an explicit future output under a generic Rust API.
    // It still panics before creating/polling a future, preserving this boundary.
    let failing = RibbonAsyncCommand::new(
        executor.spawner(),
        || -> std::future::Ready<Result<(), ferroui_system::ObservableError>> { panic!("boom") },
    );
    let reported = Rc::new(RefCell::new(None));
    let r = reported.clone();
    failing.failed(Rc::new(move |e| {
        *r.borrow_mut() = Some(e.exception());
        e.set_handled(true);
    }));
    executor.wait(failing.execute_async(None)).unwrap();
    let reported = reported.borrow();
    let error = reported.as_ref().unwrap();
    // Reference exception vocabulary stays in development fixtures; this is a Rust error.
    assert_eq!(error.to_string(), "boom");
}
