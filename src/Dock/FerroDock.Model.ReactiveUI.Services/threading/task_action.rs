// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use ferroui_system::threading::tasks::{Task, TaskWorkError};
use ferroui_system::ObservableError;
use std::any::Any;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

// Task-returning C# async methods capture synchronous exceptions in their task.
// A returned native task error retains its original shared Error; a Rust panic
// uses the shared runtime's standard TaskWorkError diagnostic representation.
pub(crate) fn panic_error(payload: Box<dyn Any + Send>) -> ObservableError {
    let message = payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| {
            payload
                .downcast_ref::<&str>()
                .map(|text| (*text).to_owned())
        })
        .unwrap_or_else(|| "Asynchronous operation panicked.".into());
    Rc::new(TaskWorkError { message })
}
pub(crate) fn invoke_task(action: impl FnOnce() -> Task<()>) -> Task<()> {
    catch_unwind(AssertUnwindSafe(action))
        .unwrap_or_else(|payload| Task::from_exception(panic_error(payload)))
}

pub(crate) fn spawn_catching<T: 'static>(
    future: impl std::future::Future<Output = Result<T, ObservableError>> + 'static,
) -> Task<T> {
    struct Catch<F> {
        future: std::pin::Pin<Box<F>>,
    }
    impl<T, F: std::future::Future<Output = Result<T, ObservableError>>> std::future::Future
        for Catch<F>
    {
        type Output = Result<T, ObservableError>;
        fn poll(
            mut self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Self::Output> {
            match catch_unwind(AssertUnwindSafe(|| self.future.as_mut().poll(cx))) {
                Ok(result) => result,
                Err(payload) => std::task::Poll::Ready(Err(panic_error(payload))),
            }
        }
    }
    Task::spawn_local(Catch {
        future: Box::pin(future),
    })
}
