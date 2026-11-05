//! Not from the reference: the `Task` of the asynchronous actions of the crate.
//!
//! The reference returns a `System.Threading.Tasks.Task` from the actions that run asynchronously
//! (`DelayAction`, `AsyncActionGroup`, the picker actions), and `AsyncActionGroup` awaits the task
//! an action returns. Here such an action returns a [`TaskHandle`]: the [`JoinHandle`] of a task of
//! the dispatcher of the user interface, shared so that it can be held in an untyped value (it
//! compares by identity) and awaited.
//!
//! Deviation (DEVIATIONS.md, Behaviors): `Task` and `Task.Delay` are [`TaskHandle`] and [`delay`],
//! futures run by the dispatcher of the user interface.

#[allow(deprecated)]
use ferroui_base::threading::DispatcherTask;
use ferroui_base::reactive::Subscription;
use ferroui_base::threading::{Dispatcher, DispatcherPriority, DispatcherTimer, JoinError, JoinHandle};
use std::any::Any;
use std::cell::RefCell;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::{Arc, Mutex, PoisonError};
use std::task::{Context, Poll, Wake, Waker};
use std::time::Duration;

/// How the task of a [`TaskHandle`] ended, once a handle has seen it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Status {
    Running,
    Completed,
    Faulted,
    Canceled,
}

/// The state that the clones of a [`TaskHandle`] share.
struct HandleState<T> {
    /// The handle of the task, until its outcome is taken from it.
    handle: Option<JoinHandle<T>>,
    status: Status,
    /// The outcome taken from the handle, until a clone that awaits the task takes it.
    outcome: Option<Result<T, JoinError>>,
    /// The message of the panic of a faulted task, for the clones that await it afterwards.
    panic_message: Option<String>,
}

impl<T: 'static> HandleState<T> {
    /// Keeps the outcome of the task, which has ended.
    fn record(&mut self, outcome: Result<T, JoinError>) {
        self.handle = None;
        self.status = match &outcome {
            Ok(_) => Status::Completed,
            Err(JoinError::Canceled) => Status::Canceled,
            Err(JoinError::Panicked(payload)) => {
                self.panic_message = Some(panic_message(&**payload));
                Status::Faulted
            }
        };
        self.outcome = Some(outcome);
    }

    /// The outcome for a clone that awaits the task after another one took it: the failure again,
    /// the cancellation again, `()` again; a result of another type is taken once.
    fn observed_again(&self) -> Result<T, JoinError> {
        match self.status {
            Status::Canceled => Err(JoinError::Canceled),
            Status::Faulted => Err(JoinError::Panicked(Box::new(self.panic_message.clone().unwrap_or_default()))),
            Status::Completed | Status::Running => match (Box::new(()) as Box<dyn Any>).downcast::<T>() {
                Ok(unit) => Ok(*unit),
                Err(_) => panic!("The result of the task has already been taken."),
            },
        }
    }
}

/// The message of the payload of a panic.
fn panic_message(payload: &(dyn Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&'static str>() {
        (*message).to_owned()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "the task panicked".to_owned()
    }
}

/// The wakers of the clones that await the task: the end of the task wakes them all.
#[derive(Default)]
struct Wakers(Mutex<Vec<Waker>>);

impl Wakers {
    fn register(&self, waker: &Waker) {
        let mut wakers = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        if !wakers.iter().any(|registered| registered.will_wake(waker)) {
            wakers.push(waker.clone());
        }
    }
}

impl Wake for Wakers {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        // Woken outside of the lock.
        let wakers = std::mem::take(&mut *self.0.lock().unwrap_or_else(PoisonError::into_inner));
        for waker in wakers {
            waker.wake();
        }
    }
}

/// A task of the dispatcher of the user interface, held as an untyped value (`Task<T>`): its
/// [`JoinHandle`], shared.
///
/// Clones refer to the same task and compare equal. Awaiting a handle gives the outcome of the
/// task: its output, or a [`JoinError`] (`Panicked` when the future panicked, a value, not raised
/// again; `Canceled` when the task was aborted or the dispatcher shut down). Any number of clones
/// may await it; the output is taken by the first, the others get the failure or the
/// cancellation again, and `()` again for a task without a value.
pub struct TaskHandle<T: 'static> {
    state: Rc<RefCell<HandleState<T>>>,
    wakers: Arc<Wakers>,
}

/// The task of an action that produces no value (`Task`).
pub type ActionTask = TaskHandle<()>;

impl<T: 'static> TaskHandle<T> {
    /// Runs `future` on the dispatcher of the user interface and returns its task
    /// (`Dispatcher.UIThread.InvokeAsync(async () => ..)`). The future starts in a job of the
    /// dispatcher at [`DispatcherPriority::DEFAULT`].
    ///
    /// # Panics
    /// Panics when called from a thread other than the thread of the user interface.
    pub fn spawn<Fut>(future: Fut) -> Self
    where
        Fut: Future<Output = T> + 'static,
    {
        Self::from_handle(Dispatcher::ui_thread().spawn_local_with_priority(future, DispatcherPriority::DEFAULT))
    }

    /// The task of the handle of a task of the dispatcher.
    pub fn from_handle(handle: JoinHandle<T>) -> Self {
        let state = HandleState { handle: Some(handle), status: Status::Running, outcome: None, panic_message: None };
        Self { state: Rc::new(RefCell::new(state)), wakers: Arc::default() }
    }

    /// The task of an existing dispatcher task.
    ///
    /// Deprecated: the task of the handle of a task of the dispatcher is
    /// [`from_handle`](Self::from_handle) (`From`); the `DispatcherTask` converts into a
    /// `JoinHandle` (on the dispatcher thread).
    #[deprecated(
        since = "0.5.0",
        note = "use `TaskHandle::from_handle` with a `JoinHandle` (`DispatcherTask` converts into one)"
    )]
    #[allow(deprecated)]
    pub fn from_task(task: DispatcherTask<T>) -> Self {
        Self::from_handle(task.into())
    }

    /// Takes the outcome from the handle when the task has ended, which tells how it ended.
    fn settle(&self) {
        let mut state = self.state.borrow_mut();
        let Some(handle) = state.handle.as_mut().filter(|handle| handle.is_finished()) else {
            return;
        };
        let waker = Waker::from(self.wakers.clone());
        if let Poll::Ready(outcome) = Pin::new(handle).poll(&mut Context::from_waker(&waker)) {
            state.record(outcome);
        }
    }

    fn status(&self) -> Status {
        self.settle();
        self.state.borrow().status
    }

    /// Whether the task has finished: completed, failed or canceled (`IsCompleted`).
    pub fn is_completed(&self) -> bool {
        self.status() != Status::Running
    }

    /// Whether the task ran to the end (`IsCompletedSuccessfully`).
    pub fn is_completed_successfully(&self) -> bool {
        self.status() == Status::Completed
    }

    /// Whether the future of the task panicked (`IsFaulted`).
    pub fn is_faulted(&self) -> bool {
        self.status() == Status::Faulted
    }

    /// Whether the task was canceled (`IsCanceled`).
    pub fn is_canceled(&self) -> bool {
        self.status() == Status::Canceled
    }

    /// Aborts the task, unless it has ended: its future is dropped, and it ends canceled.
    pub fn abort(&self) {
        // Aborted outside of the borrow: the future runs arbitrary code when it is dropped.
        let abort = self.state.borrow().handle.as_ref().map(JoinHandle::abort_handle);
        if let Some(abort) = abort {
            abort.abort();
        }
    }
}

impl ActionTask {
    /// A task that has already completed (`Task.CompletedTask`).
    pub fn completed() -> Self {
        Self::spawn(std::future::ready(()))
    }
}

impl<T: 'static> From<JoinHandle<T>> for TaskHandle<T> {
    fn from(handle: JoinHandle<T>) -> Self {
        Self::from_handle(handle)
    }
}

impl<T: 'static> Clone for TaskHandle<T> {
    fn clone(&self) -> Self {
        Self { state: self.state.clone(), wakers: self.wakers.clone() }
    }
}

impl<T: 'static> PartialEq for TaskHandle<T> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.state, &other.state)
    }
}

impl<T: 'static> fmt::Debug for TaskHandle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TaskHandle({:?})", self.state.borrow().status)
    }
}

impl<T: 'static> Future for TaskHandle<T> {
    type Output = Result<T, JoinError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        this.wakers.register(cx.waker());
        let mut state = this.state.borrow_mut();
        if let Some(handle) = state.handle.as_mut() {
            let waker = Waker::from(this.wakers.clone());
            match Pin::new(handle).poll(&mut Context::from_waker(&waker)) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(outcome) => state.record(outcome),
            }
        }
        match state.outcome.take() {
            Some(outcome) => Poll::Ready(outcome),
            None => Poll::Ready(state.observed_again()),
        }
    }
}

/// The state of a [`delay`]: whether the timer fired, and who waits for it.
#[derive(Default)]
struct DelayState {
    elapsed: bool,
    waker: Option<Waker>,
    timer: Option<Subscription>,
}

/// The future of [`delay`].
pub struct Delay {
    duration: Duration,
    state: Rc<RefCell<DelayState>>,
}

/// A future that completes after `duration` (`Task.Delay(duration)`), measured by a timer of the
/// dispatcher of the thread that first polls it. A zero duration completes at the first poll.
pub fn delay(duration: Duration) -> Delay {
    Delay { duration, state: Rc::new(RefCell::new(DelayState::default())) }
}

impl Future for Delay {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let this = self.get_mut();
        if this.duration.is_zero() {
            return Poll::Ready(());
        }

        let mut state = this.state.borrow_mut();
        if state.elapsed {
            return Poll::Ready(());
        }

        state.waker = Some(cx.waker().clone());
        if state.timer.is_none() {
            let shared = Rc::downgrade(&this.state);
            state.timer = Some(DispatcherTimer::run_once(
                move || {
                    let Some(shared) = shared.upgrade() else {
                        return;
                    };
                    let waker = {
                        let mut state = shared.borrow_mut();
                        state.elapsed = true;
                        state.waker.take()
                    };
                    if let Some(waker) = waker {
                        waker.wake();
                    }
                },
                this.duration,
                DispatcherPriority::BACKGROUND,
            ));
        }

        Poll::Pending
    }
}

impl Drop for Delay {
    fn drop(&mut self) {
        let timer = self.state.borrow_mut().timer.take();
        if let Some(timer) = timer {
            timer.dispose();
        }
    }
}

/// A future that polls `future` and swallows a panic of it: what a continuation that only owns
/// clean-up needs (`try { await operation; } catch { }`). Resolves to `None` when the future
/// panicked.
pub(crate) struct CatchUnwind<F> {
    future: Pin<Box<F>>,
}

impl<F: Future> CatchUnwind<F> {
    pub(crate) fn new(future: F) -> Self {
        Self { future: Box::pin(future) }
    }
}

impl<F: Future> Future for CatchUnwind<F> {
    type Output = Option<F::Output>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| this.future.as_mut().poll(cx))) {
            Ok(Poll::Ready(output)) => Poll::Ready(Some(output)),
            Ok(Poll::Pending) => Poll::Pending,
            Err(_) => Poll::Ready(None),
        }
    }
}
