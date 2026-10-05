use ferroui_base::input::platform::{ClipboardError, ClipboardErrorKind};
use ferroui_base::threading::Dispatcher;
use std::any::Any;
use std::future::Future;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::pin::Pin;
use std::task::{Context, Poll};

/// What the clipboard operations of the text controls share.
pub(crate) struct ClipboardHelper;

impl ClipboardHelper {
    /// Whether a failed clipboard operation failed the way platform
    /// clipboards are known to: the operation timed out, was canceled, was
    /// denied or a call of the platform clipboard interface failed. Such a
    /// failure is logged by the controls; any other one is not theirs to
    /// handle.
    pub(crate) fn is_expected_clipboard_exception(exception: &ClipboardError) -> bool {
        matches!(
            exception.kind(),
            ClipboardErrorKind::Timeout
                | ClipboardErrorKind::Canceled
                | ClipboardErrorKind::AccessDenied
                | ClipboardErrorKind::Platform
        )
    }

    /// Starts `operation` on the dispatcher thread the way calling an
    /// asynchronous method without a result does: it runs up to its first
    /// pending await before this returns and is resumed from dispatcher
    /// jobs afterwards.
    ///
    /// Nobody observes its outcome. An error the operation resolves to is
    /// the failure it did not handle (the exception that leaves the method
    /// of the reference): it is raised from a dispatcher job with
    /// [`throw`](Self::throw), and so is a panic of the operation, instead
    /// of being kept in a task nobody looks at.
    pub(crate) fn start(operation: impl Future<Output = Result<(), ClipboardError>> + 'static) {
        let scheduler = Dispatcher::ui_thread().to_task_scheduler();
        let dispatcher = scheduler.dispatcher().clone();
        let priority = scheduler.priority();

        drop(scheduler.start_local(async move {
            match CatchUnwind::new(operation).await {
                Ok(Ok(())) => {}
                Ok(Err(error)) => dispatcher.post(move || Self::throw(error), priority),
                Err(payload) => dispatcher.post(move || resume_unwind(payload), priority),
            }
        }));
    }

    /// Raises a clipboard failure nothing handles: a panic whose message
    /// names the kind of the failure and its message.
    pub(crate) fn throw(error: ClipboardError) -> ! {
        panic!("{}", Self::unhandled_message(&error))
    }

    /// The message of the panic [`throw`](Self::throw) raises for `error`.
    pub(crate) fn unhandled_message(error: &ClipboardError) -> String {
        format!("Unhandled clipboard error ({:?}): {}", error.kind(), error)
    }

    /// The text of the payload of a panic, for a log message.
    pub(crate) fn describe_panic(payload: &(dyn Any + Send)) -> String {
        if let Some(message) = payload.downcast_ref::<&'static str>() {
            (*message).to_owned()
        } else if let Some(message) = payload.downcast_ref::<String>() {
            message.clone()
        } else {
            "the operation panicked".to_owned()
        }
    }
}

/// A future that resolves to the output of another one, or to the payload
/// of the panic the other one raised while it was polled.
pub(crate) struct CatchUnwind<T> {
    future: Option<Pin<Box<dyn Future<Output = T>>>>,
}

impl<T> CatchUnwind<T> {
    pub(crate) fn new(future: impl Future<Output = T> + 'static) -> Self {
        Self { future: Some(Box::pin(future)) }
    }
}

impl<T> Future for CatchUnwind<T> {
    type Output = Result<T, Box<dyn Any + Send>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let Some(future) = self.future.as_mut() else {
            panic!("The future was polled after it completed.");
        };

        match catch_unwind(AssertUnwindSafe(|| future.as_mut().poll(cx))) {
            Ok(Poll::Pending) => Poll::Pending,
            Ok(Poll::Ready(value)) => {
                self.future = None;
                Poll::Ready(Ok(value))
            }
            Err(payload) => {
                self.future = None;
                Poll::Ready(Err(payload))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::{catch_unwind, AssertUnwindSafe};

    #[test]
    fn the_failures_of_platform_clipboards_are_expected() {
        let expected = [
            ClipboardErrorKind::Timeout,
            ClipboardErrorKind::Canceled,
            ClipboardErrorKind::AccessDenied,
            ClipboardErrorKind::Platform,
        ];
        for kind in expected {
            assert!(ClipboardHelper::is_expected_clipboard_exception(&ClipboardError::from_kind(kind)), "{kind:?}");
        }

        assert!(!ClipboardHelper::is_expected_clipboard_exception(&ClipboardError::from_kind(
            ClipboardErrorKind::Other
        )));
    }

    #[test]
    fn an_unhandled_failure_is_raised_from_the_dispatcher() {
        let _scope = Dispatcher::unit_test_scope();
        let error = ClipboardError::other("Operation is not valid.");

        let failed = error.clone();
        ClipboardHelper::start(async move { Err(failed) });

        let payload = catch_unwind(AssertUnwindSafe(|| Dispatcher::ui_thread().run_jobs(None)))
            .expect_err("the failure of the operation");
        assert_eq!(
            Some("Unhandled clipboard error (Other): Operation is not valid."),
            payload.downcast_ref::<String>().map(String::as_str)
        );
        assert_eq!(Some(&ClipboardHelper::unhandled_message(&error)), payload.downcast_ref::<String>());
    }

    #[test]
    fn a_panic_of_the_operation_is_raised_from_the_dispatcher() {
        let _scope = Dispatcher::unit_test_scope();

        ClipboardHelper::start(async { panic!("The clipboard was disposed.") });

        let payload = catch_unwind(AssertUnwindSafe(|| Dispatcher::ui_thread().run_jobs(None)))
            .expect_err("the panic of the operation");
        assert_eq!(Some(&"The clipboard was disposed."), payload.downcast_ref::<&'static str>());
    }

    #[test]
    fn a_successful_operation_leaves_nothing_behind() {
        let _scope = Dispatcher::unit_test_scope();

        ClipboardHelper::start(async { Ok(()) });

        assert!(catch_unwind(AssertUnwindSafe(|| Dispatcher::ui_thread().run_jobs(None))).is_ok());
    }
}
