//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/ReturnTest.cs` of Rx.NET.
//! Adapted: an observer that throws is one that panics, and the scheduler that runs it panics,
//! checked with `assert_throws`.

use ferroui_reactive::disposables::SerialDisposable;
use ferroui_reactive::{IDisposable, IObserver, Observable, ObservableExt};
use ferroui_reactive_testing::{assert_messages, assert_throws, ReactiveTest as RT, TestScheduler};

#[test]
fn return_basic() {
    let scheduler = TestScheduler::new();

    let s = scheduler.scheduler();
    let res = scheduler.start(move || Observable::return_on(42, s));

    assert_messages!(res.messages(), RT::on_next(201, 42), RT::on_completed::<i32>(201));
}

#[test]
fn return_disposed() {
    let scheduler = TestScheduler::new();

    let s = scheduler.scheduler();
    let res = scheduler.start_disposed(move || Observable::return_on(42, s), 200);

    assert_messages!(res.messages());
}

#[test]
fn return_disposed_after_next() {
    let scheduler = TestScheduler::new();

    let d = SerialDisposable::new();

    let xs = Observable::return_on(42, scheduler.scheduler());

    let res = scheduler.create_observer::<i32>();

    {
        let (d, res) = (d.clone(), res.clone());
        scheduler.schedule_absolute_action(100, move || {
            let (d1, r1, r2, r3) = (d.clone(), res.clone(), res.clone(), res.clone());
            let subscription = xs.subscribe_all(
                move |x| {
                    d1.dispose();
                    r1.on_next(x);
                },
                move |e| r2.on_error(e),
                move || r3.on_completed(),
            );
            d.set_disposable(Some(subscription));
        });
    }

    scheduler.run();

    assert_messages!(res.messages(), RT::on_next(101, 42));
}

#[test]
fn return_observer_throws() {
    let scheduler1 = TestScheduler::new();

    let xs = Observable::return_on(1, scheduler1.scheduler());

    xs.subscribe_next(|_| panic!("InvalidOperationException"));

    assert_throws(|| scheduler1.run());

    let scheduler2 = TestScheduler::new();

    let ys = Observable::return_on(1, scheduler2.scheduler());

    ys.subscribe_all(|_| {}, |_| {}, || panic!("InvalidOperationException"));

    assert_throws(|| scheduler2.run());
}

// Not ported: Return_ArgumentChecking - argument validation
// Not ported: Return_DefaultScheduler - real threads (compares with the sequence on DefaultScheduler.Instance, through the blocking ToEnumerable of AssertEqual)
