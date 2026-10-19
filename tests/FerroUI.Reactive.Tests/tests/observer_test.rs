//! Ported from `Tests.System.Reactive/Tests/ObserverTest.cs` of Rx.NET.
//! Adapted: an error delivered to an observer without a function for errors panics where upstream
//! rethrows the exception, so the tests check for the panic (`assert_throws`) and not for the
//! identity of the exception; `InvalidOperationException` of `Checked` is a panic too. The mutable
//! locals that the observer lambdas capture are shared cells. The overloads of
//! `Observer.Synchronize` are the methods `synchronize`, `synchronize_with_gate` (the gate is any
//! `Rc<dyn Any>`), `synchronize_prevent_reentrancy` and `synchronize_with_lock` of `ObserverExt`.
//! `Observer.ToProgress` is `to_progress` and `to_progress_on` of `ObserverExt`, and
//! `Observer.ToObserver(IProgress<T>)` is `to_observer` of `ProgressExt`; the list the observers add
//! to is a shared `RefCell`. `NotifyOn(SynchronizationContext)` is `notify_on_context`; the context
//! `MySyncCtx` of `NotifyOn_SyncCtx`, which posts to the task pool upstream, posts its `Send` work
//! to a new thread.

use ferroui_reactive::concurrency::AsyncLock;
use ferroui_reactive::{IObserver, Notification, NotificationKind, ObservableError, Observer, ObserverExt, ProgressExt};
use ferroui_reactive_testing::{assert_messages, assert_throws, new_error, ReactiveTest as RT, TestScheduler};
use ferroui_system::threading::SynchronizationContext;
use ferroui_system::IProgress;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

fn same_error(a: &ObservableError, b: &ObservableError) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b))
}

fn increment(counter: &Cell<i32>) {
    counter.set(counter.get() + 1);
}

#[test]
fn to_observer_notification_on_next() {
    let i = Cell::new(0);
    let next = move |n: Notification<i32>| {
        assert_eq!(i.replace(i.get() + 1), 0);
        assert_eq!(n.kind(), NotificationKind::OnNext);
        assert_eq!(n.value(), Some(&42));
        assert!(n.exception().is_none());
        assert!(n.has_value());
    };
    Observer::to_observer(next).on_next(42);
}

#[test]
fn to_observer_notification_on_error() {
    let ex = new_error("ex");
    let i = Cell::new(0);
    let expected = ex.clone();
    let next = move |n: Notification<i32>| {
        assert_eq!(i.replace(i.get() + 1), 0);
        assert_eq!(n.kind(), NotificationKind::OnError);
        assert!(same_error(n.exception().unwrap(), &expected));
        assert!(!n.has_value());
    };
    Observer::to_observer(next).on_error(ex);
}

#[test]
fn to_observer_notification_on_completed() {
    let i = Cell::new(0);
    let next = move |n: Notification<i32>| {
        assert_eq!(i.replace(i.get() + 1), 0);
        assert_eq!(n.kind(), NotificationKind::OnCompleted);
        assert!(!n.has_value());
    };
    Observer::to_observer(next).on_completed();
}

#[test]
fn to_notifier_forwards() {
    let obsn = Rc::new(MyObserver::default());
    let o: Rc<dyn IObserver<i32>> = obsn.clone();
    o.to_notifier()(Notification::OnNext(42));
    assert_eq!(obsn.has_on_next.get(), 42);

    let ex = new_error("ex");
    let obse = Rc::new(MyObserver::default());
    let o: Rc<dyn IObserver<i32>> = obse.clone();
    o.to_notifier()(Notification::OnError(ex.clone()));
    assert!(same_error(&ex, obse.has_on_error.borrow().as_ref().unwrap()));

    let obsc = Rc::new(MyObserver::default());
    let o: Rc<dyn IObserver<i32>> = obsc.clone();
    o.to_notifier()(Notification::OnCompleted);
    assert!(obsc.has_on_completed.get());
}

#[test]
fn create_on_next() {
    let next = Rc::new(Cell::new(false));
    let res = Observer::create({
        let next = next.clone();
        move |x: i32| {
            assert_eq!(x, 42);
            next.set(true);
        }
    });
    res.on_next(42);
    assert!(next.get());
    res.on_completed();
}

#[test]
fn create_on_next_has_error() {
    let ex = new_error("ex");

    let next = Rc::new(Cell::new(false));
    let res = Observer::create({
        let next = next.clone();
        move |x: i32| {
            assert_eq!(x, 42);
            next.set(true);
        }
    });
    res.on_next(42);
    assert!(next.get());

    assert_throws(|| res.on_error(ex.clone()));
}

#[test]
fn create_on_next_on_completed() {
    let next = Rc::new(Cell::new(false));
    let completed = Rc::new(Cell::new(false));
    let res = Observer::create_with_completed(
        {
            let next = next.clone();
            move |x: i32| {
                assert_eq!(x, 42);
                next.set(true);
            }
        },
        {
            let completed = completed.clone();
            move || completed.set(true)
        },
    );
    res.on_next(42);
    assert!(next.get());
    assert!(!completed.get());
    res.on_completed();
    assert!(completed.get());
}

#[test]
fn create_on_next_on_completed_has_error() {
    let ex = new_error("ex");

    let next = Rc::new(Cell::new(false));
    let completed = Rc::new(Cell::new(false));
    let res = Observer::create_with_completed(
        {
            let next = next.clone();
            move |x: i32| {
                assert_eq!(x, 42);
                next.set(true);
            }
        },
        {
            let completed = completed.clone();
            move || completed.set(true)
        },
    );
    res.on_next(42);
    assert!(next.get());
    assert!(!completed.get());
    assert_throws(|| res.on_error(ex.clone()));
    assert!(!completed.get());
}

#[test]
fn create_on_next_on_error() {
    let ex = new_error("ex");
    let next = Rc::new(Cell::new(true));
    let error = Rc::new(Cell::new(false));
    let res = Observer::create_with_error(
        {
            let next = next.clone();
            move |x: i32| {
                assert_eq!(x, 42);
                next.set(true);
            }
        },
        {
            let (ex, error) = (ex.clone(), error.clone());
            move |e| {
                assert!(same_error(&ex, &e));
                error.set(true);
            }
        },
    );
    res.on_next(42);
    assert!(next.get());
    assert!(!error.get());
    res.on_error(ex.clone());
    assert!(error.get());
}

#[test]
fn create_on_next_on_error_hit_completed() {
    let ex = new_error("ex");
    let next = Rc::new(Cell::new(true));
    let error = Rc::new(Cell::new(false));
    let res = Observer::create_with_error(
        {
            let next = next.clone();
            move |x: i32| {
                assert_eq!(x, 42);
                next.set(true);
            }
        },
        {
            let (ex, error) = (ex.clone(), error.clone());
            move |e| {
                assert!(same_error(&ex, &e));
                error.set(true);
            }
        },
    );
    res.on_next(42);
    assert!(next.get());
    assert!(!error.get());
    res.on_completed();
    assert!(!error.get());
}

#[test]
fn create_on_next_on_error_on_completed1() {
    let ex = new_error("ex");
    let next = Rc::new(Cell::new(true));
    let error = Rc::new(Cell::new(false));
    let completed = Rc::new(Cell::new(false));
    let res = Observer::create_all(
        {
            let next = next.clone();
            move |x: i32| {
                assert_eq!(x, 42);
                next.set(true);
            }
        },
        {
            let (ex, error) = (ex.clone(), error.clone());
            move |e| {
                assert!(same_error(&ex, &e));
                error.set(true);
            }
        },
        {
            let completed = completed.clone();
            move || completed.set(true)
        },
    );
    res.on_next(42);
    assert!(next.get());
    assert!(!error.get());
    assert!(!completed.get());
    res.on_completed();
    assert!(completed.get());
    assert!(!error.get());
}

#[test]
fn create_on_next_on_error_on_completed2() {
    let ex = new_error("ex");
    let next = Rc::new(Cell::new(true));
    let error = Rc::new(Cell::new(false));
    let completed = Rc::new(Cell::new(false));
    let res = Observer::create_all(
        {
            let next = next.clone();
            move |x: i32| {
                assert_eq!(x, 42);
                next.set(true);
            }
        },
        {
            let (ex, error) = (ex.clone(), error.clone());
            move |e| {
                assert!(same_error(&ex, &e));
                error.set(true);
            }
        },
        {
            let completed = completed.clone();
            move || completed.set(true)
        },
    );
    res.on_next(42);
    assert!(next.get());
    assert!(!error.get());
    assert!(!completed.get());
    res.on_error(ex.clone());
    assert!(error.get());
    assert!(!completed.get());
}

#[test]
fn as_observer_hides() {
    let obs: Rc<dyn IObserver<i32>> = Rc::new(MyObserver::default());
    let res = obs.as_observer();

    assert!(!std::ptr::addr_eq(Rc::as_ptr(&obs), Rc::as_ptr(&res)));
}

#[test]
fn as_observer_forwards() {
    let obsn = Rc::new(MyObserver::default());
    let o: Rc<dyn IObserver<i32>> = obsn.clone();
    o.as_observer().on_next(42);
    assert_eq!(obsn.has_on_next.get(), 42);

    let ex = new_error("ex");
    let obse = Rc::new(MyObserver::default());
    let o: Rc<dyn IObserver<i32>> = obse.clone();
    o.as_observer().on_error(ex.clone());
    assert!(same_error(&ex, obse.has_on_error.borrow().as_ref().unwrap()));

    let obsc = Rc::new(MyObserver::default());
    let o: Rc<dyn IObserver<i32>> = obsc.clone();
    o.as_observer().on_completed();
    assert!(obsc.has_on_completed.get());
}

#[derive(Default)]
struct MyObserver {
    has_on_next: Cell<i32>,
    has_on_error: RefCell<Option<ObservableError>>,
    has_on_completed: Cell<bool>,
}

impl IObserver<i32> for MyObserver {
    fn on_next(&self, value: i32) {
        self.has_on_next.set(value);
    }

    fn on_error(&self, error: ObservableError) {
        *self.has_on_error.borrow_mut() = Some(error);
    }

    fn on_completed(&self) {
        self.has_on_completed.set(true);
    }
}

#[test]
fn observer_checked_already_terminated_completed() {
    let m = Rc::new(Cell::new(0));
    let n = Rc::new(Cell::new(0));

    let o = Observer::create_all(
        {
            let m = m.clone();
            move |_: i32| increment(&m)
        },
        |_| panic!("unexpected OnError"),
        {
            let n = n.clone();
            move || increment(&n)
        },
    )
    .checked();

    o.on_next(1);
    o.on_next(2);
    o.on_completed();

    assert_throws(|| o.on_completed());
    assert_throws(|| o.on_error(new_error("ex")));

    assert_eq!(m.get(), 2);
    assert_eq!(n.get(), 1);
}

#[test]
fn observer_checked_already_terminated_error() {
    let m = Rc::new(Cell::new(0));
    let n = Rc::new(Cell::new(0));

    let o = Observer::create_all(
        {
            let m = m.clone();
            move |_: i32| increment(&m)
        },
        {
            let n = n.clone();
            move |_| increment(&n)
        },
        || panic!("unexpected OnCompleted"),
    )
    .checked();

    o.on_next(1);
    o.on_next(2);
    o.on_error(new_error("ex"));

    assert_throws(|| o.on_completed());
    assert_throws(|| o.on_error(new_error("ex")));

    assert_eq!(m.get(), 2);
    assert_eq!(n.get(), 1);
}

type ObserverSlot = Rc<RefCell<Option<Rc<dyn IObserver<i32>>>>>;

fn assert_reentrant_calls_throw(slot: &ObserverSlot) {
    let o = slot.borrow().clone().expect("the checked observer");
    assert_throws(|| o.on_next(9));
    assert_throws(|| o.on_error(new_error("ex")));
    assert_throws(|| o.on_completed());
}

#[test]
fn observer_checked_reentrant_next() {
    let n = Rc::new(Cell::new(0));

    let slot: ObserverSlot = Rc::new(RefCell::new(None));
    let o = Observer::create_all(
        {
            let (n, slot) = (n.clone(), slot.clone());
            move |_: i32| {
                increment(&n);

                assert_reentrant_calls_throw(&slot);
            }
        },
        |_| panic!("unexpected OnError"),
        || panic!("unexpected OnCompleted"),
    )
    .checked();
    *slot.borrow_mut() = Some(o.clone());

    o.on_next(1);

    assert_eq!(n.get(), 1);
}

#[test]
fn observer_checked_reentrant_error() {
    let n = Rc::new(Cell::new(0));

    let slot: ObserverSlot = Rc::new(RefCell::new(None));
    let o = Observer::create_all(
        |_: i32| panic!("unexpected OnNext"),
        {
            let (n, slot) = (n.clone(), slot.clone());
            move |_| {
                increment(&n);

                assert_reentrant_calls_throw(&slot);
            }
        },
        || panic!("unexpected OnCompleted"),
    )
    .checked();
    *slot.borrow_mut() = Some(o.clone());

    o.on_error(new_error("ex"));

    assert_eq!(n.get(), 1);
}

#[test]
fn observer_checked_reentrant_completed() {
    let n = Rc::new(Cell::new(0));

    let slot: ObserverSlot = Rc::new(RefCell::new(None));
    let o = Observer::create_all(
        |_: i32| panic!("unexpected OnNext"),
        |_| panic!("unexpected OnError"),
        {
            let (n, slot) = (n.clone(), slot.clone());
            move || {
                increment(&n);

                assert_reentrant_calls_throw(&slot);
            }
        },
    )
    .checked();
    *slot.borrow_mut() = Some(o.clone());

    o.on_completed();

    assert_eq!(n.get(), 1);
}

/// The observer of the re-entrancy tests of `Synchronize`: the value 1 sends 2 to the
/// synchronized observer in `s` (with `inOne` set while it does), and the value 2 sets `res` to
/// `record(inOne)`.
fn reentrant_observer(s: &ObserverSlot, res: &Rc<Cell<bool>>, record: fn(bool) -> bool) -> Rc<dyn IObserver<i32>> {
    let in_one = Rc::new(Cell::new(false));
    let (s, res) = (s.clone(), res.clone());
    Observer::create(move |x: i32| {
        if x == 1 {
            in_one.set(true);
            let synchronized = s.borrow().clone().expect("the synchronized observer");
            synchronized.on_next(2);
            in_one.set(false);
        } else if x == 2 {
            res.set(record(in_one.get()));
        }
    })
}

#[test]
fn observer_synchronize_monitor_reentrant1() {
    let res = Rc::new(Cell::new(false));

    let s: ObserverSlot = Rc::new(RefCell::new(None));

    let o = reentrant_observer(&s, &res, |in_one| in_one);

    let synchronized = o.synchronize();
    *s.borrow_mut() = Some(synchronized.clone());

    synchronized.on_next(1);

    assert!(res.get());
}

#[test]
fn observer_synchronize_monitor_reentrant2() {
    let res = Rc::new(Cell::new(false));

    let s: ObserverSlot = Rc::new(RefCell::new(None));

    let o = reentrant_observer(&s, &res, |in_one| in_one);

    let synchronized = o.synchronize_with_gate(Rc::new(()));
    *s.borrow_mut() = Some(synchronized.clone());

    synchronized.on_next(1);

    assert!(res.get());
}

#[test]
fn observer_synchronize_monitor_reentrant3() {
    let res = Rc::new(Cell::new(false));

    let s: ObserverSlot = Rc::new(RefCell::new(None));

    let o = reentrant_observer(&s, &res, |in_one| in_one);

    let synchronized = o.synchronize_prevent_reentrancy(false);
    *s.borrow_mut() = Some(synchronized.clone());

    synchronized.on_next(1);

    assert!(res.get());
}

#[test]
fn observer_synchronize_async_lock_non_reentrant1() {
    let res = Rc::new(Cell::new(false));

    let s: ObserverSlot = Rc::new(RefCell::new(None));

    let o = reentrant_observer(&s, &res, |in_one| !in_one);

    let synchronized = o.synchronize_with_lock(Rc::new(AsyncLock::new()));
    *s.borrow_mut() = Some(synchronized.clone());

    synchronized.on_next(1);

    assert!(res.get());
}

#[test]
fn observer_synchronize_async_lock_non_reentrant2() {
    let res = Rc::new(Cell::new(false));

    let s: ObserverSlot = Rc::new(RefCell::new(None));

    let o = reentrant_observer(&s, &res, |in_one| !in_one);

    let synchronized = o.synchronize_prevent_reentrancy(true);
    *s.borrow_mut() = Some(synchronized.clone());

    synchronized.on_next(1);

    assert!(res.get());
}

#[test]
fn observer_synchronize_async_lock() {
    {
        let res = Rc::new(Cell::new(false));

        let o = {
            let res = res.clone();
            Observer::create_all(move |x: i32| res.set(x == 1), |_| panic!("unexpected OnError"), || panic!("unexpected OnCompleted"))
        };

        let s = o.synchronize_prevent_reentrancy(true);

        s.on_next(1);

        assert!(res.get());
    }

    {
        let res: Rc<RefCell<Option<ObservableError>>> = Rc::new(RefCell::new(None));

        let err = new_error("err");

        let o = {
            let res = res.clone();
            Observer::create_all(|_: i32| panic!("unexpected OnNext"), move |ex| *res.borrow_mut() = Some(ex), || panic!("unexpected OnCompleted"))
        };

        let s = o.synchronize_prevent_reentrancy(true);

        s.on_error(err.clone());

        assert!(same_error(&err, res.borrow().as_ref().expect("the error is received")));
    }

    {
        let res = Rc::new(Cell::new(false));

        let o = {
            let res = res.clone();
            Observer::create_all(|_: i32| panic!("unexpected OnNext"), |_| panic!("unexpected OnError"), move || res.set(true))
        };

        let s = o.synchronize_prevent_reentrancy(true);

        s.on_completed();

        assert!(res.get());
    }
}

#[test]
fn notify_on_sync_ctx() {
    let lst = Rc::new(RefCell::new(Vec::new()));
    let don = Rc::new(Cell::new(false));
    let obs = {
        let (lst, don) = (lst.clone(), don.clone());
        Observer::create_all(move |x: i32| lst.borrow_mut().push(x), |_| panic!("unexpected OnError"), move || don.set(true))
    };
    let ctx = Arc::new(MySyncCtx);
    let _res = obs.notify_on_context(ctx);

    // As upstream, the notifications go to `obs` itself, not to the observer of the context.
    for i in 0..100 {
        obs.on_next(i);
    }

    obs.on_completed();

    assert!(don.get());
    assert_eq!((0..100).collect::<Vec<_>>(), *lst.borrow());
}

/// `MySyncCtx`: posts the work it is given to another thread (`Task.Run`). Work that is not `Send`
/// cannot leave the thread of the context, and runs at once.
struct MySyncCtx;

impl SynchronizationContext for MySyncCtx {
    fn post(&self, work: Box<dyn FnOnce() + Send>) {
        std::thread::spawn(work);
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

#[test]
fn observer_to_progress() {
    let xs = Rc::new(RefCell::new(Vec::new()));

    let p = {
        let xs = xs.clone();
        Observer::create(move |x: i32| xs.borrow_mut().push(x)).to_progress()
    };

    p.report(42);
    p.report(43);

    assert_eq!(vec![42, 43], *xs.borrow());
}

#[test]
fn observer_to_progress_scheduler() {
    let s = TestScheduler::new();

    let o = s.create_observer::<i32>();
    let observer: Rc<dyn IObserver<i32>> = o.clone();
    let p = observer.to_progress_on(s.scheduler());

    s.schedule_absolute_action(200, move || {
        p.report(42);
        p.report(43);
    });

    s.run();

    assert_messages!(o.messages(), RT::on_next(201, 42), RT::on_next(202, 43));
}

#[test]
fn progress_to_observer() {
    let xs = Rc::new(RefCell::new(Vec::new()));

    let p: Rc<dyn IProgress<i32>> = {
        let xs = xs.clone();
        Rc::new(MyProgress::new(move |x: i32| xs.borrow_mut().push(x)))
    };

    let o = p.to_observer();

    o.on_next(42);
    o.on_next(43);

    assert_eq!(vec![42, 43], *xs.borrow());
}

/// A progress object made of a function (`MyProgress<T>`).
struct MyProgress<T> {
    report: Box<dyn Fn(T)>,
}

impl<T> MyProgress<T> {
    fn new(report: impl Fn(T) + 'static) -> Self {
        Self { report: Box::new(report) }
    }
}

impl<T> IProgress<T> for MyProgress<T> {
    fn report(&self, value: T) {
        (self.report)(value);
    }
}

// Not ported: ToObserver_ArgumentChecking - argument validation
// Not ported: ToNotifier_ArgumentChecking - argument validation
// Not ported: Create_ArgumentChecking - argument validation
// Not ported: AsObserver_ArgumentChecking - argument validation
// Not ported: Observer_Checked_ArgumentChecking - argument validation
// Not ported: Observer_Synchronize_ArgumentChecking - argument validation
// Not ported: Observer_Synchronize_OnCompleted - real threads (Scheduler.Default, CountdownEvent)
// Not ported: Observer_Synchronize_OnError - real threads (Scheduler.Default, CountdownEvent)
// Not ported: NotifyOn_Null - argument validation
// Not ported: NotifyOn_Scheduler_OnCompleted - real threads (ThreadPoolScheduler, Thread, ManualResetEvent)
// Not ported: NotifyOn_Scheduler_OnError - real threads (ThreadPoolScheduler, Thread, ManualResetEvent)
// Not ported: Observer_ToProgress_ArgumentChecking - argument validation
// Not ported: Progress_ToObserver_ArgumentChecking - argument validation
