//! A cooperative cancellation primitive: [`CancellationToken`], in the shape of
//! `tokio_util::sync::CancellationToken` (step A4 of phase C4 of #423,
//! docs/idiomatic-rust-async.md, section 3.7), and the deprecated port of `CancellationTokenSource`
//! over it.
//!
//! A token is a cheap handle that can be shared across threads. [`CancellationToken::new`] makes
//! one that can be canceled, by any of its clones ([`CancellationToken::cancel`]);
//! [`CancellationToken::none`] one that never is. Code that observes a token asks
//! ([`CancellationToken::is_cancelled`]), awaits it ([`CancellationToken::cancelled`]), or
//! registers a callback ([`CancellationToken::register`], whose [`CancellationGuard`] unregisters
//! it when it is dropped). A child token ([`CancellationToken::child_token`]) is
//! canceled with its parent; [`CancellationToken::drop_guard`] cancels a token when the guard is
//! dropped.
//!
//! A cancellation runs the callbacks synchronously, on the cancelling thread, once, in the order
//! of registration (.NET runs them in reverse order; docs/porting/DEVIATIONS.md, Reactive), and
//! wakes the futures awaiting it in the same pass. Each callback runs outside of the lock of the
//! token: a callback may register, unregister and cancel, on this token or another, and a
//! callback unregistered by an earlier one does not run.
//!
//! The dispatcher API takes cancellation tokens in the same places the reference implementation
//! does (run loops, queued operations, frames).

use crate::task::executor::{LocalCallback, LocalCallbackHandle};
use std::collections::BTreeMap;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};
use std::task::{Context, Poll, Waker};

type Callback = Box<dyn FnOnce() + Send>;

struct Inner {
    canceled: AtomicBool,
    state: Mutex<State>,
    /// The parent of a child token and the id of the entry that cancels this one there, removed
    /// when this token is dropped.
    parent: Option<(Weak<Inner>, u64)>,
}

/// What runs when a token is canceled.
enum Entry {
    Callback(Callback),
    /// A future of the cancellation ([`WaitForCancellation`]) that waits.
    Waker(Waker),
}

#[derive(Default)]
struct State {
    next_id: u64,
    /// By id, which is the order of registration.
    entries: BTreeMap<u64, Entry>,
}

impl State {
    fn insert(&mut self, entry: Entry) -> u64 {
        self.next_id += 1;
        let id = self.next_id;
        self.entries.insert(id, entry);
        id
    }
}

impl Inner {
    fn new(canceled: bool, parent: Option<(Weak<Inner>, u64)>) -> Self {
        Self { canceled: AtomicBool::new(canceled), state: Mutex::new(State::default()), parent }
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn is_canceled(&self) -> bool {
        self.canceled.load(Ordering::SeqCst)
    }

    /// Adds `callback` and gives its id; runs it at once, and gives `None`, when the token is
    /// already canceled.
    fn add(&self, callback: Callback) -> Option<u64> {
        {
            let mut state = self.lock();
            // Checked under the lock so that a concurrent `cancel` either sees the new entry or
            // has already flipped the flag.
            if !self.is_canceled() {
                return Some(state.insert(Entry::Callback(callback)));
            }
        }
        callback();
        None
    }

    /// Removes the entry `id` unless it ran.
    fn remove(&self, id: u64) {
        let removed = self.lock().entries.remove(&id);
        // Dropped outside of the lock: a callback may own a token whose drop reaches this one.
        drop(removed);
    }

    /// Sets the flag and runs the entries, one at a time and outside of the lock; subsequent
    /// calls do nothing.
    fn cancel(&self) {
        if self.canceled.swap(true, Ordering::SeqCst) {
            return;
        }
        loop {
            let entry = self.lock().entries.pop_first();
            match entry {
                Some((_, Entry::Callback(callback))) => callback(),
                Some((_, Entry::Waker(waker))) => waker.wake(),
                None => break,
            }
        }
    }
}

impl Drop for Inner {
    fn drop(&mut self) {
        // A dropped child leaves its parent.
        if let Some((parent, id)) = self.parent.take() {
            if let Some(parent) = parent.upgrade() {
                parent.remove(id);
            }
        }
    }
}

/// Propagates notification that operations should be canceled.
///
/// The clones of a token share its state: any of them cancels it, and all of them see it.
/// `Default` is [`none`](Self::none), a token that can never be canceled (the default of .NET's
/// `CancellationToken`; `tokio_util`'s default is a new token).
#[derive(Clone, Default)]
pub struct CancellationToken {
    inner: Option<Arc<Inner>>,
}

impl CancellationToken {
    /// A token that can be canceled, by itself and its clones (a new `CancellationTokenSource`
    /// and its token in one).
    pub fn new() -> Self {
        Self { inner: Some(Arc::new(Inner::new(false, None))) }
    }

    /// A token that can never be canceled.
    pub const fn none() -> Self {
        Self { inner: None }
    }

    /// Whether this token is capable of being in the canceled state.
    pub fn can_be_cancelled(&self) -> bool {
        self.inner.is_some()
    }

    /// Whether this token is capable of being in the canceled state.
    #[deprecated(since = "0.5.0", note = "use `can_be_cancelled`")]
    pub fn can_be_canceled(&self) -> bool {
        self.can_be_cancelled()
    }

    /// Whether the token is canceled.
    pub fn is_cancelled(&self) -> bool {
        self.inner.as_ref().is_some_and(|inner| inner.is_canceled())
    }

    /// Whether the token is canceled.
    #[deprecated(since = "0.5.0", note = "use `is_cancelled`")]
    pub fn is_cancellation_requested(&self) -> bool {
        self.is_cancelled()
    }

    /// Cancels the token and its children, and runs the registered callbacks and wakes the
    /// futures awaiting it, on the calling thread, in the order of registration. Subsequent calls,
    /// and calls on a token that cannot be canceled, do nothing.
    pub fn cancel(&self) {
        if let Some(inner) = &self.inner {
            inner.cancel();
        }
    }

    /// A token canceled when this one is, which can also be canceled on its own without
    /// canceling this one. It is canceled at once when this one already is; a child of a token
    /// that cannot be canceled is a new token. The children of a token are canceled in its pass
    /// of the callbacks, in the order they were made among them; a child whose clones are all
    /// dropped is removed from its parent.
    pub fn child_token(&self) -> CancellationToken {
        let Some(parent) = &self.inner else {
            return Self::new();
        };
        let child = Arc::new_cyclic(|child: &Weak<Inner>| {
            let mut state = parent.lock();
            if parent.is_canceled() {
                return Inner::new(true, None);
            }
            let child = child.clone();
            let id = state.insert(Entry::Callback(Box::new(move || {
                if let Some(child) = child.upgrade() {
                    child.cancel();
                }
            })));
            Inner::new(false, Some((Arc::downgrade(parent), id)))
        });
        // A cancellation of the parent that reached the entry before the child was made could not
        // cancel it.
        if parent.is_canceled() {
            child.cancel();
        }
        Self { inner: Some(child) }
    }

    /// A future that completes when the token is canceled; at once when it already is, never for
    /// a token that cannot be canceled. It owns a clone of the token, is `Send`, and is woken on
    /// the cancelling thread, in the pass of the callbacks.
    pub fn cancelled(&self) -> WaitForCancellation {
        WaitForCancellation { inner: self.inner.clone(), id: None }
    }

    /// A guard that cancels the token when it is dropped, unless it is
    /// [disarmed](DropGuard::disarm).
    pub fn drop_guard(self) -> DropGuard {
        DropGuard { token: Some(self) }
    }

    /// Registers a callback that runs, on the cancelling thread, when the token is canceled; at
    /// once on the calling thread when it already is. Dropping the guard removes it
    /// ([`CancellationGuard::detach`] keeps it). The callback never runs for a token that cannot
    /// be canceled.
    pub fn register(&self, callback: impl FnOnce() + Send + 'static) -> CancellationGuard {
        let registration = self
            .inner
            .as_ref()
            .and_then(|inner| Some((inner.clone(), inner.add(Box::new(callback))?)));
        CancellationGuard { registration, local: None }
    }

    /// The former name of [`register`](Self::register).
    #[deprecated(since = "0.5.0", note = "use `register`, which gives the same guard")]
    pub fn register_guarded(&self, callback: impl FnOnce() + Send + 'static) -> CancellationGuard {
        self.register(callback)
    }

    /// Registers a callback that runs when the token is canceled. When the token is already
    /// canceled the callback runs immediately.
    ///
    /// Dropping the returned registration keeps the callback (the `IDisposable` contract).
    #[deprecated(
        since = "0.5.0",
        note = "use `register`, whose guard removes the callback when it is dropped; `detach()` keeps it"
    )]
    #[allow(deprecated)]
    pub fn register_handle(&self, callback: impl FnOnce() + Send + 'static) -> CancellationTokenRegistration {
        let id = self.inner.as_ref().and_then(|inner| inner.add(Box::new(callback)));
        match id {
            Some(id) => CancellationTokenRegistration { inner: self.inner.clone(), id },
            None => CancellationTokenRegistration { inner: None, id: 0 },
        }
    }
}

impl CancellationToken {
    /// Registers a callback that runs on the current thread when the token is canceled: at once
    /// when it is canceled on this thread, or already canceled; through the schedule of this thread
    /// ([`crate::task::set_thread_schedule`]; a dispatcher, in a job at its default priority), or
    /// its queue of [`run_pending_wakes`](crate::threading::tasks::run_pending_wakes), when it is
    /// canceled on another thread. The callback need not be `Send`; the guard is: dropped on
    /// another thread, it removes the registration at once and the callback is dropped on its own
    /// thread, the next time that thread runs its queue. [`CancellationGuard::detach`] keeps it.
    pub fn register_local(&self, callback: impl FnOnce() + 'static) -> CancellationGuard {
        let local = LocalCallback::register(Box::new(callback));
        let run = local.handle();
        let registration = self
            .inner
            .as_ref()
            .and_then(|inner| Some((inner.clone(), inner.add(Box::new(move || run.run()))?)));
        CancellationGuard { registration, local: Some(local.handle()) }
    }

    /// The former name of [`register_local`](Self::register_local).
    #[deprecated(since = "0.5.0", note = "use `register_local`, which gives the same guard")]
    pub fn register_local_guarded(&self, callback: impl FnOnce() + 'static) -> CancellationGuard {
        self.register_local(callback)
    }

    /// Registers a callback that runs on the current thread when the token is canceled, as
    /// [`register_local`](Self::register_local) does; dropping the returned registration keeps it.
    #[deprecated(
        since = "0.5.0",
        note = "use `register_local`, whose guard removes the callback when it is dropped; `detach()` keeps it"
    )]
    #[allow(deprecated)]
    pub fn register_local_handle(&self, callback: impl FnOnce() + 'static) -> LocalCancellationTokenRegistration {
        let local = LocalCallback::register(Box::new(callback));
        let handle = local.handle();
        let registration = self.register_handle(move || handle.run());
        LocalCancellationTokenRegistration { registration, local }
    }

    /// The number of callbacks, children and waiting futures registered with the token.
    #[cfg(test)]
    pub(crate) fn registration_count(&self) -> usize {
        self.inner.as_ref().map_or(0, |inner| inner.lock().entries.len())
    }
}

impl fmt::Debug for CancellationToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CancellationToken")
            .field("can_be_cancelled", &self.can_be_cancelled())
            .field("is_cancelled", &self.is_cancelled())
            .finish()
    }
}

/// A callback registered with [`CancellationToken::register`] or
/// [`CancellationToken::register_local`], removed when the guard is dropped (the place of
/// `CancellationTokenRegistration`).
///
/// The shape of [`Subscription`](crate::Subscription): [`dispose`](Self::dispose) removes it now,
/// [`detach`](Self::detach) keeps it for the life of the token. A callback that is running when
/// its guard is dropped finishes (.NET's `Dispose` waits for it).
#[must_use = "dropping the guard removes the callback; `detach()` keeps it for the life of the token"]
pub struct CancellationGuard {
    registration: Option<(Arc<Inner>, u64)>,
    /// The callback of a local registration, forgotten on its thread.
    local: Option<LocalCallbackHandle>,
}

impl CancellationGuard {
    /// Removes the callback now, the same as dropping the guard.
    pub fn dispose(self) {
        drop(self);
    }

    /// Keeps the callback for the life of the token.
    pub fn detach(mut self) {
        self.registration = None;
        self.local = None;
    }
}

impl Drop for CancellationGuard {
    fn drop(&mut self) {
        if let Some((inner, id)) = self.registration.take() {
            inner.remove(id);
        }
        if let Some(local) = self.local.take() {
            local.forget();
        }
    }
}

impl fmt::Debug for CancellationGuard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CancellationGuard")
    }
}

/// The future of [`CancellationToken::cancelled`]: completes when the token is canceled.
#[must_use = "futures do nothing unless they are awaited"]
pub struct WaitForCancellation {
    inner: Option<Arc<Inner>>,
    /// The id of the entry that holds the waker of the last poll.
    id: Option<u64>,
}

impl Future for WaitForCancellation {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let this = &mut *self;
        let Some(inner) = &this.inner else {
            return Poll::Pending;
        };
        let mut replaced = None;
        let mut state = inner.lock();
        if inner.is_canceled() {
            // The entry is gone unless the pass of the callbacks has not reached it yet.
            let removed = this.id.take().and_then(|id| state.entries.remove(&id));
            drop(state);
            drop(removed);
            return Poll::Ready(());
        }
        if let Some(Entry::Waker(waker)) = this.id.and_then(|id| state.entries.get_mut(&id)) {
            if !waker.will_wake(cx.waker()) {
                replaced = Some(std::mem::replace(waker, cx.waker().clone()));
            }
        } else {
            this.id = Some(state.insert(Entry::Waker(cx.waker().clone())));
        }
        drop(state);
        drop(replaced);
        Poll::Pending
    }
}

impl Drop for WaitForCancellation {
    fn drop(&mut self) {
        if let (Some(inner), Some(id)) = (&self.inner, self.id) {
            inner.remove(id);
        }
    }
}

impl fmt::Debug for WaitForCancellation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("WaitForCancellation")
    }
}

/// Cancels its token when it is dropped ([`CancellationToken::drop_guard`]), unless it is
/// disarmed.
#[must_use = "dropping the guard cancels the token at once"]
pub struct DropGuard {
    token: Option<CancellationToken>,
}

impl DropGuard {
    /// Gives the token back without canceling it.
    pub fn disarm(mut self) -> CancellationToken {
        self.token.take().expect("a drop guard holds its token until it is disarmed or dropped")
    }
}

impl Drop for DropGuard {
    fn drop(&mut self) {
        if let Some(token) = self.token.take() {
            token.cancel();
        }
    }
}

impl fmt::Debug for DropGuard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DropGuard").field("token", &self.token).finish()
    }
}

/// Signals to one or more [`CancellationToken`]s that they should be canceled.
///
/// A source is a [`CancellationToken`] that can be canceled ([`CancellationToken::new`]): its
/// [`token`](Self::token) is a clone of it, and canceling either cancels both.
#[deprecated(
    since = "0.5.0",
    note = "use `CancellationToken::new()`, which can be canceled, and its clones in the place of `token()`"
)]
#[derive(Clone)]
pub struct CancellationTokenSource {
    token: CancellationToken,
}

#[allow(deprecated)]
impl Default for CancellationTokenSource {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(deprecated)]
impl CancellationTokenSource {
    pub fn new() -> Self {
        Self { token: CancellationToken::new() }
    }

    /// The token observing this source.
    pub fn token(&self) -> CancellationToken {
        self.token.clone()
    }

    pub fn is_cancellation_requested(&self) -> bool {
        self.token.is_cancelled()
    }

    /// Requests cancellation and runs the registered callbacks on the
    /// calling thread. Subsequent calls do nothing.
    pub fn cancel(&self) {
        self.token.cancel();
    }
}

/// The token of the source.
#[allow(deprecated)]
impl From<CancellationTokenSource> for CancellationToken {
    fn from(source: CancellationTokenSource) -> Self {
        source.token
    }
}

/// A guard of the callback: dropping it removes the callback.
#[allow(deprecated)]
impl From<CancellationTokenRegistration> for CancellationGuard {
    fn from(registration: CancellationTokenRegistration) -> Self {
        let CancellationTokenRegistration { inner, id } = registration;
        CancellationGuard { registration: inner.map(|inner| (inner, id)), local: None }
    }
}

/// A guard of the callback: dropping it removes the callback.
#[allow(deprecated)]
impl From<LocalCancellationTokenRegistration> for CancellationGuard {
    fn from(registration: LocalCancellationTokenRegistration) -> Self {
        let LocalCancellationTokenRegistration { registration, local } = registration;
        let mut guard = CancellationGuard::from(registration);
        guard.local = Some(local.handle());
        guard
    }
}

/// A callback registered with [`CancellationToken::register_local_handle`]; it belongs to the
/// thread that registered it.
#[deprecated(
    since = "0.5.0",
    note = "use `CancellationGuard`, from `CancellationToken::register_local`; `detach()` keeps the callback"
)]
#[allow(deprecated)]
pub struct LocalCancellationTokenRegistration {
    registration: CancellationTokenRegistration,
    local: LocalCallback,
}

#[allow(deprecated)]
impl LocalCancellationTokenRegistration {
    /// Removes the callback so that it no longer runs on cancellation.
    pub fn dispose(&self) {
        self.registration.dispose();
        self.local.remove();
    }
}

/// A callback registered with [`CancellationToken::register_handle`]; dropping it keeps the
/// callback.
#[deprecated(
    since = "0.5.0",
    note = "use `CancellationGuard`, from `CancellationToken::register`; `detach()` keeps the callback"
)]
pub struct CancellationTokenRegistration {
    inner: Option<Arc<Inner>>,
    id: u64,
}

#[allow(deprecated)]
impl CancellationTokenRegistration {
    /// Removes the callback so that it no longer runs on cancellation.
    pub fn dispose(&self) {
        if let Some(inner) = &self.inner {
            inner.remove(self.id);
        }
    }
}

#[cfg(test)]
#[allow(deprecated)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn none_cannot_be_canceled() {
        let token = CancellationToken::none();
        assert!(!token.can_be_canceled());
        assert!(!token.is_cancellation_requested());
    }

    #[test]
    fn cancel_runs_callbacks_once() {
        let count = Arc::new(AtomicUsize::new(0));
        let source = CancellationTokenSource::new();
        let token = source.token();
        let c = count.clone();
        token.register_handle(move || {
            c.fetch_add(1, Ordering::SeqCst);
        });
        let c = count.clone();
        let removed = token.register_handle(move || {
            c.fetch_add(10, Ordering::SeqCst);
        });
        removed.dispose();
        source.cancel();
        source.cancel();
        assert!(token.is_cancellation_requested());
        assert_eq!(count.load(Ordering::SeqCst), 1);

        let c = count.clone();
        token.register_handle(move || {
            c.fetch_add(100, Ordering::SeqCst);
        });
        assert_eq!(count.load(Ordering::SeqCst), 101);
    }
}
