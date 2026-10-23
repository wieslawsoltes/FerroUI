//! Asynchronous data: a future run by a component and its state
//! ([`use_future`]), an element that follows that state ([`suspense`]), and a
//! future that waits on a timer of the dispatcher ([`sleep`]).
//!
//! The futures run on the executor of the thread
//! (`ferroui_system::threading::tasks::spawn_local`): a future is polled when
//! it is started and again whenever it is woken, on the thread of the user
//! interface, so what it reads and writes needs no synchronization. A future
//! that waits for work of another thread (`tasks::run`) is woken through the
//! dispatcher. Nothing here blocks the thread.

use crate::element::{rerender, Element};
use crate::hooks::{after, use_hook, use_signal};
use crate::reactive::{on_cleanup, Signal};
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_system::ObservableError;
use std::cell::{Cell, RefCell};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

/// The state of an asynchronous operation: pending until it ends, then its
/// value or the message of its error.
#[derive(Clone, Debug, PartialEq)]
pub enum AsyncState<T> {
    /// The operation has not ended.
    Pending,
    /// The operation ended with a value.
    Ready(T),
    /// The operation ended with an error.
    Failed(String),
}

impl<T> AsyncState<T> {
    /// Whether the operation has not ended.
    pub fn is_pending(&self) -> bool {
        matches!(self, AsyncState::Pending)
    }

    /// The value, once the operation ended with one.
    pub fn value(&self) -> Option<&T> {
        match self {
            AsyncState::Ready(value) => Some(value),
            _ => None,
        }
    }

    /// The message of the error, once the operation failed.
    pub fn error(&self) -> Option<&str> {
        match self {
            AsyncState::Failed(message) => Some(message),
            _ => None,
        }
    }
}

/// A future that was started and may be abandoned: its result is ignored,
/// and the executor is woken to drop it.
#[derive(Default)]
struct Abandon {
    abandoned: Cell<bool>,
    /// The waker of the last poll that was pending.
    waker: RefCell<Option<Waker>>,
}

impl Abandon {
    fn abandon(&self) {
        self.abandoned.set(true);
        let waker = self.waker.borrow_mut().take();
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

/// What a component that runs a future keeps across its runs.
struct Running<D> {
    /// The dependencies of the future that was started last.
    deps: Option<D>,
    /// That future.
    current: Option<Rc<Abandon>>,
}

/// Runs the future that `make` makes for `deps` on the executor of the
/// thread, when the component first runs and again whenever `deps` differ
/// from those of the run before; the state is [`AsyncState::Pending`] until
/// the future ends, then its value or its error. A hook: it follows the
/// rules of the [hooks](crate::hooks).
///
/// ```ignore
/// #[component(rerender)]
/// fn Weather(city: String) -> Element {
///     let forecast = use_future(city.clone(), |city| fetch_forecast(city.clone()));
///     match forecast.get() {
///         AsyncState::Pending => rsx! { ProgressBar { is_indeterminate: true } },
///         AsyncState::Ready(forecast) => rsx! { TextBlock { text: forecast.summary } },
///         AsyncState::Failed(error) => rsx! { TextBlock { text: error } },
///     }
/// }
/// ```
///
/// The future is started once the controls of the run are in place, as the
/// closure of [`use_effect_with`](crate::use_effect_with) runs. A future
/// whose dependencies were replaced, or whose component was taken out, is
/// abandoned: its result is ignored, and the executor drops it at once, which
/// ends what it waited for (a [`sleep`] stops its timer). Reading the state
/// with `get()` needs `T: Clone`; `with(|state| ..)` reads it in place.
pub fn use_future<D, T, F, Fut>(deps: D, make: F) -> Signal<AsyncState<T>>
where
    D: PartialEq + Clone + 'static,
    T: 'static,
    F: Fn(&D) -> Fut + 'static,
    Fut: Future<Output = Result<T, String>> + 'static,
{
    let state = use_signal(|| AsyncState::<T>::Pending);
    let running: Rc<RefCell<Running<D>>> = use_hook(|| {
        let running = Rc::new(RefCell::new(Running { deps: None, current: None }));
        let ended = running.clone();
        on_cleanup(move || {
            let current = ended.borrow_mut().current.take();
            if let Some(current) = current {
                current.abandon();
            }
        });
        running
    });
    let changed = running.borrow().deps.as_ref() != Some(&deps);
    if changed {
        after(move |_| {
            let previous = running.borrow_mut().current.take();
            if let Some(previous) = previous {
                previous.abandon();
            }
            if !state.is_alive() {
                return;
            }
            if !state.with_untracked(AsyncState::is_pending) {
                state.set(AsyncState::Pending);
            }
            let future = make(&deps);
            let abandon = Rc::new(Abandon::default());
            {
                let mut running = running.borrow_mut();
                running.deps = Some(deps);
                running.current = Some(abandon.clone());
            }
            spawn(future, state, abandon);
        });
    }
    state
}

/// Runs `future` on the executor of the thread and writes its outcome into
/// `state`, unless it was abandoned first.
fn spawn<T, Fut>(future: Fut, state: Signal<AsyncState<T>>, abandon: Rc<Abandon>)
where
    T: 'static,
    Fut: Future<Output = Result<T, String>> + 'static,
{
    let mut future = Box::pin(future);
    let watched = std::future::poll_fn(move |context: &mut Context<'_>| -> Poll<Result<(), ObservableError>> {
        if abandon.abandoned.get() {
            // Ending here drops the future, and what it waited for.
            return Poll::Ready(Ok(()));
        }
        match future.as_mut().poll(context) {
            Poll::Ready(outcome) => {
                // Ended: nothing is left to abandon.
                *abandon.waker.borrow_mut() = None;
                abandon.abandoned.set(true);
                if state.is_alive() {
                    state.set(match outcome {
                        Ok(value) => AsyncState::Ready(value),
                        Err(message) => AsyncState::Failed(message),
                    });
                }
                Poll::Ready(Ok(()))
            }
            Poll::Pending => {
                *abandon.waker.borrow_mut() = Some(context.waker().clone());
                Poll::Pending
            }
        }
    });
    let _task = ferroui_system::threading::tasks::spawn_local(watched);
}

/// An element that follows `state`: the element of `pending()` while it is
/// pending, of `ready(value)` once it is ready, and of `failed(message)` once
/// it failed. It is a component that runs again when the state changes; the
/// reconciler patches what the closures return, so the elements of two
/// states with the same controls keep them.
///
/// ```ignore
/// let people = use_future(query.get(), |query| load_people(query.clone()));
/// suspense(
///     people,
///     || rsx! { TextBlock { text: "Loading..." } },
///     |people| rsx! { ListBox { for p in &people { ListBoxItem { key: p.id, content: p.name.clone() } } } },
///     |error| rsx! { TextBlock { text: error } },
/// )
/// ```
pub fn suspense<T: Clone + 'static>(
    state: Signal<AsyncState<T>>,
    pending: impl Fn() -> Element + 'static,
    ready: impl Fn(T) -> Element + 'static,
    failed: impl Fn(String) -> Element + 'static,
) -> Element {
    rerender(move || match state.get() {
        AsyncState::Pending => pending(),
        AsyncState::Ready(value) => ready(value),
        AsyncState::Failed(message) => failed(message),
    })
}

/// What a [`Sleep`] shares with its timer.
#[derive(Default)]
struct SleepState {
    done: Cell<bool>,
    waker: RefCell<Option<Waker>>,
}

/// The future of [`sleep`].
struct Sleep {
    duration: Duration,
    state: Rc<SleepState>,
    timer: Option<Rc<dyn IDisposable>>,
}

impl Future for Sleep {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<()> {
        if self.state.done.get() {
            return Poll::Ready(());
        }
        *self.state.waker.borrow_mut() = Some(context.waker().clone());
        if self.timer.is_none() {
            let state = self.state.clone();
            let duration = self.duration;
            let timer = DispatcherTimer::run_once(
                move || {
                    state.done.set(true);
                    let waker = state.waker.borrow_mut().take();
                    if let Some(waker) = waker {
                        waker.wake();
                    }
                },
                duration,
                DispatcherPriority::NORMAL,
            );
            self.timer = Some(timer);
        }
        Poll::Pending
    }
}

impl Drop for Sleep {
    fn drop(&mut self) {
        // A sleep dropped before its time stops its timer.
        if let Some(timer) = self.timer.take() {
            if !self.state.done.get() {
                timer.dispose();
            }
        }
    }
}

/// A future that ends after `duration`, on a timer of the dispatcher of the
/// thread: for a delay in a future of [`use_future`], such as the debouncing
/// of what the user types. The timer starts when the future is first polled;
/// a sleep dropped before it ends (its future was abandoned) stops it.
///
/// ```ignore
/// let results = use_future(query.get(), |query| {
///     let query = query.clone();
///     async move {
///         sleep(Duration::from_millis(300)).await;
///         search(&query).await
///     }
/// });
/// ```
pub fn sleep(duration: Duration) -> impl Future<Output = ()> {
    Sleep { duration, state: Rc::new(SleepState::default()), timer: None }
}
