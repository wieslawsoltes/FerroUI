//! The helpers of the tests in place of the base class library: the `InvalidOperationException`
//! the tests throw, `Task.Delay` and `Task.Yield`. Not an upstream file.

use ferroui_system::threading::tasks::{run, Task};
use ferroui_system::ObservableError;
use std::error::Error;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};
use std::time::Duration;

/// The `InvalidOperationException` the tests throw: an error with a message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvalidOperationException {
    /// `Message`.
    pub message: String,
}

impl InvalidOperationException {
    /// The error with `message`, as the error of a task.
    pub fn error(message: &str) -> ObservableError {
        Rc::new(Self { message: message.to_owned() })
    }
}

impl fmt::Display for InvalidOperationException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for InvalidOperationException {}

/// Whether `error` is an [`InvalidOperationException`] (`Throws<InvalidOperationException>()`).
pub fn is_invalid_operation(error: &ObservableError) -> bool {
    error.downcast_ref::<InvalidOperationException>().is_some()
}

/// A task that ends after `duration` (`Task.Delay`): the time passes on a worker thread, and the
/// task ends on the current thread.
pub fn delay(duration: Duration) -> Task<()> {
    run(move || {
        std::thread::sleep(duration);
        Ok(())
    })
}

/// A future that is pending once and wakes itself (`Task.Yield()`).
pub fn yield_now() -> impl Future<Output = Result<(), ObservableError>> {
    struct YieldNow {
        yielded: bool,
    }

    impl Future for YieldNow {
        type Output = Result<(), ObservableError>;

        fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
            if self.yielded {
                return Poll::Ready(Ok(()));
            }
            self.yielded = true;
            context.waker().wake_by_ref();
            Poll::Pending
        }
    }

    YieldNow { yielded: false }
}
