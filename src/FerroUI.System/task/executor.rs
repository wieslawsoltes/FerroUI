//! The task table, the poll loop and the queue of a thread (the module's documentation).

use super::join::{AbortHandle, Id, JoinCell, JoinError, JoinHandle};
use super::schedule::{FirstPoll, Schedule};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::future::Future;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::pin::Pin;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock, PoisonError};
use std::task::{Context, Poll, Wake, Waker};
use std::thread::ThreadId;

/// How a task ended without the output of its future: what its spawner is told.
pub(crate) enum Failure {
    /// The task was aborted, and its future dropped.
    Canceled,
    /// The future panicked: the payload of the panic.
    Panicked(Box<dyn Any + Send>),
}

/// The text of a panic payload: the message of `panic!` with a literal or a format, or a generic
/// text for any other payload.
pub(crate) fn panic_message(payload: &(dyn Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|text| (*text).to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "a task panicked".to_string())
}

pub(crate) type LocalFuture = Pin<Box<dyn Future<Output = ()>>>;
pub(crate) type OnFailure = Box<dyn FnOnce(Failure)>;
pub(crate) type LocalCallbackFn = Box<dyn FnOnce()>;

/// What another thread asks of a thread: to poll a task of its queue, or to run a callback that
/// was registered there.
#[derive(Clone, Copy)]
enum Pending {
    Task(u64),
    Callback(u64),
}

/// The queue of a thread: the wakes and callbacks that came from other threads, and the schedule
/// of the thread that is told of them.
pub(crate) struct Inbox {
    queue: Mutex<Vec<Pending>>,
    arrived: Condvar,
    schedule: Mutex<Option<Arc<dyn Schedule>>>,
}

impl Inbox {
    fn new() -> Self {
        Self { queue: Mutex::new(Vec::new()), arrived: Condvar::new(), schedule: Mutex::new(None) }
    }

    fn push(&self, pending: Pending) {
        self.queue.lock().unwrap_or_else(PoisonError::into_inner).push(pending);
        self.arrived.notify_all();
        let schedule = self.schedule.lock().unwrap_or_else(PoisonError::into_inner).clone();
        if let Some(schedule) = schedule {
            schedule.schedule(Box::new(|| {
                run_pending();
            }));
        }
    }

    fn take(&self) -> Vec<Pending> {
        std::mem::take(&mut *self.queue.lock().unwrap_or_else(PoisonError::into_inner))
    }

    fn is_empty(&self) -> bool {
        self.queue.lock().unwrap_or_else(PoisonError::into_inner).is_empty()
    }

    /// Waits until something is in the queue.
    fn wait(&self) {
        let queue = self.queue.lock().unwrap_or_else(PoisonError::into_inner);
        if queue.is_empty() {
            drop(self.arrived.wait(queue).unwrap_or_else(PoisonError::into_inner));
        }
    }
}

/// Where the wakes of a task go.
enum Route {
    /// The queue of the task's thread: a wake on that thread polls at once, one from another
    /// thread waits in the inbox.
    Thread(Arc<Inbox>),
    /// Every wake queues a poll on the schedule.
    Schedule(Arc<dyn Schedule>),
}

/// The part of a task that any thread reaches: its waker, and the abort of its handles.
pub(crate) struct Header {
    id: u64,
    /// The thread of the task: the spawning thread, or for [`spawn_on`] the thread where the
    /// schedule runs the first poll (unset until then).
    thread: OnceLock<ThreadId>,
    route: Route,
    /// A poll is queued on the schedule and has not started (one at a time).
    scheduled: AtomicBool,
    /// The task was aborted; its future is dropped on its thread.
    aborted: AtomicBool,
    /// The task has ended, and its future is dropped.
    finished: AtomicBool,
}

impl Header {
    fn new(route: Route, thread: Option<ThreadId>) -> Arc<Self> {
        let header = Arc::new(Self {
            id: NEXT_TASK.fetch_add(1, Ordering::Relaxed),
            thread: OnceLock::new(),
            route,
            scheduled: AtomicBool::new(false),
            aborted: AtomicBool::new(false),
            finished: AtomicBool::new(false),
        });
        if let Some(thread) = thread {
            let _ = header.thread.set(thread);
        }
        header
    }

    fn on_its_thread(&self) -> bool {
        self.thread.get() == Some(&std::thread::current().id())
    }

    /// Queues a poll of the task: in the queue of its thread, or on its schedule unless one is
    /// queued there already.
    fn queue_poll(self: &Arc<Self>) {
        match &self.route {
            Route::Thread(inbox) => inbox.push(Pending::Task(self.id)),
            Route::Schedule(schedule) => {
                if !self.scheduled.swap(true, Ordering::AcqRel) {
                    let header = self.clone();
                    schedule.schedule(Box::new(move || {
                        header.scheduled.store(false, Ordering::Release);
                        poll_task(header.id);
                    }));
                }
            }
        }
    }

    /// Aborts the task: on its thread its future is dropped at once (after the poll that runs,
    /// when it is polled), from another thread when that thread polls it next.
    pub(crate) fn abort(self: &Arc<Self>) {
        self.aborted.store(true, Ordering::Release);
        if self.on_its_thread() {
            poll_task(self.id);
        } else {
            self.queue_poll();
        }
    }

    pub(crate) fn id(&self) -> u64 {
        self.id
    }

    pub(crate) fn is_finished(&self) -> bool {
        self.finished.load(Ordering::Acquire)
    }
}

impl Wake for Header {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        match &self.route {
            Route::Thread(_) if self.on_its_thread() => {
                poll_task(self.id);
            }
            _ => self.queue_poll(),
        }
    }
}

/// A task in the table of its thread.
struct LocalTask {
    header: Arc<Header>,
    waker: Waker,
    future: RefCell<Option<LocalFuture>>,
    on_failure: RefCell<Option<OnFailure>>,
    polling: Cell<bool>,
    repoll: Cell<bool>,
}

crate::ferro_thread_local! {
    /// The tasks of the thread, by id.
    static TASKS: RefCell<HashMap<u64, Rc<LocalTask>>> = RefCell::new(HashMap::new());
    /// The queue of the thread.
    static INBOX: Arc<Inbox> = Arc::new(Inbox::new());
    /// The callbacks of the thread that another thread may ask to run, by id.
    static CALLBACKS: RefCell<HashMap<u64, LocalCallbackFn>> = RefCell::new(HashMap::new());
    /// The id of the task being polled on the thread ([`try_id`]).
    static CURRENT: Cell<Option<u64>> = const { Cell::new(None) };
}

/// The ids of the tasks and of the callbacks, unique in the process so that a poll or a request
/// that reaches the wrong thread finds nothing.
static NEXT_TASK: AtomicU64 = AtomicU64::new(1);
static NEXT_CALLBACK: AtomicU64 = AtomicU64::new(1);

/// Polls a task of the current thread: again after the poll when it was woken from inside it
/// (the queue of the thread) or a queued poll ran inside it (a schedule, which gets a new poll);
/// ends it when its future is ready, panics, or the task was aborted. False when the thread has
/// no such task.
fn poll_task(id: u64) -> bool {
    let Some(task) = TASKS.try_with(|tasks| tasks.borrow().get(&id).cloned()).ok().flatten() else { return false };
    if task.polling.get() {
        task.repoll.set(true);
        return true;
    }
    loop {
        if task.header.aborted.load(Ordering::Acquire) {
            end_task(&task, Some(Failure::Canceled));
            return true;
        }
        task.polling.set(true);
        task.repoll.set(false);
        let polled = {
            let mut slot = task.future.borrow_mut();
            match slot.as_mut() {
                Some(future) => {
                    let mut context = Context::from_waker(&task.waker);
                    let previous = CURRENT.try_with(|current| current.replace(Some(id))).ok().flatten();
                    let polled = catch_unwind(AssertUnwindSafe(|| future.as_mut().poll(&mut context)));
                    let _ = CURRENT.try_with(|current| current.set(previous));
                    Some(polled)
                }
                None => None,
            }
        };
        task.polling.set(false);
        match polled {
            None => return true,
            Some(Ok(Poll::Ready(()))) => {
                end_task(&task, None);
                return true;
            }
            Some(Err(payload)) => {
                end_task(&task, Some(Failure::Panicked(payload)));
                return true;
            }
            Some(Ok(Poll::Pending)) => {}
        }
        if task.header.aborted.load(Ordering::Acquire) {
            end_task(&task, Some(Failure::Canceled));
            return true;
        }
        if !task.repoll.get() {
            return true;
        }
        if let Route::Schedule(_) = task.header.route {
            task.header.queue_poll();
            return true;
        }
    }
}

/// Removes a task from the table, drops its future, and tells its spawner of a failure.
fn end_task(task: &LocalTask, failure: Option<Failure>) {
    task.header.finished.store(true, Ordering::Release);
    let removed = TASKS.try_with(|tasks| tasks.borrow_mut().remove(&task.header.id)).ok().flatten();
    let future = task.future.borrow_mut().take();
    let on_failure = task.on_failure.borrow_mut().take();
    // Dropped outside of the borrows: what the future holds may wake, spawn or abort tasks.
    drop(future);
    drop(removed);
    if let (Some(failure), Some(on_failure)) = (failure, on_failure) {
        on_failure(failure);
    }
}

/// Adds a task to the table of the current thread and polls it, or queues its first poll.
/// `on_failure` is told when the task ends without the output of its future: an abort, a panic.
pub(crate) fn spawn_task(future: LocalFuture, schedule: Option<Arc<dyn Schedule>>, first_poll: FirstPoll, on_failure: OnFailure) -> Arc<Header> {
    let route = match schedule {
        Some(schedule) => Route::Schedule(schedule),
        None => Route::Thread(INBOX.with(Arc::clone)),
    };
    let header = Header::new(route, Some(std::thread::current().id()));
    start_task(header.clone(), future, on_failure, first_poll);
    header
}

/// Adds the task of `header` to the table of the current thread, tells its schedule, and polls
/// it or queues its first poll.
fn start_task(header: Arc<Header>, future: LocalFuture, on_failure: OnFailure, first_poll: FirstPoll) {
    let id = header.id;
    let task = Rc::new(LocalTask {
        header: header.clone(),
        waker: Waker::from(header.clone()),
        future: RefCell::new(Some(future)),
        on_failure: RefCell::new(Some(on_failure)),
        polling: Cell::new(false),
        repoll: Cell::new(false),
    });
    TASKS.with_borrow_mut(|tasks| tasks.insert(id, task));
    if let Route::Schedule(schedule) = &header.route {
        // The schedule may end the task here (a dispatcher that has shut down aborts it).
        schedule.spawned(AbortHandle::new(header.clone()));
    }
    if header.is_finished() {
        return;
    }
    match first_poll {
        FirstPoll::Now => {
            poll_task(id);
        }
        FirstPoll::Scheduled => header.queue_poll(),
    }
}

/// The future of a task with a handle: `future`, whose output is written to `cell`.
fn handle_future<F>(future: F, cell: Arc<JoinCell<F::Output>>) -> LocalFuture
where
    F: Future + 'static,
    F::Output: 'static,
{
    Box::pin(async move {
        let value = future.await;
        drop(cell.complete(Ok(value)));
    })
}

/// What a task with a handle does when it fails: writes the failure to `cell`.
fn handle_failure<T: 'static>(cell: Arc<JoinCell<T>>) -> OnFailure {
    Box::new(move |failure: Failure| {
        // A panic after the outcome was written (in what the completion ran) cannot fault the
        // task: it goes on from the call that polled it.
        if let Some(Err(JoinError::Panicked(payload))) = cell.complete(Err(failure.into())) {
            std::panic::resume_unwind(payload);
        }
    })
}

fn spawn_with_handle<F>(future: F, schedule: Option<Arc<dyn Schedule>>, first_poll: FirstPoll) -> JoinHandle<F::Output>
where
    F: Future + 'static,
    F::Output: 'static,
{
    let cell = Arc::new(JoinCell::new());
    let header = spawn_task(handle_future(future, cell.clone()), schedule, first_poll, handle_failure(cell.clone()));
    JoinHandle::new(header, cell)
}

/// Runs `future` as a task of the current thread, in the queue of the thread, and gives its
/// handle. The future is polled at once, up to its first pending await, and again whenever it is
/// woken: at once on this thread (inside the call that wakes it, or right after its own poll when
/// the wake comes from inside it), and when the thread runs [`run_pending`] for a wake from
/// another thread (the module's documentation).
pub fn spawn_local<F>(future: F) -> JoinHandle<F::Output>
where
    F: Future + 'static,
    F::Output: 'static,
{
    spawn_with_handle(future, None, FirstPoll::Now)
}

/// Runs `future` as a task of the current thread whose polls run on `schedule`, and gives its
/// handle. Every wake, from any thread, queues one poll on the schedule; the first poll runs
/// inside this call or is queued as well (`first_poll`).
pub fn spawn_local_on<F>(future: F, schedule: Arc<dyn Schedule>, first_poll: FirstPoll) -> JoinHandle<F::Output>
where
    F: Future + 'static,
    F::Output: 'static,
{
    spawn_with_handle(future, Some(schedule), first_poll)
}

/// Runs the future that `create` makes as a task of the thread of `schedule`, and gives its
/// handle, from any thread: `create` is `Send`, the future it makes need not be. The schedule is
/// given one piece of work that, on its thread, calls `create` and polls the future at once (both
/// inside that work, as [`FirstPoll::Now`] does inside a spawning call); every later wake queues a
/// poll there, as for [`spawn_local_on`].
///
/// The task ends canceled when it is aborted before that work runs, or when the schedule drops the
/// work without running it (a dispatcher that has shut down); a panic of `create` faults it like a
/// panic of the future.
pub fn spawn_on<C, F>(create: C, schedule: Arc<dyn Schedule>) -> JoinHandle<F::Output>
where
    C: FnOnce() -> F + Send + 'static,
    F: Future + 'static,
    F::Output: Send + 'static,
{
    let header = Header::new(Route::Schedule(schedule.clone()), None);
    // The work below is the task's queued poll until it runs.
    header.scheduled.store(true, Ordering::Release);
    let cell = Arc::new(JoinCell::new());
    let start = Start { header: header.clone(), cell: cell.clone(), create: Some(create) };
    schedule.schedule(Box::new(move || start.run::<F>()));
    JoinHandle::new(header, cell)
}

/// The first work of a task of [`spawn_on`]: makes its future on the schedule's thread and starts
/// it there; dropped without running, it cancels the task.
struct Start<C, T> {
    header: Arc<Header>,
    cell: Arc<JoinCell<T>>,
    create: Option<C>,
}

impl<C, T: 'static> Start<C, T> {
    fn run<F>(mut self)
    where
        C: FnOnce() -> F,
        F: Future<Output = T> + 'static,
    {
        let Some(create) = self.create.take() else { return };
        let header = self.header.clone();
        let _ = header.thread.set(std::thread::current().id());
        header.scheduled.store(false, Ordering::Release);
        if header.aborted.load(Ordering::Acquire) {
            header.finished.store(true, Ordering::Release);
            drop(self.cell.complete(Err(JoinError::Canceled)));
            return;
        }
        let future = match catch_unwind(AssertUnwindSafe(create)) {
            Ok(future) => future,
            Err(payload) => {
                header.finished.store(true, Ordering::Release);
                drop(self.cell.complete(Err(JoinError::Panicked(payload))));
                return;
            }
        };
        let cell = self.cell.clone();
        start_task(header, handle_future(future, cell.clone()), handle_failure(cell), FirstPoll::Now);
    }
}

impl<C, T> Drop for Start<C, T> {
    fn drop(&mut self) {
        if self.create.take().is_some() {
            self.header.finished.store(true, Ordering::Release);
            drop(self.cell.complete(Err(JoinError::Canceled)));
        }
    }
}

/// The id of the task being polled on the current thread, `None` outside of a poll of a task of
/// this module. In a task polled inside the poll of another (a wake of a task of the thread's
/// queue), the inner one.
pub fn try_id() -> Option<Id> {
    CURRENT.try_with(Cell::get).ok().flatten().map(Id::new)
}

/// Polls the task `id` of the current thread now, inside the calling code, whatever its schedule:
/// what moves one continuation of a task into the work of another schedule (a dispatcher job of
/// another priority). A task that is being polled is not polled inside its own poll: it is marked,
/// and polled again once that poll has returned (through its schedule, or at once for the queue of
/// the thread), as for a queued poll. False when the current thread has no such task (it has
/// ended, or belongs to another thread).
pub fn poll_now(id: Id) -> bool {
    poll_task(id.get())
}

/// Makes `schedule` the schedule of the current thread, `None` removes it: when a wake of a task
/// of the thread's queue, or a callback of the thread (`CancellationToken::register_local`), comes
/// from another thread, the schedule is given work that runs [`run_pending`]. Wakes that are
/// waiting already run at once.
pub fn set_thread_schedule(schedule: Option<Arc<dyn Schedule>>) {
    let inbox = INBOX.with(Arc::clone);
    let previous = std::mem::replace(&mut *inbox.schedule.lock().unwrap_or_else(PoisonError::into_inner), schedule);
    drop(previous);
    if !inbox.is_empty() {
        run_pending();
    }
}

/// Polls the tasks of the current thread's queue that other threads woke, and runs the callbacks
/// they asked for, in the order they came, until none is left; true when there was any.
pub fn run_pending() -> bool {
    let inbox = INBOX.with(Arc::clone);
    let mut ran = false;
    loop {
        let pending = inbox.take();
        if pending.is_empty() {
            return ran;
        }
        ran = true;
        for item in pending {
            match item {
                Pending::Task(id) => {
                    poll_task(id);
                }
                Pending::Callback(id) => run_callback(id),
            }
        }
    }
}

/// Runs `future` on the current thread until it ends, and gives its output: it is spawned in the
/// queue of the thread ([`spawn_local`]), and the thread waits for wakes from other threads in
/// between (a thread with nothing else to run: a console program, a test, a worker).
///
/// # Panics
/// With the panic of the future. Never returns when nothing wakes the future again: a future that
/// waits for this thread itself (work queued on a schedule of the thread) has to be ended by
/// something that runs before it waits.
pub fn block_on<F>(future: F) -> F::Output
where
    F: Future + 'static,
    F::Output: 'static,
{
    let handle = spawn_local(future);
    let inbox = INBOX.with(Arc::clone);
    loop {
        run_pending();
        if let Some(outcome) = handle.try_take() {
            return match outcome {
                Ok(output) => output,
                Err(error) => error.resume_unwind(),
            };
        }
        inbox.wait();
    }
}

/// A callback of the current thread that any thread can ask to run ([`LocalCallbackHandle::run`]):
/// on this thread it runs at once, from another it waits in the queue of this thread.
pub(crate) struct LocalCallback {
    id: u64,
    thread: ThreadId,
    inbox: Arc<Inbox>,
    /// The callback belongs to its thread.
    _thread_bound: std::marker::PhantomData<Rc<()>>,
}

impl LocalCallback {
    pub(crate) fn register(callback: LocalCallbackFn) -> Self {
        let id = NEXT_CALLBACK.fetch_add(1, Ordering::Relaxed);
        CALLBACKS.with_borrow_mut(|callbacks| callbacks.insert(id, callback));
        Self { id, thread: std::thread::current().id(), inbox: INBOX.with(Arc::clone), _thread_bound: std::marker::PhantomData }
    }

    /// The part that crosses threads.
    pub(crate) fn handle(&self) -> LocalCallbackHandle {
        LocalCallbackHandle { id: self.id, thread: self.thread, inbox: self.inbox.clone() }
    }

    /// Forgets the callback unless it ran; on the thread of the callback only.
    pub(crate) fn remove(&self) {
        let removed = CALLBACKS.try_with(|callbacks| callbacks.borrow_mut().remove(&self.id)).ok().flatten();
        drop(removed);
    }
}

/// Asks for a [`LocalCallback`] to run, from any thread; as a [`Waker`], waking it runs it.
pub(crate) struct LocalCallbackHandle {
    id: u64,
    thread: ThreadId,
    inbox: Arc<Inbox>,
}

impl LocalCallbackHandle {
    pub(crate) fn run(&self) {
        if std::thread::current().id() == self.thread {
            run_callback(self.id);
        } else {
            self.inbox.push(Pending::Callback(self.id));
        }
    }
}

impl Wake for LocalCallbackHandle {
    fn wake(self: Arc<Self>) {
        self.run();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.run();
    }
}

fn run_callback(id: u64) {
    let callback = CALLBACKS.try_with(|callbacks| callbacks.borrow_mut().remove(&id)).ok().flatten();
    if let Some(callback) = callback {
        callback();
    }
}
