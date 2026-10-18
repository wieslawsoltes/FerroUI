//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/CreateTest.cs` of Rx.NET.
//! Adapted: `Create(Func<IObserver<T>, Action>)` is `Observable::create_with_action` and
//! `Create(Func<IObserver<T>, IDisposable>)` is `Observable::create`; an exception thrown by the
//! subscription or by the observer is a panic, checked with `assert_throws`; the `stopped` flag of
//! `Create_Dispose` is a shared `Cell`.

use ferroui_reactive::concurrency::SchedulerExt;
use ferroui_reactive::disposables::{BooleanDisposable, Disposable, ICancelable};
use ferroui_reactive::{IDisposable, IObserver, Observable, ObservableExt};
use ferroui_reactive_testing::{assert_messages, assert_throws, new_error, ticks_to_duration, ReactiveTest as RT, TestScheduler};
use std::cell::Cell;
use std::rc::Rc;

#[test]
fn create_next() {
    let scheduler = TestScheduler::new();

    let res = scheduler.start(|| {
        Observable::create_with_action(|o: Rc<dyn IObserver<i32>>| {
            o.on_next(1);
            o.on_next(2);
            || {}
        })
    });

    assert_messages!(res.messages(), RT::on_next(200, 1), RT::on_next(200, 2));
}

#[test]
fn create_completed() {
    let scheduler = TestScheduler::new();

    let res = scheduler.start(|| {
        Observable::create_with_action(|o: Rc<dyn IObserver<i32>>| {
            o.on_completed();
            o.on_next(100);
            o.on_error(new_error("ex"));
            o.on_completed();
            || {}
        })
    });

    assert_messages!(res.messages(), RT::on_completed::<i32>(200));
}

#[test]
fn create_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let error = ex.clone();
    let res = scheduler.start(move || {
        Observable::create_with_action(move |o: Rc<dyn IObserver<i32>>| {
            o.on_error(error.clone());
            o.on_next(100);
            o.on_error(new_error("other"));
            o.on_completed();
            || {}
        })
    });

    assert_messages!(res.messages(), RT::on_error::<i32>(200, ex));
}

#[test]
fn create_exception() {
    assert_throws(|| {
        Observable::create_with_action(|_o: Rc<dyn IObserver<i32>>| -> fn() { panic!("InvalidOperationException") }).subscribe_ignore();
    });
}

#[test]
fn create_dispose() {
    let scheduler = TestScheduler::new();

    let s = scheduler.clone();
    let res = scheduler.start(move || {
        Observable::create_with_action(move |o: Rc<dyn IObserver<i32>>| {
            let stopped = Rc::new(Cell::new(false));

            o.on_next(1);
            o.on_next(2);
            for (ticks, value) in [(600, 3), (700, 4), (900, 5), (1100, 6)] {
                let (o, stopped) = (o.clone(), stopped.clone());
                s.scheduler().schedule_action_after(ticks_to_duration(ticks), move || {
                    if !stopped.get() {
                        o.on_next(value);
                    }
                });
            }

            move || stopped.set(true)
        })
    });

    assert_messages!(res.messages(), RT::on_next(200, 1), RT::on_next(200, 2), RT::on_next(800, 3), RT::on_next(900, 4));
}

#[test]
fn create_observer_throws() {
    assert_throws(|| {
        Observable::create_with_action(|o: Rc<dyn IObserver<i32>>| {
            o.on_next(1);
            || {}
        })
        .subscribe_next(|_| panic!("InvalidOperationException"));
    });
    assert_throws(|| {
        Observable::create_with_action(|o: Rc<dyn IObserver<i32>>| {
            o.on_error(new_error("ex"));
            || {}
        })
        .subscribe_next_error(|_| {}, |_| panic!("InvalidOperationException"));
    });
    assert_throws(|| {
        Observable::create_with_action(|o: Rc<dyn IObserver<i32>>| {
            o.on_completed();
            || {}
        })
        .subscribe_all(|_| {}, |_| {}, || panic!("InvalidOperationException"));
    });
}

#[test]
fn create_with_disposable_next() {
    let scheduler = TestScheduler::new();

    let res = scheduler.start(|| {
        Observable::create(|o: Rc<dyn IObserver<i32>>| {
            o.on_next(1);
            o.on_next(2);
            Disposable::empty()
        })
    });

    assert_messages!(res.messages(), RT::on_next(200, 1), RT::on_next(200, 2));
}

#[test]
fn create_with_disposable_completed() {
    let scheduler = TestScheduler::new();

    let res = scheduler.start(|| {
        Observable::create(|o: Rc<dyn IObserver<i32>>| {
            o.on_completed();
            o.on_next(100);
            o.on_error(new_error("ex"));
            o.on_completed();
            Disposable::empty()
        })
    });

    assert_messages!(res.messages(), RT::on_completed::<i32>(200));
}

#[test]
fn create_with_disposable_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let error = ex.clone();
    let res = scheduler.start(move || {
        Observable::create(move |o: Rc<dyn IObserver<i32>>| {
            o.on_error(error.clone());
            o.on_next(100);
            o.on_error(new_error("other"));
            o.on_completed();
            Disposable::empty()
        })
    });

    assert_messages!(res.messages(), RT::on_error::<i32>(200, ex));
}

#[test]
fn create_with_disposable_exception() {
    assert_throws(|| {
        Observable::create(|_o: Rc<dyn IObserver<i32>>| -> Rc<dyn IDisposable> { panic!("InvalidOperationException") }).subscribe_ignore();
    });
}

#[test]
fn create_with_disposable_dispose() {
    let scheduler = TestScheduler::new();

    let s = scheduler.clone();
    let res = scheduler.start(move || {
        Observable::create(move |o: Rc<dyn IObserver<i32>>| {
            let d = BooleanDisposable::new();

            o.on_next(1);
            o.on_next(2);
            for (ticks, value) in [(600, 3), (700, 4), (900, 5), (1100, 6)] {
                let (o, d) = (o.clone(), d.clone());
                s.scheduler().schedule_action_after(ticks_to_duration(ticks), move || {
                    if !d.is_disposed() {
                        o.on_next(value);
                    }
                });
            }

            let d: Rc<dyn IDisposable> = d;
            d
        })
    });

    assert_messages!(res.messages(), RT::on_next(200, 1), RT::on_next(200, 2), RT::on_next(800, 3), RT::on_next(900, 4));
}

#[test]
fn create_with_disposable_observer_throws() {
    assert_throws(|| {
        Observable::create(|o: Rc<dyn IObserver<i32>>| {
            o.on_next(1);
            Disposable::empty()
        })
        .subscribe_next(|_| panic!("InvalidOperationException"));
    });
    assert_throws(|| {
        Observable::create(|o: Rc<dyn IObserver<i32>>| {
            o.on_error(new_error("ex"));
            Disposable::empty()
        })
        .subscribe_next_error(|_| {}, |_| panic!("InvalidOperationException"));
    });
    assert_throws(|| {
        Observable::create(|o: Rc<dyn IObserver<i32>>| {
            o.on_completed();
            Disposable::empty()
        })
        .subscribe_all(|_| {}, |_| {}, || panic!("InvalidOperationException"));
    });
}

// Not ported: Create_ArgumentChecking - argument validation
// Not ported: Create_NullCoalescingAction - a null action returned by the subscription (the closure of the port returns an action, which cannot be null)
// Not ported: CreateWithDisposable_ArgumentChecking - argument validation
// Not ported: CreateWithDisposable_NullCoalescingAction - a null disposable returned by the subscription (the closure of the port returns an `Rc`, which cannot be null)
// Not ported: Iterate_ArgumentChecking - argument validation
// Not ported: Iterate_Complete - needs operator Create of an iterator (`ObservableEx.Create`, built on Concat of an enumerable of sequences) (phase 3, later)
// Not ported: Iterate_Complete_Implicit - needs operator Create of an iterator (`ObservableEx.Create`, built on Concat) (phase 3, later)
// Not ported: Iterate_Iterator_Throw - needs operator Create of an iterator (`ObservableEx.Create`, built on Concat) (phase 3, later)
// Not ported: Iterate_Iterator_Error - needs operator Create of an iterator (`ObservableEx.Create`, built on Concat) (phase 3, later)
// Not ported: Iterate_Complete_Dispose - needs operator Create of an iterator (`ObservableEx.Create`, built on Concat) (phase 3, later)
// Not ported: IteratorScenario - needs operator Create of an iterator (`ObservableEx.Create`, built on Concat) and ToListObservable (phase 3, later)
// Not ported: Iterate_Void_ArgumentChecking - argument validation
// Not ported: Iterate_Void_Complete - needs operator Create of an iterator (`ObservableEx.Create`, built on Concat) (phase 3, later)
// Not ported: Iterate_Void_Complete_Implicit - needs operator Create of an iterator (`ObservableEx.Create`, built on Concat) (phase 3, later)
// Not ported: Iterate_Void_Iterator_Throw - needs operator Create of an iterator (`ObservableEx.Create`, built on Concat) (phase 3, later)
// Not ported: Iterate_Void_Complete_Dispose - needs operator Create of an iterator (`ObservableEx.Create`, built on Concat) (phase 3, later)
// Not ported: Iterate_Void_Func_Throw - needs operator Create of an iterator (`ObservableEx.Create`, built on Concat) (phase 3, later)
// Not ported: IteratorScenario_Void - needs operator Create of an iterator (`ObservableEx.Create`, built on Concat) and ToListObservable (phase 3, later)
