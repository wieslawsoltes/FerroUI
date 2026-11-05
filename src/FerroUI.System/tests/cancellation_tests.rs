//! Tests of `threading::CancellationToken` in the shape of `tokio_util` (step A4 of phase C4 of
//! #423, docs/idiomatic-rust-async.md, section 3.7): the callbacks and their guards, the local
//! callbacks, child tokens, the future of the cancellation, the drop guard, reentrant callbacks,
//! the source over the token, and the conversions of the cancellation errors.

use crate::task::{block_on, run_pending, Canceled};
#[allow(deprecated)]
use crate::threading::tasks::TaskCanceledError;
#[allow(deprecated)]
use crate::threading::CancellationTokenSource;
use crate::threading::{CancellationGuard, CancellationToken, DropGuard, WaitForCancellation};
use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};
use std::thread;
use std::time::Duration;

/// A waker that counts its wakes.
#[derive(Default)]
struct CountingWaker(AtomicUsize);

impl CountingWaker {
    fn new() -> (Arc<Self>, Waker) {
        let counter = Arc::new(Self::default());
        let waker = Waker::from(counter.clone());
        (counter, waker)
    }

    fn count(&self) -> usize {
        self.0.load(Ordering::SeqCst)
    }
}

impl Wake for CountingWaker {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

/// Polls `future` once with `waker`.
fn poll_with<F: Future + Unpin>(future: &mut F, waker: &Waker) -> Poll<F::Output> {
    Pin::new(future).poll(&mut Context::from_waker(waker))
}

/// Runs `test` on a new thread, whose queue no other test sees, and waits for it.
fn on_own_thread(test: impl FnOnce() + Send + 'static) {
    thread::spawn(test).join().unwrap();
}

/// A log of the callbacks that ran, shared with them.
fn log() -> Arc<Mutex<Vec<u32>>> {
    Arc::new(Mutex::new(Vec::new()))
}

/// Registers a callback that writes `n` to `log`.
fn push(token: &CancellationToken, log: &Arc<Mutex<Vec<u32>>>, n: u32) -> CancellationGuard {
    let log = log.clone();
    token.register(move || log.lock().unwrap().push(n))
}

fn assert_send_sync<T: Send + Sync>() {}

fn assert_send<T: Send>() {}

#[test]
fn the_token_and_its_guards_are_send() {
    assert_send_sync::<CancellationToken>();
    assert_send::<CancellationGuard>();
    assert_send::<WaitForCancellation>();
    assert_send::<DropGuard>();
}

#[test]
#[allow(deprecated)]
fn a_new_token_is_canceled_once_and_runs_its_callbacks_in_the_order_of_registration() {
    let token = CancellationToken::new();
    assert!(token.can_be_cancelled());
    assert!(!token.is_cancelled());
    let log = log();
    let first = push(&token, &log, 0);
    let second = push(&token, &log, 1);
    let third = push(&token, &log, 2);
    let fourth = push(&token, &log, 3);
    // Removing one keeps the order of the others.
    second.dispose();

    let clone = token.clone();
    let canceller = thread::spawn(move || {
        clone.cancel();
        clone.cancel();
    });
    canceller.join().unwrap();
    assert!(token.is_cancelled());
    assert!(token.is_cancellation_requested());
    assert_eq!(*log.lock().unwrap(), [0, 2, 3]);

    // The guards of callbacks that ran do nothing.
    drop((first, third, fourth));
    token.cancel();
    assert_eq!(*log.lock().unwrap(), [0, 2, 3]);
}

#[test]
fn a_callback_registered_after_the_cancellation_runs_at_once_on_the_registering_thread() {
    let token = CancellationToken::new();
    token.cancel();
    let ran_on = Arc::new(Mutex::new(None));
    let r = ran_on.clone();
    let guard = token.register(move || *r.lock().unwrap() = Some(thread::current().id()));
    assert_eq!(*ran_on.lock().unwrap(), Some(thread::current().id()));
    drop(guard);
    assert_eq!(token.registration_count(), 0);
}

#[test]
fn dropping_the_guard_removes_the_callback_and_detach_keeps_it() {
    let token = CancellationToken::new();
    let log = log();
    drop(push(&token, &log, 0));
    push(&token, &log, 1).detach();
    let disposed = push(&token, &log, 2);
    disposed.dispose();
    assert_eq!(token.registration_count(), 1);

    token.cancel();
    assert_eq!(*log.lock().unwrap(), [1]);
}

#[test]
fn a_token_that_cannot_be_canceled_never_runs_its_callbacks() {
    let token = CancellationToken::none();
    assert!(!token.can_be_cancelled());
    let default = CancellationToken::default();
    assert!(!default.can_be_cancelled());
    let log = log();
    push(&token, &log, 0).detach();
    token.cancel();
    assert!(!token.is_cancelled());
    assert!(log.lock().unwrap().is_empty());

    let mut wait = token.cancelled();
    let (counter, waker) = CountingWaker::new();
    assert_eq!(poll_with(&mut wait, &waker), Poll::Pending);
    assert_eq!(counter.count(), 0);
}

#[test]
fn a_child_is_canceled_with_its_parent_and_not_the_other_way_round() {
    let parent = CancellationToken::new();
    let child = parent.child_token();
    let grandchild = child.child_token();
    let sibling = parent.child_token();
    assert!(child.can_be_cancelled());

    let log = log();
    let _child_callback = push(&child, &log, 1);
    sibling.cancel();
    assert!(sibling.is_cancelled());
    assert!(!parent.is_cancelled());
    assert!(!child.is_cancelled());

    let _parent_callback = push(&parent, &log, 0);
    parent.cancel();
    assert!(child.is_cancelled());
    assert!(grandchild.is_cancelled());
    // The child was made before the parent's callback: it is canceled first.
    assert_eq!(*log.lock().unwrap(), [1, 0]);

    // A child of a canceled token is canceled at once; a child of a token that cannot be canceled
    // is a token of its own.
    assert!(parent.child_token().is_cancelled());
    let orphan = CancellationToken::none().child_token();
    assert!(orphan.can_be_cancelled());
    orphan.cancel();
    assert!(orphan.is_cancelled());
}

#[test]
fn dropped_children_and_waits_do_not_stay_registered_with_the_parent() {
    let parent = CancellationToken::new();
    for _ in 0..100 {
        let child = parent.child_token();
        let log = log();
        // A registration keeps the child alive while it is.
        let guard = push(&child, &log, 0);
        drop(child);
        assert_eq!(parent.registration_count(), 1);
        drop(guard);
    }
    assert_eq!(parent.registration_count(), 0);

    // A child stays registered while one of its clones is alive.
    let child = parent.child_token();
    let clone = child.clone();
    drop(child);
    assert_eq!(parent.registration_count(), 1);
    drop(clone);
    assert_eq!(parent.registration_count(), 0);

    // A wait registers its waker when it is polled, and removes it when it is dropped.
    let (_, waker) = CountingWaker::new();
    for _ in 0..100 {
        let mut wait = parent.cancelled();
        assert_eq!(poll_with(&mut wait, &waker), Poll::Pending);
        assert_eq!(poll_with(&mut wait, &waker), Poll::Pending);
        assert_eq!(parent.registration_count(), 1);
    }
    assert_eq!(parent.registration_count(), 0);
}

#[test]
fn a_wait_is_woken_by_the_cancellation_with_the_waker_of_its_last_poll() {
    let token = CancellationToken::new();
    let mut wait = token.cancelled();
    let (first, first_waker) = CountingWaker::new();
    let (last, last_waker) = CountingWaker::new();
    assert_eq!(poll_with(&mut wait, &first_waker), Poll::Pending);
    assert_eq!(poll_with(&mut wait, &last_waker), Poll::Pending);

    token.cancel();
    assert_eq!((first.count(), last.count()), (0, 1));
    assert_eq!(poll_with(&mut wait, &last_waker), Poll::Ready(()));
    assert_eq!(token.registration_count(), 0);

    // A wait on a canceled token is ready at once.
    let mut late = token.cancelled();
    assert_eq!(poll_with(&mut late, &first_waker), Poll::Ready(()));
}

#[test]
fn a_wait_completes_when_another_thread_cancels_the_token() {
    on_own_thread(|| {
        let token = CancellationToken::new();
        let clone = token.clone();
        let canceller = thread::spawn(move || {
            thread::sleep(Duration::from_millis(20));
            clone.cancel();
        });
        block_on(token.cancelled());
        assert!(token.is_cancelled());
        canceller.join().unwrap();
    });
}

#[test]
fn a_drop_guard_cancels_its_token_unless_it_is_disarmed() {
    let token = CancellationToken::new();
    let guard = token.clone().drop_guard();
    assert!(!token.is_cancelled());
    drop(guard);
    assert!(token.is_cancelled());

    let token = CancellationToken::new();
    let disarmed = token.clone().drop_guard().disarm();
    assert!(!token.is_cancelled());
    disarmed.cancel();
    assert!(token.is_cancelled());
}

#[test]
fn a_callback_may_register_unregister_and_cancel_without_deadlocking() {
    let token = CancellationToken::new();
    let other = CancellationToken::new();
    let log = log();
    let later: Arc<Mutex<Option<CancellationGuard>>> = Arc::new(Mutex::new(None));

    let (t, o, l, slot) = (token.clone(), other.clone(), log.clone(), later.clone());
    let _reentrant = token.register(move || {
        l.lock().unwrap().push(0);
        // Canceling the token again does nothing; canceling another token runs its pass here.
        t.cancel();
        o.cancel();
        // A callback or a child registered during the pass is canceled at once.
        let l2 = l.clone();
        t.register(move || l2.lock().unwrap().push(1)).detach();
        assert!(t.child_token().is_cancelled());
        // A callback unregistered before the pass reaches it does not run.
        drop(slot.lock().unwrap().take());
    });
    let _other_callback = push(&other, &log, 2);
    *later.lock().unwrap() = Some(push(&token, &log, 3));
    let _last = push(&token, &log, 4);

    token.cancel();
    assert!(other.is_cancelled());
    assert_eq!(*log.lock().unwrap(), [0, 2, 1, 4]);
}

#[test]
fn a_local_callback_runs_on_its_thread_and_its_guard_removes_it_from_any_thread() {
    on_own_thread(|| {
        let token = CancellationToken::new();
        let log = Rc::new(RefCell::new(Vec::new()));
        let l = log.clone();
        let kept = token.register_local(move || l.borrow_mut().push("kept"));
        let l = log.clone();
        let dropped_here = token.register_local(move || l.borrow_mut().push("dropped here"));
        let owned = Rc::new(());
        let (l, o) = (log.clone(), owned.clone());
        let dropped_elsewhere = token.register_local(move || {
            let _owned = o;
            l.borrow_mut().push("dropped elsewhere");
        });
        drop(dropped_here);
        // Dropped on another thread, the guard removes the registration at once, and the callback
        // is dropped on its own thread when that thread runs its queue.
        thread::spawn(move || drop(dropped_elsewhere)).join().unwrap();
        assert_eq!(token.registration_count(), 1);
        assert_eq!(Rc::strong_count(&owned), 2);
        run_pending();
        assert_eq!(Rc::strong_count(&owned), 1);

        // Canceled on another thread, the callback waits for the queue of this thread.
        let clone = token.clone();
        thread::spawn(move || clone.cancel()).join().unwrap();
        assert!(log.borrow().is_empty());
        run_pending();
        assert_eq!(*log.borrow(), ["kept"]);
        drop(kept);

        // Registered on a canceled token, it runs at once.
        let l = log.clone();
        let _late = token.register_local(move || l.borrow_mut().push("late"));
        assert_eq!(*log.borrow(), ["kept", "late"]);
    });
}

#[test]
#[allow(deprecated)]
fn a_source_is_a_token_that_can_be_canceled_and_its_registrations_stay_when_dropped() {
    let source = CancellationTokenSource::new();
    let token = source.token();
    assert!(token.can_be_canceled());
    let count = Arc::new(AtomicUsize::new(0));
    let c = count.clone();
    // Dropping the registration of `register_handle` keeps the callback.
    drop(token.register_handle(move || {
        c.fetch_add(1, Ordering::SeqCst);
    }));
    let child = token.child_token();
    let wait = token.cancelled();

    source.cancel();
    assert!(source.is_cancellation_requested());
    assert!(token.is_cancelled());
    assert!(child.is_cancelled());
    assert_eq!(count.load(Ordering::SeqCst), 1);
    block_on(wait);

    // The token cancels its source as well.
    let source = CancellationTokenSource::new();
    source.token().cancel();
    assert!(source.is_cancellation_requested());
}

#[test]
#[allow(deprecated)]
fn the_cancellation_errors_convert_to_and_from_canceled() {
    let error: TaskCanceledError = Canceled.into();
    assert_eq!(error, TaskCanceledError::default());
    let canceled: Canceled = TaskCanceledError { task_id: Some(7) }.into();
    assert_eq!(canceled, Canceled);
}
