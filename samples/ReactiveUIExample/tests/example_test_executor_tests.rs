//! Port of `ExampleTestExecutorTests.cs`: guards against reporting success before a dispatched
//! asynchronous assertion completes.
//!
//! The reference checks that its test executor propagates a failure that happens after an
//! asynchronous yield.
//!
//! Deviation (DEVIATIONS.md, samples): the tests of the port have no executor: a test awaits on its
//! own thread with [`await_task`], which this test checks the same way.

use super::support::{await_task, start_application};
use crate::services::InvalidOperationError;
use ferroui_system::threading::tasks::Task;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

/// A future that is pending once, then ready (`Task.Yield()`).
struct YieldOnce {
    yielded: bool,
}

impl Future for YieldOnce {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<()> {
        if self.yielded {
            return Poll::Ready(());
        }
        self.yielded = true;
        context.waker().wake_by_ref();
        Poll::Pending
    }
}

#[test]
fn headless_executor_propagates_failure_after_async_yield() {
    let _app = start_application();
    let task: Task<()> = Task::spawn_local(async {
        YieldOnce { yielded: false }.await;
        Err(InvalidOperationError::new("Expected asynchronous test failure.").into_error())
    });

    let error = await_task(&task).expect_err("the failure propagates");
    assert!(error.downcast_ref::<InvalidOperationError>().is_some(), "{error}");
}
