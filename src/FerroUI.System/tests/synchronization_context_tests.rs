//! Tests of `threading::SynchronizationContext`: the current context of a thread, and the context
//! as the thread poster of its thread.
//!
//! Each test runs on a thread of its own, so that the context it sets is not seen by another test
//! that the harness runs on the same thread.

use crate::threading::tasks::{run, run_pending_wakes};
use crate::threading::{current_synchronization_context, set_synchronization_context, CancellationTokenSource, SynchronizationContext};
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Condvar, Mutex};
use std::time::Duration;

crate::ferro_thread_local! {
    /// The work posted with `post_local` to a `QueueContext` of the thread.
    static LOCAL_WORK: RefCell<VecDeque<Box<dyn FnOnce()>>> = RefCell::new(VecDeque::new());
}

/// A context that queues the work it is posted and runs it when the test asks.
#[derive(Default)]
struct QueueContext {
    posted: Mutex<VecDeque<Box<dyn FnOnce() + Send>>>,
    arrived: Condvar,
    started: AtomicUsize,
    completed: AtomicUsize,
}

impl QueueContext {
    /// Waits for the next work posted with `post` and runs it on the calling thread.
    fn run_next(&self) {
        let mut posted = self.posted.lock().unwrap();
        loop {
            if let Some(work) = posted.pop_front() {
                drop(posted);
                work();
                return;
            }
            let (guard, timeout) = self.arrived.wait_timeout(posted, Duration::from_secs(10)).unwrap();
            assert!(!timeout.timed_out(), "nothing was posted");
            posted = guard;
        }
    }

    fn posted_count(&self) -> usize {
        self.posted.lock().unwrap().len()
    }
}

/// Runs the work posted with `post_local` on the calling thread; the number of items run.
fn run_local_work() -> usize {
    let mut count = 0;
    loop {
        let work = LOCAL_WORK.with_borrow_mut(|queue| queue.pop_front());
        let Some(work) = work else { return count };
        work();
        count += 1;
    }
}

impl SynchronizationContext for QueueContext {
    fn post(&self, work: Box<dyn FnOnce() + Send>) {
        self.posted.lock().unwrap().push_back(work);
        self.arrived.notify_all();
    }

    fn send(&self, work: Box<dyn FnOnce() + Send>) {
        work();
    }

    fn post_local(&self, work: Box<dyn FnOnce()>) {
        LOCAL_WORK.with_borrow_mut(|queue| queue.push_back(work));
    }

    fn send_local(&self, work: Box<dyn FnOnce()>) {
        work();
    }

    fn operation_started(&self) {
        self.started.fetch_add(1, Ordering::SeqCst);
    }

    fn operation_completed(&self) {
        self.completed.fetch_add(1, Ordering::SeqCst);
    }
}

/// A context that implements only the required members.
struct InlineContext;

impl SynchronizationContext for InlineContext {
    fn post(&self, work: Box<dyn FnOnce() + Send>) {
        work();
    }

    fn send(&self, work: Box<dyn FnOnce() + Send>) {
        work();
    }

    fn post_local(&self, work: Box<dyn FnOnce()>) {
        work();
    }

    fn send_local(&self, work: Box<dyn FnOnce()>) {
        work();
    }
}

fn same(a: &Arc<dyn SynchronizationContext>, b: &Arc<dyn SynchronizationContext>) -> bool {
    std::ptr::addr_eq(Arc::as_ptr(a), Arc::as_ptr(b))
}

/// Runs `test` on a new thread and waits for it.
fn on_own_thread(test: impl FnOnce() + Send + 'static) {
    std::thread::spawn(test).join().unwrap();
}

#[test]
fn a_thread_has_no_context_until_one_is_set() {
    on_own_thread(|| {
        assert!(current_synchronization_context().is_none());
        assert!(<dyn SynchronizationContext>::current().is_none());

        let context: Arc<dyn SynchronizationContext> = Arc::new(QueueContext::default());
        set_synchronization_context(Some(context.clone()));
        let current = current_synchronization_context().expect("the context is set");
        assert!(same(&current, &context));
        assert!(<dyn SynchronizationContext>::is_current(&context));

        let other: Arc<dyn SynchronizationContext> = Arc::new(InlineContext);
        assert!(!<dyn SynchronizationContext>::is_current(&other));
        <dyn SynchronizationContext>::set_synchronization_context(Some(other.clone()));
        assert!(<dyn SynchronizationContext>::is_current(&other));
        assert!(!<dyn SynchronizationContext>::is_current(&context));

        set_synchronization_context(None);
        assert!(current_synchronization_context().is_none());
        assert!(!<dyn SynchronizationContext>::is_current(&other));
    });
}

#[test]
fn the_context_belongs_to_the_thread_that_set_it() {
    on_own_thread(|| {
        let context: Arc<dyn SynchronizationContext> = Arc::new(InlineContext);
        set_synchronization_context(Some(context.clone()));

        let seen_elsewhere = std::thread::spawn(|| current_synchronization_context().is_some()).join().unwrap();
        assert!(!seen_elsewhere);
        assert!(<dyn SynchronizationContext>::is_current(&context));

        set_synchronization_context(None);
    });
}

#[test]
fn the_defaults_of_operation_started_and_completed_do_nothing() {
    let context = InlineContext;
    context.operation_started();
    context.operation_completed();

    let queue = QueueContext::default();
    queue.operation_started();
    queue.operation_completed();
    queue.operation_completed();
    assert_eq!(queue.started.load(Ordering::SeqCst), 1);
    assert_eq!(queue.completed.load(Ordering::SeqCst), 2);
}

#[test]
fn local_work_is_posted_and_sent_on_the_thread_of_the_context() {
    on_own_thread(|| {
        let context = Arc::new(QueueContext::default());
        let ran = Rc::new(Cell::new(0));

        let r = ran.clone();
        context.post_local(Box::new(move || r.set(r.get() + 1)));
        assert_eq!(ran.get(), 0);
        assert_eq!(run_local_work(), 1);
        assert_eq!(ran.get(), 1);

        let r = ran.clone();
        context.send_local(Box::new(move || r.set(r.get() + 10)));
        assert_eq!(ran.get(), 11);
    });
}

#[test]
fn a_wake_from_another_thread_is_posted_to_the_context() {
    on_own_thread(|| {
        let context = Arc::new(QueueContext::default());
        set_synchronization_context(Some(context.clone()));

        let (go, wait) = mpsc::channel::<()>();
        let task = run(move || {
            wait.recv().ok();
            Ok(5)
        });
        assert!(!task.is_completed());

        go.send(()).unwrap();
        // The work of the worker wakes the task from its thread: the wake is posted to the
        // context, and runs the task here when the context runs it.
        context.run_next();
        assert!(task.is_completed());
        assert_eq!(task.result().unwrap(), 5);

        set_synchronization_context(None);
    });
}

#[test]
fn a_cancellation_from_another_thread_is_posted_to_the_context() {
    on_own_thread(|| {
        let context = Arc::new(QueueContext::default());
        set_synchronization_context(Some(context.clone()));

        let source = CancellationTokenSource::new();
        let ran = Rc::new(Cell::new(false));
        let r = ran.clone();
        let _registration = source.token().register_local(move || r.set(true));

        let remote = source.clone();
        std::thread::spawn(move || remote.cancel()).join().unwrap();
        assert!(!ran.get());
        assert_eq!(context.posted_count(), 1);

        context.run_next();
        assert!(ran.get());

        set_synchronization_context(None);
    });
}

#[test]
fn clearing_the_context_removes_the_poster() {
    on_own_thread(|| {
        let context = Arc::new(QueueContext::default());
        set_synchronization_context(Some(context.clone()));
        set_synchronization_context(None);

        let source = CancellationTokenSource::new();
        let ran = Rc::new(Cell::new(false));
        let r = ran.clone();
        let _registration = source.token().register_local(move || r.set(true));

        let remote = source.clone();
        std::thread::spawn(move || remote.cancel()).join().unwrap();
        // Nothing reaches the context: the callback waits in the queue of the thread.
        assert_eq!(context.posted_count(), 0);
        assert!(!ran.get());
        assert!(run_pending_wakes());
        assert!(ran.get());
    });
}

#[test]
fn setting_a_context_runs_the_wakes_that_came_before() {
    on_own_thread(|| {
        let source = CancellationTokenSource::new();
        let ran = Rc::new(Cell::new(false));
        let r = ran.clone();
        let _registration = source.token().register_local(move || r.set(true));

        let remote = source.clone();
        std::thread::spawn(move || remote.cancel()).join().unwrap();
        assert!(!ran.get());

        // A poster set while wakes are queued runs them (`set_thread_poster`).
        let context = Arc::new(QueueContext::default());
        set_synchronization_context(Some(context.clone()));
        assert!(ran.get());

        set_synchronization_context(None);
    });
}
