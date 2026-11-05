//! A value sent once to the code that waits for it: the port of `TaskCompletionSource` (step A3
//! of phase C4 of #423, docs/idiomatic-rust-async.md, section 3.3).
//!
//! [`channel`] makes the two ends. The [`Sender`] sends one value ([`Sender::send`], which
//! consumes it); dropping it without sending cancels the channel, and the [`Receiver`] ends with
//! [`Canceled`] instead of waiting for ever. The receiver is a future of the value, and can also
//! be asked without waiting ([`Receiver::try_recv`]) or waited for by blocking the thread
//! ([`Receiver::blocking_recv`], [`Receiver::recv_timeout`]). [`Receiver::shared`] makes a
//! receiver that several waiters clone, each getting a copy of the value.
//!
//! The channel has no thread of its own: either end may be used on any thread (both are `Send`
//! and `Sync` when the value is `Send`), and where the code that awaits the receiver continues is
//! decided by the waker of its task. Sending wakes that waker inside the call that sends; the
//! executor of the task decides whether the task runs at once (a task of the queue of the same
//! thread) or later (a dispatcher job, the queue of another thread).
//!
//! In terms of the `TaskCompletionSource` of .NET: `SetResult` is `send(value)`, `SetException`
//! is `send(Err(error))` on a channel of a `Result`, `SetCanceled` is dropping the sender, and the
//! `TrySet` forms are `send`, which gives the value back when the receiver is gone. The `flatten`
//! of a receiver of a `Result<T, ObservableError>` is a future of that result, with a dropped
//! sender as the error [`Canceled`]: the `Task` of the source, as the asynchronous forms of the
//! reactive libraries take it.

pub use super::Canceled;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::task::{Context, Poll, Waker};
use std::time::{Duration, Instant};

/// Makes a channel that carries one value: the end that sends it and the end that waits for it.
pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let inner = Arc::new(Inner {
        state: Mutex::new(State { value: None, complete: false, receivers: 1, next_id: 1, wakers: Vec::new() }),
        changed: Condvar::new(),
    });
    (Sender { inner: inner.clone(), sent: false }, Receiver { inner, id: 0 })
}

struct Inner<T> {
    state: Mutex<State<T>>,
    /// Told when the channel completes, for the receives that block their thread.
    changed: Condvar,
}

struct State<T> {
    /// The value, from when it is sent until a [`Receiver`] takes it.
    value: Option<T>,
    /// The value was sent or the sender is gone: nothing more comes.
    complete: bool,
    /// The receiving handles that are alive: the receiver, or the clones of its shared form.
    receivers: usize,
    /// The id of the next receiving handle.
    next_id: u64,
    /// The waker of the last poll of each receiving handle that waits, by its id.
    wakers: Vec<(u64, Waker)>,
}

impl<T> Inner<T> {
    fn lock(&self) -> MutexGuard<'_, State<T>> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Completes the channel, with the value when there is one, and wakes the waiters; gives the
    /// value back when no receiving handle is alive.
    fn complete(&self, value: Option<T>) -> Result<(), T> {
        let wakers = {
            let mut state = self.lock();
            if let Some(value) = value {
                if state.receivers == 0 {
                    return Err(value);
                }
                state.value = Some(value);
            }
            state.complete = true;
            std::mem::take(&mut state.wakers)
        };
        // Woken outside of the lock: a waker may poll the receiver at once.
        self.changed.notify_all();
        for (index, (_, waker)) in wakers.iter().enumerate() {
            // A task that awaits several clones of the shared receiver is woken once.
            if !wakers[..index].iter().any(|(_, earlier)| earlier.will_wake(waker)) {
                waker.wake_by_ref();
            }
        }
        Ok(())
    }

    /// Adds a receiving handle and gives its id.
    fn add_receiver(&self) -> u64 {
        let mut state = self.lock();
        state.receivers += 1;
        let id = state.next_id;
        state.next_id += 1;
        id
    }

    /// Removes a receiving handle and its waker.
    fn remove_receiver(&self, id: u64) {
        let waker = {
            let mut state = self.lock();
            state.receivers -= 1;
            let index = state.wakers.iter().position(|(waiter, _)| *waiter == id);
            index.map(|index| state.wakers.remove(index))
        };
        // Dropped outside of the lock.
        drop(waker);
    }

    /// Keeps the waker of a poll of the handle `id`; the one of its earlier poll is replaced.
    fn register(state: &mut State<T>, id: u64, waker: &Waker) {
        match state.wakers.iter_mut().find(|(waiter, _)| *waiter == id) {
            Some((_, current)) => {
                if !current.will_wake(waker) {
                    *current = waker.clone();
                }
            }
            None => state.wakers.push((id, waker.clone())),
        }
    }

    /// Blocks the thread until the channel completes or `timeout` has passed (never, when `None`),
    /// and gives the state.
    fn wait(&self, timeout: Option<Duration>) -> MutexGuard<'_, State<T>> {
        // A timeout too long to be a point in time waits as if there were none.
        let deadline = timeout.and_then(|timeout| Instant::now().checked_add(timeout));
        let mut state = self.lock();
        while !state.complete {
            state = match deadline {
                None => self.changed.wait(state).unwrap_or_else(PoisonError::into_inner),
                Some(deadline) => {
                    let now = Instant::now();
                    if now >= deadline {
                        break;
                    }
                    self.changed.wait_timeout(state, deadline - now).unwrap_or_else(PoisonError::into_inner).0
                }
            };
        }
        state
    }
}

/// The end of a [`channel`] that sends the value: once, from any thread.
///
/// Dropping it without sending cancels the channel: the receiver ends with [`Canceled`].
pub struct Sender<T> {
    inner: Arc<Inner<T>>,
    /// The value was sent, or given back: dropping the sender does nothing more.
    sent: bool,
}

impl<T> Sender<T> {
    /// Sends the value and wakes whoever waits for it, inside this call. Gives the value back
    /// when the receiver (and every clone of its shared form) is gone.
    pub fn send(mut self, value: T) -> Result<(), T> {
        self.sent = true;
        self.inner.complete(Some(value))
    }

    /// Whether the receiver (and every clone of its shared form) is gone, so that a value sent
    /// would be given back.
    pub fn is_closed(&self) -> bool {
        self.inner.lock().receivers == 0
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        if !self.sent {
            // Without a value: cannot give one back.
            let _ = self.inner.complete(None);
        }
    }
}

impl<T> fmt::Debug for Sender<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Sender").field("closed", &self.is_closed()).finish()
    }
}

/// The end of a [`channel`] that waits for the value: a future of it, or of [`Canceled`] when the
/// sender was dropped without sending.
///
/// The value is taken once: after the receiver gave it (as the output of a poll, or from
/// [`try_recv`](Self::try_recv)), the channel is empty for good, and the receiver gives
/// [`Canceled`]. Dropping the receiver makes [`Sender::send`] give the value back.
pub struct Receiver<T> {
    inner: Arc<Inner<T>>,
    id: u64,
}

impl<T> Receiver<T> {
    /// The value when it was sent (`Ok(Some)`), `Ok(None)` while it was not, and [`Canceled`] when
    /// the sender was dropped without sending, or the value was taken already.
    pub fn try_recv(&mut self) -> Result<Option<T>, Canceled> {
        let mut state = self.inner.lock();
        match state.value.take() {
            Some(value) => Ok(Some(value)),
            None if state.complete => Err(Canceled),
            None => Ok(None),
        }
    }

    /// Whether the channel has completed: the value was sent or the sender is gone, so that a
    /// poll is ready.
    pub fn is_finished(&self) -> bool {
        self.inner.lock().complete
    }

    /// Blocks the thread until the value is sent, or the sender is dropped.
    ///
    /// Never returns when the value is to be sent by work of this same thread.
    pub fn blocking_recv(self) -> Result<T, Canceled> {
        let mut state = self.inner.wait(None);
        let value = state.value.take();
        drop(state);
        value.ok_or(Canceled)
    }

    /// Blocks the thread until the value is sent, the sender is dropped, or `timeout` has passed;
    /// `Ok(None)` is the timeout.
    pub fn recv_timeout(&mut self, timeout: Duration) -> Result<Option<T>, Canceled> {
        let mut state = self.inner.wait(Some(timeout));
        match state.value.take() {
            Some(value) => Ok(Some(value)),
            None if state.complete => Err(Canceled),
            None => Ok(None),
        }
    }

    /// A receiver for several waiters: see [`SharedReceiver`].
    pub fn shared(self) -> SharedReceiver<T>
    where
        T: Clone,
    {
        let id = self.inner.add_receiver();
        SharedReceiver { inner: self.inner.clone(), id }
    }
}

impl<T> Receiver<Result<T, crate::ObservableError>> {
    /// A future of the result that the channel carries: what was sent, or the error [`Canceled`]
    /// when the sender was dropped without sending (`SetCanceled`).
    pub fn flatten(self) -> impl Future<Output = Result<T, crate::ObservableError>> {
        async move { self.await.unwrap_or_else(|canceled| Err(std::rc::Rc::new(canceled) as crate::ObservableError)) }
    }
}

impl<T> Future for Receiver<T> {
    type Output = Result<T, Canceled>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = self.inner.lock();
        if let Some(value) = state.value.take() {
            return Poll::Ready(Ok(value));
        }
        if state.complete {
            return Poll::Ready(Err(Canceled));
        }
        Inner::register(&mut *state, self.id, cx.waker());
        Poll::Pending
    }
}

impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        self.inner.remove_receiver(self.id);
    }
}

impl<T> fmt::Debug for Receiver<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Receiver").field("finished", &self.is_finished()).finish()
    }
}

/// A receiver for several waiters ([`Receiver::shared`]): each clone is a future of a copy of the
/// value, or of [`Canceled`]; the value stays in the channel for every clone, the ones made later
/// included. The awaiting clones are woken in the order they first waited, and a task that awaits
/// several of them is woken once.
///
/// The sender is closed ([`Sender::is_closed`]) when every clone is gone.
pub struct SharedReceiver<T> {
    inner: Arc<Inner<T>>,
    id: u64,
}

impl<T: Clone> SharedReceiver<T> {
    /// A copy of the value when it was sent (`Ok(Some)`), `Ok(None)` while it was not, and
    /// [`Canceled`] when the sender was dropped without sending.
    pub fn try_recv(&self) -> Result<Option<T>, Canceled> {
        Self::outcome(&*self.inner.lock())
    }

    /// Blocks the thread until the value is sent, or the sender is dropped.
    ///
    /// Never returns when the value is to be sent by work of this same thread.
    pub fn blocking_recv(&self) -> Result<T, Canceled> {
        let state = self.inner.wait(None);
        state.value.clone().ok_or(Canceled)
    }

    /// Blocks the thread until the value is sent, the sender is dropped, or `timeout` has passed;
    /// `Ok(None)` is the timeout.
    pub fn recv_timeout(&self, timeout: Duration) -> Result<Option<T>, Canceled> {
        Self::outcome(&*self.inner.wait(Some(timeout)))
    }

    fn outcome(state: &State<T>) -> Result<Option<T>, Canceled> {
        match &state.value {
            Some(value) => Ok(Some(value.clone())),
            None if state.complete => Err(Canceled),
            None => Ok(None),
        }
    }
}

impl<T> SharedReceiver<T> {
    /// Whether the channel has completed: the value was sent or the sender is gone, so that a
    /// poll is ready.
    pub fn is_finished(&self) -> bool {
        self.inner.lock().complete
    }
}

impl<T: Clone> SharedReceiver<Result<T, crate::ObservableError>> {
    /// A future of the result that the channel carries, for this waiter: what was sent, or the
    /// error [`Canceled`] when the sender was dropped without sending (`SetCanceled`).
    pub fn flatten(self) -> impl Future<Output = Result<T, crate::ObservableError>> {
        async move { self.await.unwrap_or_else(|canceled| Err(std::rc::Rc::new(canceled) as crate::ObservableError)) }
    }
}

impl<T> Clone for SharedReceiver<T> {
    fn clone(&self) -> Self {
        let id = self.inner.add_receiver();
        Self { inner: self.inner.clone(), id }
    }
}

impl<T: Clone> Future for SharedReceiver<T> {
    type Output = Result<T, Canceled>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = self.inner.lock();
        match Self::outcome(&*state) {
            Ok(Some(value)) => Poll::Ready(Ok(value)),
            Err(canceled) => Poll::Ready(Err(canceled)),
            Ok(None) => {
                Inner::register(&mut *state, self.id, cx.waker());
                Poll::Pending
            }
        }
    }
}

impl<T> Drop for SharedReceiver<T> {
    fn drop(&mut self) {
        self.inner.remove_receiver(self.id);
    }
}

impl<T> fmt::Debug for SharedReceiver<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SharedReceiver").field("finished", &self.is_finished()).finish()
    }
}
