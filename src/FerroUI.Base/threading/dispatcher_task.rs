//! [`DispatcherTask`]: the outcome of a future running on a dispatcher, the
//! counterpart of a task object of the reference implementation.
//!
//! The future runs as a task of the dispatcher (`dispatcher_spawn`: the
//! executor of `ferroui-system`, polled from dispatcher jobs at the priority
//! the future was started with); [`DispatcherTask`] keeps the outcome for its
//! observers, with the contract it had before the executor moved: clones,
//! blocking waits on other threads, a panic of the future raised again to
//! whoever observes it, a result taken once.

use std::any::{Any, TypeId};
use std::fmt;
use std::future::Future;
use std::marker::PhantomData;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::task::{Context, Poll, Waker};

use super::dispatcher_operation::{next_operation_id, DispatcherException};
use super::Dispatcher;
use ferroui_system::task::Canceled;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TaskStatus {
    Running,
    Completed,
    Faulted,
    Canceled,
}

struct TaskState {
    status: TaskStatus,
    result: Option<Box<dyn Any + Send>>,
    panic: Option<DispatcherException>,
    wakers: Vec<Waker>,
}

/// The shareable part of a task: its outcome and whoever waits for it.
pub(crate) struct TaskShared {
    id: u64,
    dispatcher: Arc<Dispatcher>,
    /// The result is kept in the dispatcher thread's local storage.
    local_result: bool,
    local_storage_used: AtomicBool,
    state: Mutex<TaskState>,
    finished: Condvar,
}

impl TaskShared {
    pub(crate) fn new(dispatcher: &Arc<Dispatcher>, local_result: bool) -> Arc<Self> {
        Arc::new(Self {
            id: next_operation_id(),
            dispatcher: dispatcher.clone(),
            local_result,
            local_storage_used: AtomicBool::new(false),
            state: Mutex::new(TaskState { status: TaskStatus::Running, result: None, panic: None, wakers: Vec::new() }),
            finished: Condvar::new(),
        })
    }

    fn lock(&self) -> MutexGuard<'_, TaskState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn finish(&self, update: impl FnOnce(&mut TaskState)) -> bool {
        let wakers = {
            let mut state = self.lock();
            if state.status != TaskStatus::Running {
                return false;
            }
            update(&mut state);
            self.finished.notify_all();
            std::mem::take(&mut state.wakers)
        };
        for waker in wakers {
            waker.wake();
        }
        true
    }

    fn complete_send(&self, value: Box<dyn Any + Send>) {
        self.finish(|state| {
            state.status = TaskStatus::Completed;
            state.result = Some(value);
        });
    }

    /// Dispatcher thread only. `None` stands for `()`.
    fn complete_local(&self, value: Option<Box<dyn Any>>) {
        if let (Some(value), Some(local)) = (value, self.dispatcher.try_local()) {
            local.results.borrow_mut().insert(self.id, value);
            self.local_storage_used.store(true, Ordering::SeqCst);
        }
        self.finish(|state| state.status = TaskStatus::Completed);
    }

    fn fail(&self, payload: DispatcherException) {
        self.finish(|state| {
            state.status = TaskStatus::Faulted;
            state.panic = Some(payload);
        });
    }

    fn cancel(&self) {
        self.finish(|state| state.status = TaskStatus::Canceled);
    }

    fn is_finished(&self) -> bool {
        self.lock().status != TaskStatus::Running
    }

    fn take_outcome<T: 'static>(&self) -> Result<T, Canceled> {
        let (status, panic, result) = {
            let mut state = self.lock();
            (state.status, state.panic.take(), state.result.take())
        };
        match status {
            TaskStatus::Canceled => return Err(Canceled),
            TaskStatus::Faulted => match panic {
                Some(payload) => resume_unwind(payload),
                None => panic!("The failure of the dispatcher task has already been observed."),
            },
            TaskStatus::Completed | TaskStatus::Running => {}
        }

        let value: Option<Box<dyn Any>> = if !self.local_result {
            result.map(|value| value as Box<dyn Any>)
        } else if TypeId::of::<T>() == TypeId::of::<()>() {
            Some(Box::new(()))
        } else {
            if !self.dispatcher.check_access() {
                panic!("The result of a local dispatcher task can only be read on the dispatcher thread.");
            }
            self.local_storage_used.store(false, Ordering::SeqCst);
            self.dispatcher.try_local().and_then(|local| local.results.borrow_mut().remove(&self.id))
        };

        match value.and_then(|value| value.downcast::<T>().ok()) {
            Some(value) => Ok(*value),
            None => panic!("The result of the dispatcher task has already been taken."),
        }
    }
}

impl Drop for TaskShared {
    fn drop(&mut self) {
        if self.local_storage_used.swap(false, Ordering::SeqCst) {
            self.dispatcher.discard_local_operation(self.id);
        }
    }
}

/// Completes the task as canceled when the code that was going to produce
/// its outcome is dropped without having run to the end.
struct TaskCompletion(Arc<TaskShared>);

impl TaskCompletion {
    fn fail(&self, payload: DispatcherException) {
        self.0.fail(payload);
    }
}

impl Drop for TaskCompletion {
    fn drop(&mut self) {
        self.0.cancel();
    }
}

/// The outcome of a future running on a [`Dispatcher`]; what the reference
/// implementation returns as a task object.
///
/// The handle can be cloned and sent to other threads. It is a [`Future`]
/// that resolves to `Ok(result)` when the future has completed and to
/// `Err(Canceled)` when it was dropped before completing (the
/// dispatcher shut down, or the operation that was going to start it was
/// aborted). A panic raised by the future is re-raised to whoever observes
/// the outcome.
pub struct DispatcherTask<T = ()> {
    shared: Arc<TaskShared>,
    _result: PhantomData<fn() -> T>,
}

impl<T> Clone for DispatcherTask<T> {
    fn clone(&self) -> Self {
        Self { shared: self.shared.clone(), _result: PhantomData }
    }
}

impl<T> fmt::Debug for DispatcherTask<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DispatcherTask #{} {:?}", self.shared.id, self.shared.lock().status)
    }
}

impl<T: 'static> DispatcherTask<T> {
    /// The dispatcher the future runs on.
    pub fn dispatcher(&self) -> &Arc<Dispatcher> {
        &self.shared.dispatcher
    }

    /// Whether the task has finished: completed, failed or canceled.
    pub fn is_completed(&self) -> bool {
        self.shared.is_finished()
    }

    /// Whether the task ran to the end without panicking.
    pub fn is_completed_successfully(&self) -> bool {
        self.shared.lock().status == TaskStatus::Completed
    }

    /// Whether the future panicked.
    pub fn is_faulted(&self) -> bool {
        self.shared.lock().status == TaskStatus::Faulted
    }

    /// Whether the future was dropped before it completed.
    pub fn is_canceled(&self) -> bool {
        self.shared.lock().status == TaskStatus::Canceled
    }

    fn block_until_finished(&self) {
        if self.shared.is_finished() {
            return;
        }
        if self.shared.dispatcher.check_access() {
            // The dispatcher thread is the one that has to make progress.
            panic!("Synchronous wait is only supported on non-UI threads");
        }
        let mut state = self.shared.lock();
        while state.status == TaskStatus::Running {
            state = self.shared.finished.wait(state).unwrap_or_else(PoisonError::into_inner);
        }
    }

    /// Blocks until the task has finished. Only supported on threads other
    /// than the dispatcher thread, unless the task has already finished.
    ///
    /// # Panics
    /// Re-raises the panic of the future. Panics when the task is still
    /// running and the calling thread is the dispatcher thread.
    pub fn wait(&self) -> Result<(), Canceled> {
        self.block_until_finished();
        let (status, panic) = {
            let mut state = self.shared.lock();
            (state.status, state.panic.take())
        };
        if let Some(payload) = panic {
            resume_unwind(payload);
        }
        match status {
            TaskStatus::Canceled => Err(Canceled),
            _ => Ok(()),
        }
    }

    /// Blocks like [`wait`](Self::wait) and takes the result.
    ///
    /// # Panics
    /// See [`wait`](Self::wait). Also panics when the result has already been
    /// taken, and when a result that is not `Send` is requested from another
    /// thread.
    pub fn result(&self) -> Result<T, Canceled> {
        self.block_until_finished();
        self.shared.take_outcome()
    }
}

impl<T: 'static> Future for DispatcherTask<T> {
    type Output = Result<T, Canceled>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        {
            let mut state = self.shared.lock();
            if state.status == TaskStatus::Running {
                if !state.wakers.iter().any(|waker| waker.will_wake(cx.waker())) {
                    state.wakers.push(cx.waker().clone());
                }
                return Poll::Pending;
            }
        }
        Poll::Ready(self.shared.take_outcome())
    }
}

/// The producer side of a [`DispatcherTask`] before its future runs: dropped
/// unused, it cancels the task.
pub(crate) struct TaskStart<T> {
    completion: TaskCompletion,
    complete: fn(&TaskShared, T),
}

impl<T: 'static> TaskStart<T> {
    /// The future of the task: runs `future`, writes its output to the task,
    /// and faults the task with a panic of the future.
    pub(crate) fn body<F: Future<Output = T> + 'static>(self, future: F) -> TaskBody<F> {
        TaskBody { future: Some(Box::pin(future)), start: Some(self) }
    }

    /// Runs `callback`, and gives the [`body`](Self::body) of the future it
    /// returns; a panic of the callback faults the task, whose body is then
    /// a future that is ready at once.
    pub(crate) fn run<F: Future<Output = T> + 'static>(self, callback: impl FnOnce() -> F) -> TaskBody<F> {
        match catch_unwind(AssertUnwindSafe(callback)) {
            Ok(future) => self.body(future),
            Err(payload) => {
                self.completion.fail(payload);
                TaskBody { future: None, start: None }
            }
        }
    }
}

/// The future of a [`DispatcherTask`] (see [`TaskStart::body`]). Dropped
/// before it is done (the dispatcher aborts its tasks when it shuts down),
/// it drops the future and then cancels the task.
pub(crate) struct TaskBody<F: Future> {
    future: Option<Pin<Box<F>>>,
    start: Option<TaskStart<F::Output>>,
}

impl<F: Future + 'static> Future for TaskBody<F>
where
    F::Output: 'static,
{
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let this = &mut *self;
        let Some(future) = this.future.as_mut() else {
            return Poll::Ready(());
        };
        let start = &mut this.start;
        let polled = catch_unwind(AssertUnwindSafe(|| match future.as_mut().poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(value) => {
                if let Some(start) = start.take() {
                    (start.complete)(&start.completion.0, value);
                }
                Poll::Ready(())
            }
        }));
        match polled {
            Ok(Poll::Pending) => Poll::Pending,
            Ok(Poll::Ready(())) => {
                this.future = None;
                Poll::Ready(())
            }
            Err(payload) => {
                // The future panicked (or what its completion woke did, which
                // the finished task ignores).
                if let Some(start) = this.start.take() {
                    start.completion.fail(payload);
                }
                this.future = None;
                Poll::Ready(())
            }
        }
    }
}

/// Creates a task whose result can cross threads, and its producer side.
pub(crate) fn new_send_task<T: Send + 'static>(dispatcher: &Arc<Dispatcher>) -> (DispatcherTask<T>, TaskStart<T>) {
    let shared = TaskShared::new(dispatcher, false);
    let task = DispatcherTask { shared: shared.clone(), _result: PhantomData };
    let complete: fn(&TaskShared, T) = |shared, value| shared.complete_send(Box::new(value));
    (task, TaskStart { completion: TaskCompletion(shared), complete })
}

/// Creates a task whose result stays on the dispatcher thread, and its
/// producer side.
pub(crate) fn new_local_task<T: 'static>(dispatcher: &Arc<Dispatcher>) -> (DispatcherTask<T>, TaskStart<T>) {
    let shared = TaskShared::new(dispatcher, true);
    let task = DispatcherTask { shared: shared.clone(), _result: PhantomData };
    let complete: fn(&TaskShared, T) = |shared, value| {
        if TypeId::of::<T>() == TypeId::of::<()>() {
            shared.complete_local(None);
        } else {
            shared.complete_local(Some(Box::new(value)));
        }
    };
    (task, TaskStart { completion: TaskCompletion(shared), complete })
}
