//! Not from the reference: the `Task` of the asynchronous actions of the crate.
//!
//! The reference returns a `System.Threading.Tasks.Task` from the actions that run asynchronously
//! (`DelayAction`, `AsyncActionGroup`, the picker actions), and `AsyncActionGroup` awaits the task
//! an action returns. Here such an action returns a [`TaskHandle`]: a task of the dispatcher of the
//! user interface ([`DispatcherTask`]) that can be held in an untyped value (it compares by
//! identity) and awaited.
//!
//! Deviation (DEVIATIONS.md, Behaviors): `Task` and `Task.Delay` are [`TaskHandle`] and [`delay`],
//! futures run by the dispatcher of the user interface.

use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::{Dispatcher, DispatcherPriority, DispatcherTask, DispatcherTimer, OperationCanceledError};
use std::cell::RefCell;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

/// A task of the dispatcher of the user interface, held as an untyped value (`Task<T>`).
///
/// Clones refer to the same task and compare equal; the result of the task can be taken once
/// (awaiting it takes it).
pub struct TaskHandle<T: 'static> {
    task: DispatcherTask<T>,
    identity: Rc<()>,
}

/// The task of an action that produces no value (`Task`).
pub type ActionTask = TaskHandle<()>;

impl<T: 'static> TaskHandle<T> {
    /// Runs `future` on the dispatcher of the user interface and returns its task
    /// (`Dispatcher.UIThread.InvokeAsync(async () => ..)`). The future starts in a job of the
    /// dispatcher.
    ///
    /// # Panics
    /// Panics when called from a thread other than the thread of the user interface.
    pub fn spawn<Fut>(future: Fut) -> Self
    where
        Fut: Future<Output = T> + 'static,
    {
        Self::from_task(Dispatcher::ui_thread().invoke_async_task_local(move || future))
    }

    /// The handle of an existing dispatcher task.
    pub fn from_task(task: DispatcherTask<T>) -> Self {
        Self { task, identity: Rc::new(()) }
    }

    /// The dispatcher task.
    pub fn task(&self) -> &DispatcherTask<T> {
        &self.task
    }

    /// Whether the task has finished: completed, failed or canceled (`IsCompleted`).
    pub fn is_completed(&self) -> bool {
        self.task.is_completed()
    }

    /// Whether the task ran to the end (`IsCompletedSuccessfully`).
    pub fn is_completed_successfully(&self) -> bool {
        self.task.is_completed_successfully()
    }

    /// Whether the future of the task panicked (`IsFaulted`).
    pub fn is_faulted(&self) -> bool {
        self.task.is_faulted()
    }

    /// Whether the task was canceled (`IsCanceled`).
    pub fn is_canceled(&self) -> bool {
        self.task.is_canceled()
    }
}

impl ActionTask {
    /// A task that has already completed (`Task.CompletedTask`).
    pub fn completed() -> Self {
        Self::spawn(std::future::ready(()))
    }
}

impl<T: 'static> Clone for TaskHandle<T> {
    fn clone(&self) -> Self {
        Self { task: self.task.clone(), identity: self.identity.clone() }
    }
}

impl<T: 'static> PartialEq for TaskHandle<T> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.identity, &other.identity)
    }
}

impl<T: 'static> fmt::Debug for TaskHandle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TaskHandle({:?})", self.task)
    }
}

impl<T: 'static> Future for TaskHandle<T> {
    type Output = Result<T, OperationCanceledError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.get_mut().task).poll(cx)
    }
}

/// The state of a [`delay`]: whether the timer fired, and who waits for it.
#[derive(Default)]
struct DelayState {
    elapsed: bool,
    waker: Option<Waker>,
    timer: Option<Rc<dyn IDisposable>>,
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
