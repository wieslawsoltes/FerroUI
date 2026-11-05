//! Asynchronous data: a future run by a component and its state
//! ([`use_future`]), an element that follows that state ([`suspense`]), and a
//! future that waits on a timer of the dispatcher ([`sleep`]).
//!
//! The futures run on the dispatcher of the thread
//! (`Dispatcher::start_local`): a future is polled for the first time when
//! it is started, and every later poll is a dispatcher job that a wake
//! queues, at the priority of the job that started it
//! ([`DispatcherPriority::DEFAULT`] outside of a job), on the thread of the
//! user interface, so what it reads and writes needs no synchronization. A
//! load that is replaced is aborted (`JoinHandle::abort`), which drops its
//! future. Nothing here blocks the thread.

use crate::element::{rerender, Element};
use crate::hooks::{after, use_hook, use_signal};
use crate::reactive::{on_cleanup, Signal};
use ferroui_base::reactive::Subscription;
use ferroui_base::threading::{Dispatcher, DispatcherPriority, DispatcherTimer, JoinHandle};
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

/// What a component that runs a future keeps across its runs.
struct Running<D> {
    /// The dependencies of the future that was started last.
    deps: Option<D>,
    /// The task of that future, until it is replaced or the component ends.
    current: Option<JoinHandle<()>>,
    /// The number of the load started last, shared with the tasks: a load
    /// writes the state only while it is the latest. The abort drops the
    /// future of a replaced load at once, except while that load is being
    /// polled (it replaced itself, by writing a signal that the dependencies
    /// follow), where the future is dropped once the poll returns, and this
    /// keeps that poll from writing.
    latest: Rc<Cell<u64>>,
}

impl<D> Running<D> {
    /// Ends the load that was started last: no load is the latest any more,
    /// and the task is taken, for the caller to abort outside of the borrow
    /// (dropping a future runs arbitrary code).
    fn supersede(&mut self) -> Option<JoinHandle<()>> {
        self.latest.set(self.latest.get() + 1);
        self.current.take()
    }
}

/// Runs the future that `make` makes for `deps` on the dispatcher of the
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
/// closure of [`use_effect_with`](crate::use_effect_with) runs, with
/// `Dispatcher::start_local`: it runs up to its first pending `await` inside
/// that call, and every later poll is a job of the dispatcher that a wake
/// queues, at the priority of the job that started it
/// ([`DispatcherPriority::DEFAULT`] outside of a job). So the state is written
/// in the job that completes the future (or in the start, for a future that
/// does not wait), never inside the call that woke it, and behind the work of
/// that priority queued before the wake.
///
/// A future whose dependencies were replaced, or whose component was taken
/// out, is aborted (`JoinHandle::abort`): the dispatcher drops it at once,
/// which ends what it waited for (a [`sleep`] stops its timer), and it never
/// writes the state, also when what it awaited completed before it was
/// replaced. Reading the state with `get()` needs `T: Clone`;
/// `with(|state| ..)` reads it in place.
pub fn use_future<D, T, F, Fut>(deps: D, make: F) -> Signal<AsyncState<T>>
where
    D: PartialEq + Clone + 'static,
    T: 'static,
    F: Fn(&D) -> Fut + 'static,
    Fut: Future<Output = Result<T, String>> + 'static,
{
    let state = use_signal(|| AsyncState::<T>::Pending);
    let running: Rc<RefCell<Running<D>>> = use_hook(|| {
        let running = Rc::new(RefCell::new(Running { deps: None, current: None, latest: Rc::new(Cell::new(0)) }));
        let ended = running.clone();
        on_cleanup(move || {
            let current = ended.borrow_mut().supersede();
            if let Some(current) = current {
                current.abort();
            }
        });
        running
    });
    let changed = running.borrow().deps.as_ref() != Some(&deps);
    if changed {
        after(move |_| {
            let previous = running.borrow_mut().supersede();
            if let Some(previous) = previous {
                previous.abort();
            }
            if !state.is_alive() {
                return;
            }
            if !state.with_untracked(AsyncState::is_pending) {
                state.set(AsyncState::Pending);
            }
            let future = make(&deps);
            let (latest, load) = {
                let mut running = running.borrow_mut();
                running.deps = Some(deps);
                (running.latest.clone(), running.latest.get())
            };
            let task = start(future, state, latest.clone(), load);
            if latest.get() == load {
                running.borrow_mut().current = Some(task);
            } else {
                // Replaced inside its first poll: a later load is the current one.
                task.abort();
            }
        });
    }
    state
}

/// Starts `future` on the dispatcher of the thread and writes its outcome
/// into `state` in the job that completes it, if the load is still the
/// latest (`latest` is `load`) and the state is alive.
fn start<T, Fut>(future: Fut, state: Signal<AsyncState<T>>, latest: Rc<Cell<u64>>, load: u64) -> JoinHandle<()>
where
    T: 'static,
    Fut: Future<Output = Result<T, String>> + 'static,
{
    Dispatcher::current_dispatcher().start_local(async move {
        let outcome = future.await;
        if latest.get() == load && state.is_alive() {
            state.set(match outcome {
                Ok(value) => AsyncState::Ready(value),
                Err(message) => AsyncState::Failed(message),
            });
        }
    })
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
    /// The timer; a sleep dropped before its time stops it with the guard.
    timer: Option<Subscription>,
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
