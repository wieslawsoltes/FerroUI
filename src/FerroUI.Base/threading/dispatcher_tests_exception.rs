// Some of these tests are based on the WPF dispatcher unhandled-exception tests.

use std::any::Any;
use std::cell::Cell;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;
use std::sync::Arc;
use std::thread;

use super::{
    CancellationToken, Dispatcher, DispatcherEventArgs, DispatcherFrame, DispatcherPriority,
    DispatcherUnhandledExceptionEventArgs, DispatcherUnhandledExceptionFilterEventArgs, UnitTestDispatcherScope,
};

pub(super) const EXPECTED_EXCEPTION_TEXT: &str = "Exception thrown inside Dispatcher.Invoke / Dispatcher.BeginInvoke.";

/// The per-test state of the dispatcher tests. Every test runs on its own
/// thread with its own dispatcher.
pub(super) struct DispatcherTests {
    pub(super) number_of_handler_on_unhandled_event_invoked: Rc<Cell<i32>>,
    pub(super) number_of_handler_on_unhandled_event_filter_invoked: Rc<Cell<i32>>,
    pub(super) ui_thread: Arc<Dispatcher>,
    // Dropped last: resets the dispatcher of the test thread.
    _scope: UnitTestDispatcherScope,
}

pub(super) fn message_of(payload: &(dyn Any + Send)) -> Option<&str> {
    if let Some(message) = payload.downcast_ref::<&'static str>() {
        Some(message)
    } else {
        payload.downcast_ref::<String>().map(String::as_str)
    }
}

fn throw_an_exception() {
    panic!("{}", EXPECTED_EXCEPTION_TEXT);
}

impl DispatcherTests {
    pub(super) fn new() -> Self {
        let scope = Dispatcher::unit_test_scope();
        Self::verify_dispatcher_sanity();
        Self {
            number_of_handler_on_unhandled_event_invoked: Rc::new(Cell::new(0)),
            number_of_handler_on_unhandled_event_filter_invoked: Rc::new(Cell::new(0)),
            ui_thread: Dispatcher::current_dispatcher(),
            _scope: scope,
        }
    }

    fn verify_dispatcher_sanity() {
        // Verify that we are in a clear-ish state. Do this for every test to ensure that our reset procedure is working
        assert!(Dispatcher::from_thread(thread::current().id()).is_none());
        assert!(Dispatcher::try_get_ui_thread().is_none());

        // The first (this) dispatcher becomes UI thread one
        let current = Dispatcher::current_dispatcher();
        assert!(Arc::ptr_eq(&Dispatcher::try_get_ui_thread().unwrap(), &current));
        assert!(Arc::ptr_eq(&Dispatcher::ui_thread(), &current));

        // Dispatcher::from_thread works
        assert!(Arc::ptr_eq(&current, &Dispatcher::from_thread(thread::current().id()).unwrap()));
        assert!(Arc::ptr_eq(&Dispatcher::ui_thread(), &Dispatcher::from_thread(thread::current().id()).unwrap()));
    }

    /// Posts the panicking job, runs the queue and reports whether the
    /// expected panic came out of it (`None` when nothing was raised).
    fn post_and_run(&self) -> Option<bool> {
        let ui_thread = self.ui_thread.clone();
        let result = catch_unwind(AssertUnwindSafe(|| {
            ui_thread.post(throw_an_exception, DispatcherPriority::NORMAL);
            ui_thread.run_jobs_with_cancellation(None, &CancellationToken::none());
        }));
        match result {
            Ok(()) => None,
            Err(e) => Some(message_of(&*e) == Some(EXPECTED_EXCEPTION_TEXT)),
        }
    }

    fn verification(
        &self,
        caught_correct_exception: bool,
        number_of_handler_on_unhandled_event_should_invoke: i32,
        number_of_handler_on_unhandled_event_filter_should_invoke: i32,
    ) {
        assert!(
            self.number_of_handler_on_unhandled_event_invoked.get() >= number_of_handler_on_unhandled_event_should_invoke,
            "Number of handler invoked on UnhandledException is invalid"
        );

        assert!(
            self.number_of_handler_on_unhandled_event_filter_invoked.get()
                >= number_of_handler_on_unhandled_event_filter_should_invoke,
            "Number of handler invoked on UnhandledExceptionFilter is invalid"
        );

        assert!(caught_correct_exception, "Wrong exception caught.");
    }

    fn handler_on_unhandled_exception_filter_request_catch(
        &self,
    ) -> impl for<'a> Fn(&DispatcherUnhandledExceptionFilterEventArgs<'a>) + 'static {
        let count = self.number_of_handler_on_unhandled_event_filter_invoked.clone();
        move |args| {
            args.set_request_catch(true);

            count.set(count.get() + 1);
            assert_eq!(args.exception_message(), Some(EXPECTED_EXCEPTION_TEXT));
        }
    }

    fn handler_on_unhandled_exception_filter_not_request_catch_push_frame(
        &self,
    ) -> impl for<'a> Fn(&DispatcherUnhandledExceptionFilterEventArgs<'a>) + 'static {
        let inner = self.handler_on_unhandled_exception_filter_not_request_catch();
        move |args| {
            inner(args);
            let frame = DispatcherFrame::new();
            let to_stop = frame.clone();
            args.dispatcher()
                .invoke_async_with_priority(move || to_stop.set_continue(false), DispatcherPriority::BACKGROUND);
            args.dispatcher().push_frame(&frame);
        }
    }

    fn handler_on_unhandled_exception_filter_not_request_catch(
        &self,
    ) -> impl for<'a> Fn(&DispatcherUnhandledExceptionFilterEventArgs<'a>) + 'static {
        let count = self.number_of_handler_on_unhandled_event_filter_invoked.clone();
        move |args| {
            args.set_request_catch(false);
            count.set(count.get() + 1);

            assert_eq!(args.exception_message(), Some(EXPECTED_EXCEPTION_TEXT));
        }
    }

    fn handler_on_unhandled_exception_handled_push_frame(
        &self,
    ) -> impl for<'a> Fn(&DispatcherUnhandledExceptionEventArgs<'a>) + 'static {
        let count = self.number_of_handler_on_unhandled_event_invoked.clone();
        let filter_count = self.number_of_handler_on_unhandled_event_filter_invoked.clone();
        move |args| {
            assert_eq!(args.exception_message(), Some(EXPECTED_EXCEPTION_TEXT));
            assert!(filter_count.get() != 0, "UnhandledExceptionFilter should be invoked before UnhandledException.");

            args.set_handled(true);
            count.set(count.get() + 1);

            let dispatcher = args.dispatcher();
            let frame = DispatcherFrame::new();
            dispatcher.begin_invoke_shutdown(DispatcherPriority::BACKGROUND);
            dispatcher.push_frame(&frame);
        }
    }

    fn handler_on_unhandled_exception_handled(
        &self,
    ) -> impl for<'a> Fn(&DispatcherUnhandledExceptionEventArgs<'a>) + 'static {
        let count = self.number_of_handler_on_unhandled_event_invoked.clone();
        let filter_count = self.number_of_handler_on_unhandled_event_filter_invoked.clone();
        move |args| {
            assert_eq!(args.exception_message(), Some(EXPECTED_EXCEPTION_TEXT));
            assert!(filter_count.get() != 0, "UnhandledExceptionFilter should be invoked before UnhandledException.");

            args.set_handled(true);
            count.set(count.get() + 1);
        }
    }

    fn handler_on_unhandled_exception_not_handled(
        &self,
    ) -> impl for<'a> Fn(&DispatcherUnhandledExceptionEventArgs<'a>) + 'static {
        let count = self.number_of_handler_on_unhandled_event_invoked.clone();
        let filter_count = self.number_of_handler_on_unhandled_event_filter_invoked.clone();
        move |args| {
            assert_eq!(args.exception_message(), Some(EXPECTED_EXCEPTION_TEXT));
            assert!(filter_count.get() != 0, "UnhandledExceptionFilter should be invoked before UnhandledException.");

            args.set_handled(false);
            count.set(count.get() + 1);
        }
    }
}

#[test]
fn different_threads_auto_spawn_dispatchers() {
    let _t = DispatcherTests::new();
    let dispatcher = Dispatcher::current_dispatcher();
    thread::spawn(move || {
        assert!(Dispatcher::from_thread(thread::current().id()).is_none());
        let current = Dispatcher::current_dispatcher();
        assert!(!Arc::ptr_eq(&dispatcher, &current));
        assert!(Arc::ptr_eq(&current, &Dispatcher::from_thread(thread::current().id()).unwrap()));
        assert!(current.check_access());
        assert!(!dispatcher.check_access());
    })
    .join()
    .unwrap();
}

#[test]
fn dispatcher_handles_exception_with_post() {
    let t = DispatcherTests::new();
    let handled = Rc::new(Cell::new(false));
    let executed = Rc::new(Cell::new(false));
    let h = handled.clone();
    t.ui_thread.unhandled_exception(move |args| {
        h.set(true);
        args.set_handled(true);
    });
    t.ui_thread.post(throw_an_exception, DispatcherPriority::DEFAULT);
    let e = executed.clone();
    t.ui_thread.post_local(move || e.set(true), DispatcherPriority::DEFAULT);

    t.ui_thread.run_jobs_with_cancellation(None, &CancellationToken::none());

    assert!(handled.get());
    assert!(executed.get());
}

#[test]
fn dispatcher_handles_exception_with_post_local() {
    let t = DispatcherTests::new();
    let handled = Rc::new(Cell::new(false));
    let h = handled.clone();
    t.ui_thread.unhandled_exception(move |args| {
        h.set(true);
        args.set_handled(true);
    });
    t.ui_thread.post_local(throw_an_exception, DispatcherPriority::DEFAULT);

    t.ui_thread.run_jobs(None);

    assert!(handled.get());
}

#[test]
fn can_remove_dispatcher_exception_handler() {
    let t = DispatcherTests::new();

    let filter = t.ui_thread.unhandled_exception_filter(t.handler_on_unhandled_exception_filter_request_catch());
    let handler = t.ui_thread.unhandled_exception(t.handler_on_unhandled_exception_not_handled());

    filter.dispose();
    handler.dispose();

    let caught_correct_exception = t.post_and_run().unwrap_or(false);
    t.verification(caught_correct_exception, 0, 0);
    assert_eq!(t.number_of_handler_on_unhandled_event_invoked.get(), 0);
    assert_eq!(t.number_of_handler_on_unhandled_event_filter_invoked.get(), 0);
}

#[test]
fn can_handle_exception_with_unhandled_exception() {
    let t = DispatcherTests::new();
    t.ui_thread.unhandled_exception_filter(t.handler_on_unhandled_exception_filter_request_catch());

    t.ui_thread.unhandled_exception(t.handler_on_unhandled_exception_handled());
    // should be no exception here.
    let caught_correct_exception = t.post_and_run().is_none();
    t.verification(caught_correct_exception, 1, 1);
}

#[test]
fn invoke_method_doesnt_trigger_unhandled_exception() {
    let t = DispatcherTests::new();
    t.ui_thread.unhandled_exception_filter(t.handler_on_unhandled_exception_filter_request_catch());

    t.ui_thread.unhandled_exception(t.handler_on_unhandled_exception_handled());
    // Since both invoke and invoke_async can re-raise the panic, there is no need to pass them to the unhandled_exception.
    let ui_thread = t.ui_thread.clone();
    let result = catch_unwind(AssertUnwindSafe(|| {
        let _ = ui_thread.invoke_with_priority(throw_an_exception, DispatcherPriority::NORMAL);
        ui_thread.run_jobs(None);
    }));
    let caught_correct_exception = match result {
        Ok(()) => false,
        Err(e) => message_of(&*e) == Some(EXPECTED_EXCEPTION_TEXT),
    };
    t.verification(caught_correct_exception, 0, 0);
    assert_eq!(t.number_of_handler_on_unhandled_event_invoked.get(), 0);
    assert_eq!(t.number_of_handler_on_unhandled_event_filter_invoked.get(), 0);
}

#[test]
fn invoke_async_method_doesnt_trigger_unhandled_exception() {
    let t = DispatcherTests::new();
    t.ui_thread.unhandled_exception_filter(t.handler_on_unhandled_exception_filter_request_catch());

    t.ui_thread.unhandled_exception(t.handler_on_unhandled_exception_handled());
    // Since both invoke and invoke_async can re-raise the panic, there is no need to pass them to the unhandled_exception.
    let ui_thread = t.ui_thread.clone();
    let result = catch_unwind(AssertUnwindSafe(|| {
        let op = ui_thread.invoke_async_with_priority(throw_an_exception, DispatcherPriority::NORMAL);
        let _ = op.wait();
        ui_thread.run_jobs(None);
    }));
    let caught_correct_exception = match result {
        Ok(()) => false,
        Err(e) => message_of(&*e) == Some(EXPECTED_EXCEPTION_TEXT),
    };
    t.verification(caught_correct_exception, 0, 0);
    assert_eq!(t.number_of_handler_on_unhandled_event_invoked.get(), 0);
    assert_eq!(t.number_of_handler_on_unhandled_event_filter_invoked.get(), 0);
}

#[test]
fn can_rethrow_exception_with_unhandled_exception() {
    let t = DispatcherTests::new();
    t.ui_thread.unhandled_exception_filter(t.handler_on_unhandled_exception_filter_request_catch());

    t.ui_thread.unhandled_exception(t.handler_on_unhandled_exception_not_handled());
    let caught_correct_exception = t.post_and_run().unwrap_or(false);
    t.verification(caught_correct_exception, 1, 1);
}

#[test]
fn multiple_unhandled_exception_filter_cannot_reset_request_catch_flag() {
    let t = DispatcherTests::new();
    t.ui_thread.unhandled_exception_filter(t.handler_on_unhandled_exception_filter_not_request_catch());
    t.ui_thread.unhandled_exception_filter(t.handler_on_unhandled_exception_filter_request_catch());

    t.ui_thread.unhandled_exception(t.handler_on_unhandled_exception_not_handled());
    t.ui_thread.unhandled_exception(t.handler_on_unhandled_exception_handled());
    let caught_correct_exception = t.post_and_run().unwrap_or(false);
    t.verification(caught_correct_exception, 0, 2);
    assert_eq!(t.number_of_handler_on_unhandled_event_invoked.get(), 0);
}

#[test]
fn multiple_unhandled_exception_cannot_reset_handle_flag() {
    let t = DispatcherTests::new();
    t.ui_thread.unhandled_exception_filter(t.handler_on_unhandled_exception_filter_request_catch());

    t.ui_thread.unhandled_exception(t.handler_on_unhandled_exception_handled());
    t.ui_thread.unhandled_exception(t.handler_on_unhandled_exception_not_handled());
    // should be no exception here.
    let caught_correct_exception = t.post_and_run().is_none();
    t.verification(caught_correct_exception, 1, 1);
    assert_eq!(t.number_of_handler_on_unhandled_event_invoked.get(), 2);
}

#[test]
fn can_push_frame_and_shutdown_dispatcher_from_unhandled_exception() {
    let t = DispatcherTests::new();
    t.ui_thread.unhandled_exception_filter(t.handler_on_unhandled_exception_filter_not_request_catch_push_frame());

    t.ui_thread.unhandled_exception(t.handler_on_unhandled_exception_handled_push_frame());
    let caught_correct_exception = t.post_and_run().unwrap_or(false);
    t.verification(caught_correct_exception, 0, 1);
}

#[test]
fn can_push_frame_and_shutdown_dispatcher_from_unhandled_exception_handler() {
    // Not in the reference suite: the handler variant of the test above,
    // where the handler pumps a nested frame until the shutdown it requested.
    let t = DispatcherTests::new();
    t.ui_thread.unhandled_exception_filter(t.handler_on_unhandled_exception_filter_request_catch());
    t.ui_thread.unhandled_exception(t.handler_on_unhandled_exception_handled_push_frame());
    let finished = Rc::new(Cell::new(false));
    let f = finished.clone();
    t.ui_thread.shutdown_finished(move |_| f.set(true));

    assert!(t.post_and_run().is_none());
    t.verification(true, 1, 1);
    assert!(finished.get());
}

#[test]
fn nested_frame_does_not_filter_the_same_exception_twice() {
    // A panic that escapes a nested frame passes through the operation that
    // pushed the frame; the dispatcher must not offer it to the handlers again.
    let t = DispatcherTests::new();
    t.ui_thread.unhandled_exception_filter(t.handler_on_unhandled_exception_filter_request_catch());
    t.ui_thread.unhandled_exception(t.handler_on_unhandled_exception_not_handled());

    let ui_thread = t.ui_thread.clone();
    t.ui_thread.post_local(
        move || {
            ui_thread.post(throw_an_exception, DispatcherPriority::NORMAL);
            ui_thread.run_jobs(None);
        },
        DispatcherPriority::NORMAL,
    );

    let ui_thread = t.ui_thread.clone();
    let result = catch_unwind(AssertUnwindSafe(|| ui_thread.run_jobs(None)));
    let payload = result.unwrap_err();
    assert_eq!(message_of(&*payload), Some(EXPECTED_EXCEPTION_TEXT));
    assert_eq!(t.number_of_handler_on_unhandled_event_filter_invoked.get(), 1);
    assert_eq!(t.number_of_handler_on_unhandled_event_invoked.get(), 1);

    // The bookkeeping is reset once the panic has left the dispatcher.
    assert!(t.post_and_run().unwrap_or(false));
    assert_eq!(t.number_of_handler_on_unhandled_event_filter_invoked.get(), 2);
}

#[test]
fn sync_context_exception_can_be_handled_with_post() {
    let t = DispatcherTests::new();
    let sync_context = t.ui_thread.get_context_with_priority(DispatcherPriority::BACKGROUND);

    let handled = Rc::new(Cell::new(false));
    let executed = Rc::new(Cell::new(false));
    let h = handled.clone();
    t.ui_thread.unhandled_exception(move |args| {
        h.set(true);
        args.set_handled(true);
    });

    sync_context.post(throw_an_exception);
    let e = executed.clone();
    sync_context.post_local(move || e.set(true));

    t.ui_thread.run_jobs_with_cancellation(None, &CancellationToken::none());

    assert!(handled.get());
    assert!(executed.get());
}
