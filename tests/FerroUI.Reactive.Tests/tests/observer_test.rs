//! Ported from `Tests.System.Reactive/Tests/ObserverTest.cs` of Rx.NET.
//! Adapted: an error delivered to an observer without a function for errors panics where upstream
//! rethrows the exception, so the tests check for the panic (`assert_throws`) and not for the
//! identity of the exception; `InvalidOperationException` of `Checked` is a panic too. The mutable
//! locals that the observer lambdas capture are shared cells.

use ferroui_reactive::{IObserver, Notification, NotificationKind, ObservableError, Observer, ObserverExt};
use ferroui_reactive_testing::{assert_throws, new_error};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

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

// Not ported: ToObserver_ArgumentChecking - argument validation
// Not ported: ToNotifier_ArgumentChecking - argument validation
// Not ported: Create_ArgumentChecking - argument validation
// Not ported: AsObserver_ArgumentChecking - argument validation
// Not ported: Observer_Checked_ArgumentChecking - argument validation
// Not ported: Observer_Synchronize_ArgumentChecking - argument validation
// Not ported: Observer_Synchronize_Monitor_Reentrant1 - Observer.Synchronize(IObserver<T>) is not in the crate
// Not ported: Observer_Synchronize_Monitor_Reentrant2 - Observer.Synchronize(IObserver<T>, object) is not in the crate
// Not ported: Observer_Synchronize_Monitor_Reentrant3 - Observer.Synchronize(IObserver<T>, bool) is not in the crate
// Not ported: Observer_Synchronize_AsyncLock_NonReentrant1 - Observer.Synchronize(IObserver<T>, AsyncLock) is not in the crate
// Not ported: Observer_Synchronize_AsyncLock_NonReentrant2 - Observer.Synchronize(IObserver<T>, bool) is not in the crate
// Not ported: Observer_Synchronize_AsyncLock - Observer.Synchronize(IObserver<T>, bool) is not in the crate
// Not ported: Observer_Synchronize_OnCompleted - real threads (Scheduler.Default, CountdownEvent); Observer.Synchronize is not in the crate
// Not ported: Observer_Synchronize_OnError - real threads (Scheduler.Default, CountdownEvent); Observer.Synchronize is not in the crate
// Not ported: NotifyOn_Null - argument validation
// Not ported: NotifyOn_Scheduler_OnCompleted - real threads (ThreadPoolScheduler, Thread, ManualResetEvent); Observer.NotifyOn is not in the crate
// Not ported: NotifyOn_Scheduler_OnError - real threads (ThreadPoolScheduler, Thread, ManualResetEvent); Observer.NotifyOn is not in the crate
// Not ported: NotifyOn_SyncCtx - real threads (SynchronizationContext posting to Task.Run, ManualResetEvent); Observer.NotifyOn is not in the crate
// Not ported: Observer_ToProgress_ArgumentChecking - argument validation
// Not ported: Observer_ToProgress - Observer.ToProgress and IProgress<T> are not in the crate
// Not ported: Observer_ToProgress_Scheduler - Observer.ToProgress(IObserver<T>, IScheduler) and IProgress<T> are not in the crate
// Not ported: Progress_ToObserver_ArgumentChecking - argument validation
// Not ported: Progress_ToObserver - Observer.ToObserver(IProgress<T>) and IProgress<T> are not in the crate
