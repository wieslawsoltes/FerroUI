//! Ported from `Tests.System.Reactive/Tests/Concurrency/SynchronizationContextSchedulerTest.cs` of Rx.NET.
//! Adapted: the context `MySync` runs the work it is posted at once, from `post` and `post_local`
//! alike (the scheduler posts its work with `post_local`), and counts with atomics, since a context
//! is shared between threads; `Send` panics, where upstream throws `NotImplementedException`. The
//! delayed work of the scheduler waits on the default scheduler of the thread, the current-thread
//! scheduler, which blocks for the delay, so the tests that wait for a `ManualResetEvent` check a
//! flag once the scheduling has returned. `DontPost_Same` and `AlwaysPost_Same` start a thread to
//! set the context as current there, as upstream does; the scheduler and its work stay on that
//! thread.

use ferroui_reactive::concurrency::{IScheduler, SchedulerExt, SynchronizationContextScheduler};
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive_testing::assert_throws;
use ferroui_system::threading::{set_synchronization_context, SynchronizationContext};
use std::cell::Cell;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

/// `MySync`: runs the work it is posted at once, counting the posts and the operations.
#[derive(Default)]
struct MySync {
    count: AtomicUsize,
    started: AtomicUsize,
    completed: AtomicUsize,
}

impl MySync {
    fn count(&self) -> usize {
        self.count.load(Ordering::SeqCst)
    }

    fn started(&self) -> usize {
        self.started.load(Ordering::SeqCst)
    }

    fn completed(&self) -> usize {
        self.completed.load(Ordering::SeqCst)
    }
}

impl SynchronizationContext for MySync {
    fn post(&self, work: Box<dyn FnOnce() + Send>) {
        self.count.fetch_add(1, Ordering::SeqCst);
        work();
    }

    fn send(&self, _work: Box<dyn FnOnce() + Send>) {
        panic!("The method or operation is not implemented.");
    }

    fn post_local(&self, work: Box<dyn FnOnce()>) {
        self.count.fetch_add(1, Ordering::SeqCst);
        work();
    }

    fn send_local(&self, _work: Box<dyn FnOnce()>) {
        panic!("The method or operation is not implemented.");
    }

    fn operation_started(&self) {
        self.started.fetch_add(1, Ordering::SeqCst);
    }

    fn operation_completed(&self) {
        self.completed.fetch_add(1, Ordering::SeqCst);
    }
}

fn scheduler_of(ms: &Arc<MySync>) -> Rc<dyn IScheduler> {
    SynchronizationContextScheduler::new(ms.clone())
}

#[test]
fn synchronization_context_now() {
    let ms = Arc::new(MySync::default());
    let s = scheduler_of(&ms);

    let now = SystemTime::now();
    let res = match s.now().duration_since(now) {
        Ok(ahead) => ahead,
        Err(behind) => behind.duration(),
    };
    assert!(res < Duration::from_secs(1));
}

#[test]
fn synchronization_context_schedule_action() {
    let ms = Arc::new(MySync::default());
    let s = scheduler_of(&ms);

    let ran = Rc::new(Cell::new(false));
    let r = ran.clone();
    s.schedule_action(move || r.set(true));
    assert!(ms.count() == 1);
    assert!(ran.get());
}

#[test]
fn synchronization_context_schedule_action_time_span() {
    let ms = Arc::new(MySync::default());
    let s = scheduler_of(&ms);

    let e = Rc::new(Cell::new(false));
    let set = e.clone();
    s.schedule_action_after(Duration::from_millis(1), move || set.set(true));

    assert!(e.get());
    assert!(ms.count() == 1);
}

#[test]
fn synchronization_context_schedule_action_date_time_offset() {
    let ms = Arc::new(MySync::default());
    let s = scheduler_of(&ms);

    let e = Rc::new(Cell::new(false));
    let set = e.clone();
    s.schedule_action_at(SystemTime::now() + Duration::from_millis(100), move || set.set(true));

    assert!(e.get());
    assert!(ms.count() >= 1); // Can be > 1 in case of timer queue retry operations.
}

#[test]
fn synchronization_context_schedule_action_error() {
    let ms = Arc::new(MySync::default());
    let s = scheduler_of(&ms);

    // The action panics, where upstream's throws its exception out of `Schedule`.
    assert_throws(|| {
        s.schedule_action(|| panic!("ex"));
    });

    assert!(ms.count() == 1);
}

#[test]
fn synchronization_context_schedule_action_due() {
    let ms = Arc::new(MySync::default());
    let s = scheduler_of(&ms);

    let evt = Rc::new(Cell::new(false));
    let elapsed = Rc::new(Cell::new(Duration::ZERO));
    let sw = Instant::now();
    let (set, stop) = (evt.clone(), elapsed.clone());
    s.schedule_action_after(Duration::from_millis(200), move || {
        stop.set(sw.elapsed());
        set.set(true);
    });
    assert!(evt.get());
    assert!(elapsed.get() > Duration::from_millis(180), "due {:?}", elapsed.get());
    assert!(ms.count() == 1);
}

#[test]
fn synchronization_context_started_completed() {
    let ms = Arc::new(MySync::default());
    let s = scheduler_of(&ms);

    let started = Rc::new(Cell::new(0));
    let (st, context) = (started.clone(), ms.clone());
    s.schedule_after(
        Duration::ZERO,
        Box::new(move |_: &Rc<dyn IScheduler>| {
            st.set(context.started());
            Disposable::empty()
        }),
    );

    assert!(started.get() == 1);
    assert!(ms.count() == 1);
    assert!(ms.completed() == 1);
}

#[test]
fn synchronization_context_dont_post_different() {
    let ms = Arc::new(MySync::default());
    let s: Rc<dyn IScheduler> = SynchronizationContextScheduler::with_always_post(ms.clone(), false);

    let ran = Rc::new(Cell::new(false));
    let r = ran.clone();
    s.schedule_action(move || r.set(true));
    assert!(ms.count() == 1);
    assert!(ran.get());
}

#[test]
fn synchronization_context_dont_post_same() {
    let (count, ran) = std::thread::spawn(|| {
        let ms = Arc::new(MySync::default());
        set_synchronization_context(Some(ms.clone()));

        let s: Rc<dyn IScheduler> = SynchronizationContextScheduler::with_always_post(ms.clone(), false);

        let ran = Rc::new(Cell::new(false));
        let r = ran.clone();
        s.schedule_action(move || r.set(true));
        let count = ms.count();

        set_synchronization_context(None);
        (count, ran.get())
    })
    .join()
    .unwrap();

    assert!(count == 0 /* no post */);
    assert!(ran);
}

#[test]
fn synchronization_context_always_post_different() {
    let ms = Arc::new(MySync::default());
    let s: Rc<dyn IScheduler> = SynchronizationContextScheduler::with_always_post(ms.clone(), true);

    let ran = Rc::new(Cell::new(false));
    let r = ran.clone();
    s.schedule_action(move || r.set(true));
    assert!(ms.count() == 1);
    assert!(ran.get());
}

#[test]
fn synchronization_context_always_post_same() {
    let (count, ran) = std::thread::spawn(|| {
        let ms = Arc::new(MySync::default());
        set_synchronization_context(Some(ms.clone()));

        let s: Rc<dyn IScheduler> = SynchronizationContextScheduler::with_always_post(ms.clone(), true);

        let ran = Rc::new(Cell::new(false));
        let r = ran.clone();
        s.schedule_action(move || r.set(true));
        let count = ms.count();

        set_synchronization_context(None);
        (count, ran.get())
    })
    .join()
    .unwrap();

    assert!(count == 1 /* post */);
    assert!(ran);
}

// Not ported: SynchronizationContext_ArgumentChecking - argument validation
