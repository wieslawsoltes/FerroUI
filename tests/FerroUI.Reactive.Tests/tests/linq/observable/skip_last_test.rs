//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/SkipLastTest.cs` of Rx.NET.
//! The operators are applied to the handle of the test observable (`as_observable`); `SkipLast(TimeSpan, IScheduler)` is `skip_last_time_on`.

use ferroui_reactive::ObservableExt;
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ticks_to_duration, ReactiveTest as RT, TestScheduler};
use std::time::Duration;

#[test]
fn skip_last_zero_completed() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(180, 1),
        RT::on_next(210, 2),
        RT::on_next(250, 3),
        RT::on_next(270, 4),
        RT::on_next(310, 5),
        RT::on_next(360, 6),
        RT::on_next(380, 7),
        RT::on_next(410, 8),
        RT::on_next(590, 9),
        RT::on_completed::<i32>(650),
    ]);

    let res = {
        let xs = xs.clone();
        scheduler.start(move || xs.as_observable().skip_last(0))
    };

    assert_messages!(res.messages(),
        RT::on_next(210, 2),
        RT::on_next(250, 3),
        RT::on_next(270, 4),
        RT::on_next(310, 5),
        RT::on_next(360, 6),
        RT::on_next(380, 7),
        RT::on_next(410, 8),
        RT::on_next(590, 9),
        RT::on_completed::<i32>(650),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 650),
    );
}

#[test]
fn skip_last_zero_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(180, 1),
        RT::on_next(210, 2),
        RT::on_next(250, 3),
        RT::on_next(270, 4),
        RT::on_next(310, 5),
        RT::on_next(360, 6),
        RT::on_next(380, 7),
        RT::on_next(410, 8),
        RT::on_next(590, 9),
        RT::on_error::<i32>(650, ex.clone()),
    ]);

    let res = {
        let xs = xs.clone();
        scheduler.start(move || xs.as_observable().skip_last(0))
    };

    assert_messages!(res.messages(),
        RT::on_next(210, 2),
        RT::on_next(250, 3),
        RT::on_next(270, 4),
        RT::on_next(310, 5),
        RT::on_next(360, 6),
        RT::on_next(380, 7),
        RT::on_next(410, 8),
        RT::on_next(590, 9),
        RT::on_error::<i32>(650, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 650),
    );
}

#[test]
fn skip_last_zero_disposed() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(180, 1),
        RT::on_next(210, 2),
        RT::on_next(250, 3),
        RT::on_next(270, 4),
        RT::on_next(310, 5),
        RT::on_next(360, 6),
        RT::on_next(380, 7),
        RT::on_next(410, 8),
        RT::on_next(590, 9),
    ]);

    let res = {
        let xs = xs.clone();
        scheduler.start(move || xs.as_observable().skip_last(0))
    };

    assert_messages!(res.messages(),
        RT::on_next(210, 2),
        RT::on_next(250, 3),
        RT::on_next(270, 4),
        RT::on_next(310, 5),
        RT::on_next(360, 6),
        RT::on_next(380, 7),
        RT::on_next(410, 8),
        RT::on_next(590, 9),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 1000),
    );
}

#[test]
fn skip_last_one_completed() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(180, 1),
        RT::on_next(210, 2),
        RT::on_next(250, 3),
        RT::on_next(270, 4),
        RT::on_next(310, 5),
        RT::on_next(360, 6),
        RT::on_next(380, 7),
        RT::on_next(410, 8),
        RT::on_next(590, 9),
        RT::on_completed::<i32>(650),
    ]);

    let res = {
        let xs = xs.clone();
        scheduler.start(move || xs.as_observable().skip_last(1))
    };

    assert_messages!(res.messages(),
        RT::on_next(250, 2),
        RT::on_next(270, 3),
        RT::on_next(310, 4),
        RT::on_next(360, 5),
        RT::on_next(380, 6),
        RT::on_next(410, 7),
        RT::on_next(590, 8),
        RT::on_completed::<i32>(650),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 650),
    );
}

#[test]
fn skip_last_one_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(180, 1),
        RT::on_next(210, 2),
        RT::on_next(250, 3),
        RT::on_next(270, 4),
        RT::on_next(310, 5),
        RT::on_next(360, 6),
        RT::on_next(380, 7),
        RT::on_next(410, 8),
        RT::on_next(590, 9),
        RT::on_error::<i32>(650, ex.clone()),
    ]);

    let res = {
        let xs = xs.clone();
        scheduler.start(move || xs.as_observable().skip_last(1))
    };

    assert_messages!(res.messages(),
        RT::on_next(250, 2),
        RT::on_next(270, 3),
        RT::on_next(310, 4),
        RT::on_next(360, 5),
        RT::on_next(380, 6),
        RT::on_next(410, 7),
        RT::on_next(590, 8),
        RT::on_error::<i32>(650, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 650),
    );
}

#[test]
fn skip_last_one_disposed() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(180, 1),
        RT::on_next(210, 2),
        RT::on_next(250, 3),
        RT::on_next(270, 4),
        RT::on_next(310, 5),
        RT::on_next(360, 6),
        RT::on_next(380, 7),
        RT::on_next(410, 8),
        RT::on_next(590, 9),
    ]);

    let res = {
        let xs = xs.clone();
        scheduler.start(move || xs.as_observable().skip_last(1))
    };

    assert_messages!(res.messages(),
        RT::on_next(250, 2),
        RT::on_next(270, 3),
        RT::on_next(310, 4),
        RT::on_next(360, 5),
        RT::on_next(380, 6),
        RT::on_next(410, 7),
        RT::on_next(590, 8),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 1000),
    );
}

#[test]
fn skip_last_three_completed() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(180, 1),
        RT::on_next(210, 2),
        RT::on_next(250, 3),
        RT::on_next(270, 4),
        RT::on_next(310, 5),
        RT::on_next(360, 6),
        RT::on_next(380, 7),
        RT::on_next(410, 8),
        RT::on_next(590, 9),
        RT::on_completed::<i32>(650),
    ]);

    let res = {
        let xs = xs.clone();
        scheduler.start(move || xs.as_observable().skip_last(3))
    };

    assert_messages!(res.messages(),
        RT::on_next(310, 2),
        RT::on_next(360, 3),
        RT::on_next(380, 4),
        RT::on_next(410, 5),
        RT::on_next(590, 6),
        RT::on_completed::<i32>(650),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 650),
    );
}

#[test]
fn skip_last_three_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(180, 1),
        RT::on_next(210, 2),
        RT::on_next(250, 3),
        RT::on_next(270, 4),
        RT::on_next(310, 5),
        RT::on_next(360, 6),
        RT::on_next(380, 7),
        RT::on_next(410, 8),
        RT::on_next(590, 9),
        RT::on_error::<i32>(650, ex.clone()),
    ]);

    let res = {
        let xs = xs.clone();
        scheduler.start(move || xs.as_observable().skip_last(3))
    };

    assert_messages!(res.messages(),
        RT::on_next(310, 2),
        RT::on_next(360, 3),
        RT::on_next(380, 4),
        RT::on_next(410, 5),
        RT::on_next(590, 6),
        RT::on_error::<i32>(650, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 650),
    );
}

#[test]
fn skip_last_three_disposed() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(180, 1),
        RT::on_next(210, 2),
        RT::on_next(250, 3),
        RT::on_next(270, 4),
        RT::on_next(310, 5),
        RT::on_next(360, 6),
        RT::on_next(380, 7),
        RT::on_next(410, 8),
        RT::on_next(590, 9),
    ]);

    let res = {
        let xs = xs.clone();
        scheduler.start(move || xs.as_observable().skip_last(3))
    };

    assert_messages!(res.messages(),
        RT::on_next(310, 2),
        RT::on_next(360, 3),
        RT::on_next(380, 4),
        RT::on_next(410, 5),
        RT::on_next(590, 6),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 1000),
    );
}

#[test]
fn skip_last_zero1() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, 1),
        RT::on_next(220, 2),
        RT::on_completed::<i32>(230),
    ]);

    let res = {
        let xs = xs.clone();
        let s = scheduler.scheduler();
        scheduler.start(move || xs.as_observable().skip_last_time_on(Duration::ZERO, s.clone()))
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
fn skip_last_zero2() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, 1),
        RT::on_next(220, 2),
        RT::on_next(230, 3),
        RT::on_completed::<i32>(230),
    ]);

    let res = {
        let xs = xs.clone();
        let s = scheduler.scheduler();
        scheduler.start(move || xs.as_observable().skip_last_time_on(Duration::ZERO, s.clone()))
    };

    assert_messages!(res.messages(),
        RT::on_next(210, 1),
        RT::on_next(220, 2),
        RT::on_next(230, 3),
        RT::on_completed::<i32>(230),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 230),
    );
}

#[test]
fn skip_last_some1() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, 1),
        RT::on_next(220, 2),
        RT::on_completed::<i32>(230),
    ]);

    let res = {
        let xs = xs.clone();
        let s = scheduler.scheduler();
        scheduler.start(move || xs.as_observable().skip_last_time_on(ticks_to_duration(15), s.clone()))
    };

    assert_messages!(res.messages(),
        RT::on_next(230, 1),
        RT::on_completed::<i32>(230),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 230),
    );
}

#[test]
fn skip_last_some2() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, 1),
        RT::on_next(220, 2),
        RT::on_next(230, 3),
        RT::on_next(240, 4),
        RT::on_next(250, 5),
        RT::on_next(260, 6),
        RT::on_next(270, 7),
        RT::on_next(280, 8),
        RT::on_next(290, 9),
        RT::on_completed::<i32>(300),
    ]);

    let res = {
        let xs = xs.clone();
        let s = scheduler.scheduler();
        scheduler.start(move || xs.as_observable().skip_last_time_on(ticks_to_duration(45), s.clone()))
    };

    assert_messages!(res.messages(),
        RT::on_next(260, 1),
        RT::on_next(270, 2),
        RT::on_next(280, 3),
        RT::on_next(290, 4),
        RT::on_next(300, 5),
        RT::on_completed::<i32>(300),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 300),
    );
}

#[test]
fn skip_last_all() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, 1),
        RT::on_next(220, 2),
        RT::on_completed::<i32>(230),
    ]);

    let res = {
        let xs = xs.clone();
        let s = scheduler.scheduler();
        scheduler.start(move || xs.as_observable().skip_last_time_on(ticks_to_duration(50), s.clone()))
    };

    assert_messages!(res.messages(),
        RT::on_completed::<i32>(230),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 230),
    );
}

#[test]
fn skip_last_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_error::<i32>(210, ex.clone()),
    ]);

    let res = {
        let xs = xs.clone();
        let s = scheduler.scheduler();
        scheduler.start(move || xs.as_observable().skip_last_time_on(ticks_to_duration(50), s.clone()))
    };

    assert_messages!(res.messages(),
        RT::on_error::<i32>(210, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 210),
    );
}

#[test]
fn skip_last_never() {
    let scheduler = TestScheduler::new();

    let _ex = new_error("ex");

    let xs = scheduler.create_hot_observable::<i32>(vec![]);

    let res = {
        let xs = xs.clone();
        let s = scheduler.scheduler();
        scheduler.start(move || xs.as_observable().skip_last_time_on(ticks_to_duration(50), s.clone()))
    };

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 1000),
    );
}

// Not ported: SkipLast_ArgumentChecking - argument validation
// Not ported: SkipLast_Timed_ArgumentChecking - argument validation
// Not ported: SkipLast_Default1 - real threads
// Not ported: SkipLast_Default2 - real threads
// Not ported: SkipLast_Default3 - real threads
// Not ported: SkipLast_Default4 - real threads
