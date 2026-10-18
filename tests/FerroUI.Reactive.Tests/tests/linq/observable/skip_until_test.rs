//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/SkipUntilTest.cs` of Rx.NET.
//! The operators are applied to the handle of the test observable (`as_observable`); `SkipUntil(DateTimeOffset, IScheduler)` is `skip_until_at_on`, with `new DateTimeOffset(t, TimeSpan.Zero)` as `ticks_to_time(t)`.

use ferroui_reactive::{IObserver, Observable, ObservableExt};
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ticks_to_time, ReactiveTest as RT, TestScheduler};
use std::cell::Cell;
use std::rc::Rc;

#[test]
fn skip_until_some_data_next() {
    let scheduler = TestScheduler::new();

    let l = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_next(230, 4),
        RT::on_next(240, 5),
        RT::on_completed::<i32>(250),
    ]);

    let r = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(225, 99),
        RT::on_completed::<i32>(230),
    ]);

    let res = {
        let l = l.clone();
        let r = r.clone();
        scheduler.start(move || l.as_observable().skip_until(r.as_observable()))
    };

    assert_messages!(res.messages(),
        RT::on_next(230, 4),
        RT::on_next(240, 5),
        RT::on_completed::<i32>(250),
    );

    assert_subscriptions!(l.subscriptions(),
        RT::subscribe(200, 250),
    );

    assert_subscriptions!(r.subscriptions(),
        RT::subscribe(200, 225),
    );
}

#[test]
fn skip_until_some_data_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let l = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_next(230, 4),
        RT::on_next(240, 5),
        RT::on_completed::<i32>(250),
    ]);

    let r = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_error::<i32>(225, ex.clone()),
    ]);

    let res = {
        let l = l.clone();
        let r = r.clone();
        scheduler.start(move || l.as_observable().skip_until(r.as_observable()))
    };

    assert_messages!(res.messages(),
        RT::on_error::<i32>(225, ex.clone()),
    );

    assert_subscriptions!(l.subscriptions(),
        RT::subscribe(200, 225),
    );

    assert_subscriptions!(r.subscriptions(),
        RT::subscribe(200, 225),
    );
}

#[test]
fn skip_until_error_some_data() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let l = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_error::<i32>(220, ex.clone()),
    ]);

    let r = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(230, 2),
        RT::on_completed::<i32>(250),
    ]);

    let res = {
        let l = l.clone();
        let r = r.clone();
        scheduler.start(move || l.as_observable().skip_until(r.as_observable()))
    };

    assert_messages!(res.messages(),
        RT::on_error::<i32>(220, ex.clone()),
    );

    assert_subscriptions!(l.subscriptions(),
        RT::subscribe(200, 220),
    );

    assert_subscriptions!(r.subscriptions(),
        RT::subscribe(200, 220),
    );
}

#[test]
fn skip_until_some_data_empty() {
    let scheduler = TestScheduler::new();

    let l = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_next(230, 4),
        RT::on_next(240, 5),
        RT::on_completed::<i32>(250),
    ]);

    let r = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_completed::<i32>(225),
    ]);

    let res = {
        let l = l.clone();
        let r = r.clone();
        scheduler.start(move || l.as_observable().skip_until(r.as_observable()))
    };

    assert_messages!(res.messages());

    assert_subscriptions!(l.subscriptions(),
        RT::subscribe(200, 250),
    );

    assert_subscriptions!(r.subscriptions(),
        RT::subscribe(200, 225),
    );
}

#[test]
fn skip_until_never_next() {
    let scheduler = TestScheduler::new();

    let l = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
    ]);

    let r = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(225, 2),
        RT::on_completed::<i32>(250),
    ]);

    let res = {
        let l = l.clone();
        let r = r.clone();
        scheduler.start(move || l.as_observable().skip_until(r.as_observable()))
    };

    assert_messages!(res.messages());

    assert_subscriptions!(l.subscriptions(),
        RT::subscribe(200, 1000),
    );

    assert_subscriptions!(r.subscriptions(),
        RT::subscribe(200, 225),
    );
}

#[test]
fn skip_until_never_error1() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let l = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
    ]);

    let r = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_error::<i32>(225, ex.clone()),
    ]);

    let res = {
        let l = l.clone();
        let r = r.clone();
        scheduler.start(move || l.as_observable().skip_until(r.as_observable()))
    };

    assert_messages!(res.messages(),
        RT::on_error::<i32>(225, ex.clone()),
    );

    assert_subscriptions!(l.subscriptions(),
        RT::subscribe(200, 225),
    );

    assert_subscriptions!(r.subscriptions(),
        RT::subscribe(200, 225),
    );
}

#[test]
fn skip_until_some_data_error2() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let l = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_next(230, 4),
        RT::on_next(240, 5),
        RT::on_completed::<i32>(250),
    ]);

    let r = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_error::<i32>(300, ex.clone()),
    ]);

    let res = {
        let l = l.clone();
        let r = r.clone();
        scheduler.start(move || l.as_observable().skip_until(r.as_observable()))
    };

    assert_messages!(res.messages(),
        RT::on_error::<i32>(300, ex.clone()),
    );

    assert_subscriptions!(l.subscriptions(),
        RT::subscribe(200, 250),
    );

    assert_subscriptions!(r.subscriptions(),
        RT::subscribe(200, 300),
    );
}

#[test]
fn skip_until_some_data_never() {
    let scheduler = TestScheduler::new();

    let l = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_next(230, 4),
        RT::on_next(240, 5),
        RT::on_completed::<i32>(250),
    ]);

    let r = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
    ]);

    let res = {
        let l = l.clone();
        let r = r.clone();
        scheduler.start(move || l.as_observable().skip_until(r.as_observable()))
    };

    assert_messages!(res.messages());

    assert_subscriptions!(l.subscriptions(),
        RT::subscribe(200, 250),
    );

    assert_subscriptions!(r.subscriptions(),
        // The other sequence cannot be disposed early: its error could be on its way.
        RT::subscribe(200, 1000),
    );
}

#[test]
fn skip_until_never_empty() {
    let scheduler = TestScheduler::new();

    let l = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
    ]);

    let r = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_completed::<i32>(225),
    ]);

    let res = {
        let l = l.clone();
        let r = r.clone();
        scheduler.start(move || l.as_observable().skip_until(r.as_observable()))
    };

    assert_messages!(res.messages());

    assert_subscriptions!(l.subscriptions(),
        RT::subscribe(200, 1000),
    );

    assert_subscriptions!(r.subscriptions(),
        RT::subscribe(200, 225),
    );
}

#[test]
fn skip_until_never_never() {
    let scheduler = TestScheduler::new();

    let l = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
    ]);

    let r = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
    ]);

    let res = {
        let l = l.clone();
        let r = r.clone();
        scheduler.start(move || l.as_observable().skip_until(r.as_observable()))
    };

    assert_messages!(res.messages());

    assert_subscriptions!(l.subscriptions(),
        RT::subscribe(200, 1000),
    );

    assert_subscriptions!(r.subscriptions(),
        RT::subscribe(200, 1000),
    );
}

#[test]
fn skip_until_has_completed_causes_disposal() {
    let scheduler = TestScheduler::new();

    let disposed = Rc::new(Cell::new(false));

    let l = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_next(230, 4),
        RT::on_next(240, 5),
        RT::on_completed::<i32>(250),
    ]);

    let r = {
        let disposed = disposed.clone();
        Observable::create_with_action(move |_obs: Rc<dyn IObserver<i32>>| {
            let disposed = disposed.clone();
            move || disposed.set(true)
        })
    };

    let res = {
        let l = l.clone();
        scheduler.start(move || l.as_observable().skip_until(r))
    };

    assert_messages!(res.messages());

    assert!(disposed.get(), "disposed");
}

#[test]
fn skip_until_immediate() {
    let scheduler = TestScheduler::new();

    let xs = Observable::return_(1);
    let ys = Observable::return_("bar");

    let res = {
        scheduler.start(move || xs.skip_until(ys))
    };

    assert_messages!(res.messages(),
        RT::on_next(200, 1),
        RT::on_completed::<i32>(200),
    );
}

#[test]
fn skip_until_empty_empty() {
    let scheduler = TestScheduler::new();

    let l = scheduler.create_hot_observable(vec![
        RT::on_completed::<i32>(250),
    ]);

    let r = scheduler.create_hot_observable(vec![
        RT::on_completed::<i32>(250),
    ]);

    let res = {
        let l = l.clone();
        let r = r.clone();
        scheduler.start(move || l.as_observable().skip_until(r.as_observable()))
    };

    // Upstream asserts this behaviour although it considers it buggy (a fix would be breaking); the
    // desired behaviour is a completion at 250.
    assert_messages!(res.messages());
}

#[test]
fn skip_until_zero() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, 1),
        RT::on_next(220, 2),
        RT::on_completed::<i32>(230),
    ]);

    let res = {
        let xs = xs.clone();
        let s = scheduler.scheduler();
        scheduler.start(move || xs.as_observable().skip_until_at_on(ticks_to_time(0), s.clone()))
    };

    assert_messages!(res.messages(),
        RT::on_next(210, 1),
        RT::on_next(220, 2),
        RT::on_completed::<i32>(230),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 230),
    );
}

#[test]
fn skip_until_some() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, 1),
        RT::on_next(220, 2),
        RT::on_completed::<i32>(230),
    ]);

    let res = {
        let xs = xs.clone();
        let s = scheduler.scheduler();
        scheduler.start(move || xs.as_observable().skip_until_at_on(ticks_to_time(215), s.clone()))
    };

    assert_messages!(res.messages(),
        RT::on_next(220, 2),
        RT::on_completed::<i32>(230),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 230),
    );
}

#[test]
fn skip_until_late() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, 1),
        RT::on_next(220, 2),
        RT::on_completed::<i32>(230),
    ]);

    let res = {
        let xs = xs.clone();
        let s = scheduler.scheduler();
        scheduler.start(move || xs.as_observable().skip_until_at_on(ticks_to_time(250), s.clone()))
    };

    assert_messages!(res.messages(),
        RT::on_completed::<i32>(230),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 230),
    );
}

#[test]
fn skip_until_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_error::<i32>(210, ex.clone()),
    ]);

    let res = {
        let xs = xs.clone();
        let s = scheduler.scheduler();
        scheduler.start(move || xs.as_observable().skip_until_at_on(ticks_to_time(250), s.clone()))
    };

    assert_messages!(res.messages(),
        RT::on_error::<i32>(210, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 210),
    );
}

#[test]
fn skip_until_never() {
    let scheduler = TestScheduler::new();

    let _ex = new_error("ex");

    let xs = scheduler.create_hot_observable::<i32>(vec![]);

    let res = {
        let xs = xs.clone();
        let s = scheduler.scheduler();
        scheduler.start(move || xs.as_observable().skip_until_at_on(ticks_to_time(250), s.clone()))
    };

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 1000),
    );
}

#[test]
fn skip_until_twice1() {
    let scheduler = TestScheduler::new();

    let _ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, 1),
        RT::on_next(220, 2),
        RT::on_next(230, 3),
        RT::on_next(240, 4),
        RT::on_next(250, 5),
        RT::on_next(260, 6),
        RT::on_completed::<i32>(270),
    ]);

    let res = {
        let xs = xs.clone();
        let s = scheduler.scheduler();
        scheduler.start(move || xs.as_observable().skip_until_at_on(ticks_to_time(215), s.clone()).skip_until_at_on(ticks_to_time(230), s.clone()))
    };

    assert_messages!(res.messages(),
        RT::on_next(240, 4),
        RT::on_next(250, 5),
        RT::on_next(260, 6),
        RT::on_completed::<i32>(270),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 270),
    );
}

#[test]
fn skip_until_twice2() {
    let scheduler = TestScheduler::new();

    let _ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, 1),
        RT::on_next(220, 2),
        RT::on_next(230, 3),
        RT::on_next(240, 4),
        RT::on_next(250, 5),
        RT::on_next(260, 6),
        RT::on_completed::<i32>(270),
    ]);

    let res = {
        let xs = xs.clone();
        let s = scheduler.scheduler();
        scheduler.start(move || xs.as_observable().skip_until_at_on(ticks_to_time(230), s.clone()).skip_until_at_on(ticks_to_time(215), s.clone()))
    };

    assert_messages!(res.messages(),
        RT::on_next(240, 4),
        RT::on_next(250, 5),
        RT::on_next(260, 6),
        RT::on_completed::<i32>(270),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 270),
    );
}

// Not ported: SkipUntil_ArgumentChecking - argument validation
// Not ported: SkipUntil_Timed_ArgumentChecking - argument validation
// Not ported: SkipUntil_Default - real threads
