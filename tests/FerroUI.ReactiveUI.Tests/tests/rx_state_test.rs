//! `RxStateTest` (`ReactiveUI.Tests/RxStateTest.cs`).

use ferroui_reactiveui::*;
use ferroui_system::{IObserver, ObservableError};
use std::rc::Rc;

/// An exception handler that does nothing (`NoOpHandler`).
struct NoOpHandler;

impl IObserver<ObservableError> for NoOpHandler {
    fn on_next(&self, _value: ObservableError) {}

    fn on_error(&self, _error: ObservableError) {}

    fn on_completed(&self) {}
}

fn same(first: &Rc<dyn IObserver<ObservableError>>, second: &Rc<dyn IObserver<ObservableError>>) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(first), Rc::as_ptr(second))
}

// Not ported: DefaultExceptionHandler_ConcurrentFirstRead_NeverReturnsNull - real threads
// Not ported: InitializeExceptionHandler_Null_ThrowsAndKeepsDefault - argument validation

#[test]
fn initialize_exception_handler_before_first_read_is_returned() {
    RxState::reset_for_testing();
    let handler: Rc<dyn IObserver<ObservableError>> = Rc::new(NoOpHandler);

    RxState::initialize_exception_handler(handler.clone());

    assert!(same(&RxState::default_exception_handler(), &handler));
    RxState::reset_for_testing();
}

#[test]
fn initialize_exception_handler_after_first_read_keeps_first_handler() {
    RxState::reset_for_testing();
    let first = RxState::default_exception_handler();

    RxState::initialize_exception_handler(Rc::new(NoOpHandler));

    assert!(same(&RxState::default_exception_handler(), &first));
    RxState::reset_for_testing();
}
