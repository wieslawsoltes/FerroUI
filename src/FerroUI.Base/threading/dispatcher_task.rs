//! Futures running on a dispatcher.
//!
//! This is the executor half of what the reference implementation gets from
//! its synchronization context: a future is stored on the dispatcher thread
//! and polled from dispatcher jobs; its waker queues the next poll at the
//! priority the future was started with. [`DispatcherTask`] is the handle to
//! the outcome, the counterpart of a task object.

use std::any::{Any, TypeId};
use std::cell::Cell;
use std::fmt;
use std::future::Future;
use std::marker::PhantomData;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::task::{Context, Poll, Wake, Waker};

use super::dispatcher_operation::{next_operation_id, DispatcherException};
use super::{Dispatcher, DispatcherPriority, OperationCanceledError};

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

    fn take_outcome<T: 'static>(&self) -> Result<T, OperationCanceledError> {
        let (status, panic, result) = {
            let mut state = self.lock();
            (state.status, state.panic.take(), state.result.take())
        };
        match status {
            TaskStatus::Canceled => return Err(OperationCanceledError),
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
pub(crate) struct TaskCompletion(Arc<TaskShared>);

impl TaskCompletion {
    pub(crate) fn fail(&self, payload: DispatcherException) {
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
/// `Err(OperationCanceledError)` when it was dropped before completing (the
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
    pub fn wait(&self) -> Result<(), OperationCanceledError> {
        self.block_until_finished();
        let (status, panic) = {
            let mut state = self.shared.lock();
            (state.status, state.panic.take())
        };
        if let Some(payload) = panic {
            resume_unwind(payload);
        }
        match status {
            TaskStatus::Canceled => Err(OperationCanceledError),
            _ => Ok(()),
        }
    }

    /// Blocks like [`wait`](Self::wait) and takes the result.
    ///
    /// # Panics
    /// See [`wait`](Self::wait). Also panics when the result has already been
    /// taken, and when a result that is not `Send` is requested from another
    /// thread.
    pub fn result(&self) -> Result<T, OperationCanceledError> {
        self.block_until_finished();
        self.shared.take_outcome()
    }
}

impl<T: 'static> Future for DispatcherTask<T> {
    type Output = Result<T, OperationCanceledError>;

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

/// Queues polls of one task at the task's priority.
pub(crate) struct TaskWaker {
    dispatcher: Arc<Dispatcher>,
    priority: DispatcherPriority,
    task_id: u64,
    scheduled: AtomicBool,
}

impl TaskWaker {
    fn schedule(self: &Arc<Self>) {
        if !self.scheduled.swap(true, Ordering::SeqCst) {
            let this = self.clone();
            self.dispatcher.post(
                move || {
                    this.dispatcher.poll_task(this.task_id);
                },
                self.priority,
            );
        }
    }
}

impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        self.schedule();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.schedule();
    }
}

/// A future stored on the dispatcher thread.
pub(crate) struct LocalTask {
    future: Pin<Box<dyn Future<Output = ()>>>,
    waker: Waker,
    wake_state: Arc<TaskWaker>,
    completion: TaskCompletion,
}

thread_local! {
    /// The (dispatcher id, task id) of the task being polled on this thread.
    static CURRENT_TASK: Cell<Option<(u64, u64)>> = const { Cell::new(None) };
}

/// The task of `dispatcher` that is being polled right now, if any.
pub(crate) fn current_task_of(dispatcher: &Dispatcher) -> Option<u64> {
    match CURRENT_TASK.try_with(Cell::get).ok().flatten() {
        Some((dispatcher_id, task_id)) if dispatcher_id == dispatcher.id => Some(task_id),
        _ => None,
    }
}

/// Creates the handle of a task whose result can cross threads, and the
/// future that produces it.
pub(crate) fn wrap_send_future<Fut>(
    dispatcher: &Arc<Dispatcher>,
) -> (DispatcherTask<Fut::Output>, TaskCompletion, impl FnOnce(Fut) -> Pin<Box<dyn Future<Output = ()>>>)
where
    Fut: Future + 'static,
    Fut::Output: Send + 'static,
{
    let shared = TaskShared::new(dispatcher, false);
    let task = DispatcherTask { shared: shared.clone(), _result: PhantomData };
    let completion = TaskCompletion(shared.clone());
    let wrap = move |future: Fut| -> Pin<Box<dyn Future<Output = ()>>> {
        Box::pin(async move {
            let value = future.await;
            shared.complete_send(Box::new(value));
        })
    };
    (task, completion, wrap)
}

/// Creates the handle of a task whose result stays on the dispatcher thread,
/// and the future that produces it.
pub(crate) fn wrap_local_future<Fut>(
    dispatcher: &Arc<Dispatcher>,
) -> (DispatcherTask<Fut::Output>, TaskCompletion, impl FnOnce(Fut) -> Pin<Box<dyn Future<Output = ()>>>)
where
    Fut: Future + 'static,
    Fut::Output: 'static,
{
    let shared = TaskShared::new(dispatcher, true);
    let task = DispatcherTask { shared: shared.clone(), _result: PhantomData };
    let completion = TaskCompletion(shared.clone());
    let wrap = move |future: Fut| -> Pin<Box<dyn Future<Output = ()>>> {
        Box::pin(async move {
            let value = future.await;
            if TypeId::of::<Fut::Output>() == TypeId::of::<()>() {
                shared.complete_local(None);
            } else {
                shared.complete_local(Some(Box::new(value)));
            }
        })
    };
    (task, completion, wrap)
}

impl Dispatcher {
    /// Stores `future` on this thread and polls it from jobs queued at
    /// `priority`. With `poll_now` the first poll happens before returning,
    /// otherwise it is queued.
    ///
    /// Dispatcher thread only.
    pub(crate) fn start_task(
        &self,
        priority: DispatcherPriority,
        future: Pin<Box<dyn Future<Output = ()>>>,
        completion: TaskCompletion,
        poll_now: bool,
    ) {
        let Some(local) = self.try_local() else {
            // Dropping the completion cancels the task.
            return;
        };
        if self.lock().has_shutdown_finished {
            return;
        }
        let task_id = completion.0.id;
        let wake_state =
            Arc::new(TaskWaker { dispatcher: self.to_arc(), priority, task_id, scheduled: AtomicBool::new(false) });
        let task = LocalTask { future, waker: Waker::from(wake_state.clone()), wake_state: wake_state.clone(), completion };
        local.tasks.borrow_mut().insert(task_id, task);
        if poll_now {
            self.poll_task(task_id);
        } else {
            wake_state.schedule();
        }
    }

    /// Polls the task once. Returns `false` when the task is not waiting to
    /// be polled: it has finished, or it is the one being polled right now.
    ///
    /// Dispatcher thread only.
    pub(crate) fn poll_task(&self, task_id: u64) -> bool {
        let Some(local) = self.try_local() else {
            return false;
        };
        let task = local.tasks.borrow_mut().remove(&task_id);
        let Some(mut task) = task else {
            return false;
        };
        task.wake_state.scheduled.store(false, Ordering::SeqCst);

        let previous = CURRENT_TASK.with(|current| current.replace(Some((self.id, task_id))));
        let result = catch_unwind(AssertUnwindSafe(|| {
            let mut cx = Context::from_waker(&task.waker);
            task.future.as_mut().poll(&mut cx)
        }));
        CURRENT_TASK.with(|current| current.set(previous));

        match result {
            Ok(Poll::Pending) => {
                if self.lock().has_shutdown_finished {
                    // Nothing can poll it again.
                    drop(task);
                } else {
                    local.tasks.borrow_mut().insert(task_id, task);
                }
            }
            Ok(Poll::Ready(())) => drop(task),
            Err(payload) => {
                task.completion.fail(payload);
                drop(task);
            }
        }
        true
    }

    /// Drops every future running on this dispatcher, which completes their
    /// tasks as canceled.
    pub(crate) fn cancel_tasks(&self) {
        if let Some(local) = self.try_local() {
            let tasks = std::mem::take(&mut *local.tasks.borrow_mut());
            // Dropped outside of the borrow: futures run arbitrary code when dropped.
            drop(tasks);
        }
    }
}
