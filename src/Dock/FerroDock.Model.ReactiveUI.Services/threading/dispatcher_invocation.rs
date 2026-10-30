// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use ferroui_system::threading::tasks::Task;
use ferroui_system::ObservableError;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

// All result/status storage remains the real System task. Only its await wake
// is adapted: an await registered while pending must not resume inline inside
// InvokeAsync's completion, equivalent to RunContinuationsAsynchronously.
// The Send Waker crosses the worker boundary, never a thread-affine Rc/value.
// Its System executor forwards the wake to its owner's queue/poster.
#[derive(Clone)]
pub(crate) struct DispatcherInvocation {
    task: Task<()>,
    source_waker: Option<Waker>,
    async_waker: Option<Waker>,
}
impl DispatcherInvocation {
    pub(crate) fn new(task: Task<()>) -> Self {
        Self {
            task,
            source_waker: None,
            async_waker: None,
        }
    }
}
struct AsyncWake(Waker);
impl Wake for AsyncWake {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }
    fn wake_by_ref(self: &Arc<Self>) {
        let waker = self.0.clone();
        let _ = std::thread::spawn(move || waker.wake());
    }
}
impl Future for DispatcherInvocation {
    type Output = Result<(), ObservableError>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.task.is_completed() {
            return Poll::Ready(self.task.result());
        }
        if self
            .source_waker
            .as_ref()
            .is_none_or(|old| !old.will_wake(cx.waker()))
        {
            self.source_waker = Some(cx.waker().clone());
            self.async_waker = Some(Waker::from(Arc::new(AsyncWake(cx.waker().clone()))));
        }
        let waker = self
            .async_waker
            .as_ref()
            .expect("cached asynchronous wake")
            .clone();
        Pin::new(&mut self.task).poll(&mut Context::from_waker(&waker))
    }
}
