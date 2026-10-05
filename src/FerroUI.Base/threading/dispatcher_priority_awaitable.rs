use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};

use super::dispatcher_task::current_task_of;
use super::{Dispatcher, DispatcherPriority};

/// Resumes the awaiting code from a dispatcher job of a given priority.
struct Resumption {
    dispatcher: Arc<Dispatcher>,
    priority: DispatcherPriority,
    posted: bool,
    resumed: Arc<AtomicBool>,
}

impl Resumption {
    fn new(dispatcher: Arc<Dispatcher>, priority: DispatcherPriority) -> Self {
        Self { dispatcher, priority, posted: false, resumed: Arc::new(AtomicBool::new(false)) }
    }

    /// Never ready the first time it is polled.
    fn poll(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        if self.resumed.load(Ordering::SeqCst) {
            return Poll::Ready(());
        }
        if !self.posted {
            self.posted = true;
            let resumed = self.resumed.clone();
            let waker = cx.waker().clone();
            // When the awaiting code is a task of this dispatcher, the job
            // polls it directly, so that it continues inside the job of the
            // requested priority. Any other executor is woken up instead.
            let task = current_task_of(&self.dispatcher).map(|task_id| (self.dispatcher.clone(), task_id));
            self.dispatcher.post(
                move || {
                    resumed.store(true, Ordering::SeqCst);
                    let polled = match &task {
                        Some((dispatcher, task_id)) => dispatcher.poll_task(*task_id),
                        None => false,
                    };
                    if !polled {
                        waker.wake();
                    }
                },
                self.priority,
            );
        }
        Poll::Pending
    }
}

/// A future that completes after the dispatcher has processed a job at the
/// requested priority, which resumes the awaiting code from the dispatcher
/// thread. Created by [`Dispatcher::resume`] and [`Dispatcher::yield_now`].
///
/// Like its counterpart in the reference implementation it is never
/// complete when first awaited.
pub struct DispatcherPriorityAwaitable {
    resumption: Resumption,
}

impl DispatcherPriorityAwaitable {
    pub(crate) fn new(dispatcher: Arc<Dispatcher>, priority: DispatcherPriority) -> Self {
        Self { resumption: Resumption::new(dispatcher, priority) }
    }
}

impl Future for DispatcherPriorityAwaitable {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        self.resumption.poll(cx)
    }
}

/// A future that awaits another future and then resumes the awaiting code
/// from a dispatcher job of the requested priority, yielding the inner
/// future's output. Created by [`Dispatcher::await_with_priority`].
///
/// The awaiting code is resumed through the dispatcher even when the inner
/// future is already complete.
pub struct DispatcherPriorityTaskAwaitable<F: Future> {
    task: Pin<Box<F>>,
    output: Option<F::Output>,
    resumption: Resumption,
}

// The inner future is boxed and the output is never pinned.
impl<F: Future> Unpin for DispatcherPriorityTaskAwaitable<F> {}

impl<F: Future> DispatcherPriorityTaskAwaitable<F> {
    pub(crate) fn new(dispatcher: Arc<Dispatcher>, task: F, priority: DispatcherPriority) -> Self {
        Self { task: Box::pin(task), output: None, resumption: Resumption::new(dispatcher, priority) }
    }
}

impl<F: Future> Future for DispatcherPriorityTaskAwaitable<F> {
    type Output = F::Output;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<F::Output> {
        let this = &mut *self;
        if this.output.is_none() && !this.resumption.posted {
            match this.task.as_mut().poll(cx) {
                Poll::Ready(output) => this.output = Some(output),
                Poll::Pending => return Poll::Pending,
            }
        }
        match this.resumption.poll(cx) {
            Poll::Ready(()) => match this.output.take() {
                Some(output) => Poll::Ready(output),
                None => panic!("DispatcherPriorityTaskAwaitable polled after completion"),
            },
            Poll::Pending => Poll::Pending,
        }
    }
}
