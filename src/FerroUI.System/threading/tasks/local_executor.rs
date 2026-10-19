//! The running of futures on the thread that started them (docs/porting/reactive.md, section 5.8):
//! [`spawn_local`], the thread poster through which another thread wakes them, and the queue of
//! such wakes on a thread that has no poster.
//!
//! A future is polled when it is spawned, and again whenever it is woken: at once on its own
//! thread, or right after the poll that runs when the wake comes from inside it. A wake from
//! another thread goes into the queue of the task's thread and through that thread's poster, which
//! runs [`run_pending_wakes`] there; the dispatcher of the user interface is the poster of its
//! thread.

use super::task::{Task, TaskCanceledError};
use crate::ObservableError;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::task::{Context, Poll, Wake, Waker};
use std::thread::ThreadId;

/// Runs work on the thread it belongs to, from any thread (`SynchronizationContext.Post`).
pub type ThreadPoster = Arc<dyn Fn(Box<dyn FnOnce() + Send>) + Send + Sync>;

/// The wakes that came to a thread from other threads.
struct Inbox {
    queue: Mutex<Vec<u64>>,
    arrived: Condvar,
    poster: Mutex<Option<ThreadPoster>>,
}

impl Inbox {
    fn push(&self, task_id: u64) {
        self.queue.lock().unwrap_or_else(PoisonError::into_inner).push(task_id);
        self.arrived.notify_all();
        let poster = self.poster.lock().unwrap_or_else(PoisonError::into_inner).clone();
        if let Some(poster) = poster {
            poster(Box::new(|| {
                run_pending_wakes();
            }));
        }
    }

    fn take(&self) -> Vec<u64> {
        std::mem::take(&mut *self.queue.lock().unwrap_or_else(PoisonError::into_inner))
    }
}

/// A future running on its thread.
struct LocalTask {
    future: RefCell<Option<Pin<Box<dyn Future<Output = ()>>>>>,
    polling: Cell<bool>,
    repoll: Cell<bool>,
    waker: Waker,
}

crate::ferro_thread_local! {
    /// The futures of the thread, by the id of their task.
    static TASKS: RefCell<HashMap<u64, Rc<LocalTask>>> = RefCell::new(HashMap::new());
    /// The wakes from other threads.
    static INBOX: Arc<Inbox> = Arc::new(Inbox { queue: Mutex::new(Vec::new()), arrived: Condvar::new(), poster: Mutex::new(None) });
}

/// Wakes a future of a thread.
struct TaskWaker {
    task_id: u64,
    thread: ThreadId,
    inbox: Arc<Inbox>,
}

impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        if std::thread::current().id() == self.thread {
            poll_task(self.task_id);
        } else {
            self.inbox.push(self.task_id);
        }
    }
}

/// Polls the future of a task of the thread until it is pending without a wake from inside its
/// own poll.
fn poll_task(task_id: u64) {
    let Some(task) = TASKS.with_borrow(|tasks| tasks.get(&task_id).cloned()) else { return };
    if task.polling.get() {
        task.repoll.set(true);
        return;
    }
    loop {
        task.polling.set(true);
        task.repoll.set(false);
        let done = {
            let mut slot = task.future.borrow_mut();
            let Some(future) = slot.as_mut() else { break };
            let mut context = Context::from_waker(&task.waker);
            let done = future.as_mut().poll(&mut context).is_ready();
            if done {
                slot.take();
            }
            done
        };
        task.polling.set(false);
        if done {
            TASKS.with_borrow_mut(|tasks| tasks.remove(&task_id));
            break;
        }
        if !task.repoll.get() {
            break;
        }
    }
    task.polling.set(false);
}

/// Runs `future` on the current thread and gives its task: the output of the future ends it, and
/// an error that is a [`TaskCanceledError`] cancels it. The future is polled at once, and again
/// whenever it is woken (the module's documentation).
pub fn spawn_local<T: 'static>(future: impl Future<Output = Result<T, ObservableError>> + 'static) -> Task<T> {
    let task = Task::with_status(super::TaskStatus::Running);
    let target = task.clone();
    let id = task.id();
    let inbox = INBOX.with(Arc::clone);
    let waker = Waker::from(Arc::new(TaskWaker { task_id: id, thread: std::thread::current().id(), inbox }));
    let local = Rc::new(LocalTask {
        future: RefCell::new(Some(Box::pin(async move {
            let outcome = future.await;
            target.try_complete(outcome);
        }))),
        polling: Cell::new(false),
        repoll: Cell::new(false),
        waker,
    });
    TASKS.with_borrow_mut(|tasks| tasks.insert(id, local));
    poll_task(id);
    task
}

/// Makes `poster` the way other threads reach the current thread (the `SynchronizationContext` of
/// the thread): a wake of one of its futures from another thread is posted through it, and the
/// work it is given must run on this thread. `None` removes it.
pub fn set_thread_poster(poster: Option<ThreadPoster>) {
    let inbox = INBOX.with(Arc::clone);
    *inbox.poster.lock().unwrap_or_else(PoisonError::into_inner) = poster;
    if !inbox.queue.lock().unwrap_or_else(PoisonError::into_inner).is_empty() {
        run_pending_wakes();
    }
}

/// Polls the futures of the current thread that other threads woke, until none is left; true
/// when there was any.
pub fn run_pending_wakes() -> bool {
    let inbox = INBOX.with(Arc::clone);
    let mut ran = false;
    loop {
        let ids = inbox.take();
        if ids.is_empty() {
            return ran;
        }
        ran = true;
        for id in ids {
            poll_task(id);
        }
    }
}

/// Runs `future` on the current thread until it ends, waiting for wakes from other threads in
/// between (a thread with nothing else to run: a console program, a test).
///
/// # Panics
/// Never returns when nothing wakes the future again: a future that waits for this thread itself
/// (a scheduler of the thread, a task that only this thread ends) has to be ended by something
/// that runs before it waits.
pub fn block_on<T: Clone + 'static>(future: impl Future<Output = Result<T, ObservableError>> + 'static) -> Result<T, ObservableError> {
    let task = spawn_local(future);
    let inbox = INBOX.with(Arc::clone);
    loop {
        run_pending_wakes();
        if task.is_completed() {
            return task.result();
        }
        let queue = inbox.queue.lock().unwrap_or_else(PoisonError::into_inner);
        if queue.is_empty() {
            drop(inbox.arrived.wait(queue).unwrap_or_else(PoisonError::into_inner));
        }
    }
}

/// The error of work run by [`run`] that ended with an error or panicked: its message, since an
/// error crossing threads must be `Send`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskWorkError {
    pub message: String,
}

impl std::fmt::Display for TaskWorkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for TaskWorkError {}

/// Runs `work` on another thread and gives its task, which ends on the current thread
/// (`Task.Run`): through the poster of the thread, or the queue of [`run_pending_wakes`].
/// An `Err` of the work, or a panic, faults the task with a [`TaskWorkError`].
pub fn run<T: Send + 'static>(work: impl FnOnce() -> Result<T, Box<dyn std::error::Error + Send + Sync>> + Send + 'static) -> Task<T> {
    struct Slot<T> {
        outcome: Option<Result<T, String>>,
        waker: Option<Waker>,
    }
    let slot = Arc::new(Mutex::new(Slot { outcome: None, waker: None }));
    let worker_slot = slot.clone();
    std::thread::spawn(move || {
        let outcome = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(work)) {
            Ok(Ok(value)) => Ok(value),
            Ok(Err(error)) => Err(error.to_string()),
            Err(panic) => Err(panic
                .downcast_ref::<&str>()
                .map(|text| (*text).to_string())
                .or_else(|| panic.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "the work panicked".to_string())),
        };
        let waker = {
            let mut slot = worker_slot.lock().unwrap_or_else(PoisonError::into_inner);
            slot.outcome = Some(outcome);
            slot.waker.take()
        };
        if let Some(waker) = waker {
            waker.wake();
        }
    });
    spawn_local(std::future::poll_fn(move |context| {
        let mut slot = slot.lock().unwrap_or_else(PoisonError::into_inner);
        match slot.outcome.take() {
            Some(Ok(value)) => Poll::Ready(Ok(value)),
            Some(Err(message)) => Poll::Ready(Err(Rc::new(TaskWorkError { message }) as ObservableError)),
            None => {
                slot.waker = Some(context.waker().clone());
                Poll::Pending
            }
        }
    }))
}

/// A future that ends with a cancellation (`Task.FromCanceled` awaited): for code that stops
/// because its token was cancelled.
pub fn canceled<T>() -> Result<T, ObservableError> {
    Err(Rc::new(TaskCanceledError::default()))
}
