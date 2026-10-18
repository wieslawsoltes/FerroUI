//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/TakeLastTest.cs` of Rx.NET.
//! The operators are applied to the handle of the test observable (`as_observable`); `TakeLast(TimeSpan, IScheduler)` is `take_last_time_on` and `TakeLast(TimeSpan, IScheduler, IScheduler)` is `take_last_time_on_with_loop`.

use ferroui_reactive::ObservableExt;
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ticks_to_duration, ReactiveTest as RT, TestScheduler};
use std::time::Duration;

#[test]
fn take_last_zero_completed() {
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
        scheduler.start(move || xs.as_observable().take_last(0))
    };

    assert_messages!(res.messages(),
        RT::on_completed::<i32>(650),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 650),
    );
}

#[test]
fn take_last_zero_error() {
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
        scheduler.start(move || xs.as_observable().take_last(0))
    };

    assert_messages!(res.messages(),
        RT::on_error::<i32>(650, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 650),
    );
}

#[test]
fn take_last_zero_disposed() {
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
        scheduler.start(move || xs.as_observable().take_last(0))
    };

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 1000),
    );
}

#[test]
fn take_last_one_completed() {
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
        scheduler.start(move || xs.as_observable().take_last(1))
    };

    assert_messages!(res.messages(),
        RT::on_next(650, 9),
        RT::on_completed::<i32>(650),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 650),
    );
}

#[test]
fn take_last_one_error() {
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
        scheduler.start(move || xs.as_observable().take_last(1))
    };

    assert_messages!(res.messages(),
        RT::on_error::<i32>(650, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 650),
    );
}

#[test]
fn take_last_one_disposed() {
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
        scheduler.start(move || xs.as_observable().take_last(1))
    };

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 1000),
    );
}

#[test]
fn take_last_three_completed() {
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
        scheduler.start(move || xs.as_observable().take_last(3))
    };

    assert_messages!(res.messages(),
        RT::on_next(650, 7),
        RT::on_next(650, 8),
        RT::on_next(650, 9),
        RT::on_completed::<i32>(650),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 650),
    );
}

#[test]
fn take_last_three_error() {
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
        scheduler.start(move || xs.as_observable().take_last(3))
    };

    assert_messages!(res.messages(),
        RT::on_error::<i32>(650, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 650),
    );
}

#[test]
fn take_last_three_disposed() {
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
        scheduler.start(move || xs.as_observable().take_last(3))
    };

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 1000),
    );
}

#[test]
fn take_last_zero1() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, 1),
        RT::on_next(220, 2),
        RT::on_completed::<i32>(230),
    ]);

    let res = {
        let xs = xs.clone();
        let s = scheduler.scheduler();
        scheduler.start(move || xs.as_observable().take_last_time_on(Duration::ZERO, s.clone()))
    };

    assert_messages!(res.messages(),
        RT::on_completed::<i32>(230),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 230),
    );
}

#[test]
fn take_last_zero1_with_loop_scheduler() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, 1),
        RT::on_next(220, 2),
        RT::on_completed::<i32>(230),
    ]);

    let res = {
        let xs = xs.clone();
        let s = scheduler.scheduler();
        scheduler.start(move || xs.as_observable().take_last_time_on_with_loop(Duration::ZERO, s.clone(), s.clone()))
    };

    assert_messages!(res.messages(),
        RT::on_completed::<i32>(231),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 230),
    );
}

#[test]
fn take_last_zero2() {
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
        scheduler.start(move || xs.as_observable().take_last_time_on(Duration::ZERO, s.clone()))
    };

    assert_messages!(res.messages(),
        RT::on_completed::<i32>(230),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 230),
    );
}

#[test]
fn take_last_zero2_with_loop_scheduler() {
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
        scheduler.start(move || xs.as_observable().take_last_time_on_with_loop(Duration::ZERO, s.clone(), s.clone()))
    };

    assert_messages!(res.messages(),
        RT::on_completed::<i32>(231),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 230),
    );
}

#[test]
fn take_last_some1() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, 1),
        RT::on_next(220, 2),
        RT::on_next(230, 3),
        RT::on_completed::<i32>(240),
    ]);

    let res = {
        let xs = xs.clone();
        let s = scheduler.scheduler();
        scheduler.start(move || xs.as_observable().take_last_time_on(ticks_to_duration(25), s.clone()))
    };

    assert_messages!(res.messages(),
        RT::on_next(240, 2),
        RT::on_next(240, 3),
        RT::on_completed::<i32>(240),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 240),
    );
}

#[test]
fn take_last_some1_with_loop_scheduler() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, 1),
        RT::on_next(220, 2),
        RT::on_next(230, 3),
        RT::on_completed::<i32>(240),
    ]);

    let res = {
        let xs = xs.clone();
        let s = scheduler.scheduler();
        scheduler.start(move || xs.as_observable().take_last_time_on_with_loop(ticks_to_duration(25), s.clone(), s.clone()))
    };

    assert_messages!(res.messages(),
        RT::on_next(241, 2),
        RT::on_next(242, 3),
        RT::on_completed::<i32>(243),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 240),
    );
}

#[test]
fn take_last_some2() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, 1),
        RT::on_next(220, 2),
        RT::on_next(230, 3),
        RT::on_completed::<i32>(300),
    ]);

    let res = {
        let xs = xs.clone();
        let s = scheduler.scheduler();
        scheduler.start(move || xs.as_observable().take_last_time_on(ticks_to_duration(25), s.clone()))
    };

    assert_messages!(res.messages(),
        RT::on_completed::<i32>(300),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 300),
    );
}

#[test]
fn take_last_some2_with_loop_scheduler() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, 1),
        RT::on_next(220, 2),
        RT::on_next(230, 3),
        RT::on_completed::<i32>(300),
    ]);

    let res = {
        let xs = xs.clone();
        let s = scheduler.scheduler();
        scheduler.start(move || xs.as_observable().take_last_time_on_with_loop(ticks_to_duration(25), s.clone(), s.clone()))
    };

    assert_messages!(res.messages(),
        RT::on_completed::<i32>(301),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 300),
    );
}

#[test]
fn take_last_some3() {
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
        scheduler.start(move || xs.as_observable().take_last_time_on(ticks_to_duration(45), s.clone()))
    };

    assert_messages!(res.messages(),
        RT::on_next(300, 6),
        RT::on_next(300, 7),
        RT::on_next(300, 8),
        RT::on_next(300, 9),
        RT::on_completed::<i32>(300),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 300),
    );
}

#[test]
fn take_last_some3_with_loop_scheduler() {
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
        scheduler.start(move || xs.as_observable().take_last_time_on_with_loop(ticks_to_duration(45), s.clone(), s.clone()))
    };

    assert_messages!(res.messages(),
        RT::on_next(301, 6),
        RT::on_next(302, 7),
        RT::on_next(303, 8),
        RT::on_next(304, 9),
        RT::on_completed::<i32>(305),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 300),
    );
}

#[test]
fn take_last_some4() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, 1),
        RT::on_next(240, 2),
        RT::on_next(250, 3),
        RT::on_next(280, 4),
        RT::on_next(290, 5),
        RT::on_next(300, 6),
        RT::on_completed::<i32>(350),
    ]);

    let res = {
        let xs = xs.clone();
        let s = scheduler.scheduler();
        scheduler.start(move || xs.as_observable().take_last_time_on(ticks_to_duration(25), s.clone()))
    };

    assert_messages!(res.messages(),
        RT::on_completed::<i32>(350),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 350),
    );
}

#[test]
fn take_last_some4_with_loop_scheduler() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, 1),
        RT::on_next(240, 2),
        RT::on_next(250, 3),
        RT::on_next(280, 4),
        RT::on_next(290, 5),
        RT::on_next(300, 6),
        RT::on_completed::<i32>(350),
    ]);

    let res = {
        let xs = xs.clone();
        let s = scheduler.scheduler();
        scheduler.start(move || xs.as_observable().take_last_time_on_with_loop(ticks_to_duration(25), s.clone(), s.clone()))
    };

    assert_messages!(res.messages(),
        RT::on_completed::<i32>(351),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 350),
    );
}

#[test]
fn take_last_all() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, 1),
        RT::on_next(220, 2),
        RT::on_completed::<i32>(230),
    ]);

    let res = {
        let xs = xs.clone();
        let s = scheduler.scheduler();
        scheduler.start(move || xs.as_observable().take_last_time_on(ticks_to_duration(50), s.clone()))
    };

    assert_messages!(res.messages(),
        RT::on_next(230, 1),
        RT::on_next(230, 2),
        RT::on_completed::<i32>(230),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 230),
    );
}

#[test]
fn take_last_all_with_loop_scheduler() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, 1),
        RT::on_next(220, 2),
        RT::on_completed::<i32>(230),
    ]);

    let res = {
        let xs = xs.clone();
        let s = scheduler.scheduler();
        scheduler.start(move || xs.as_observable().take_last_time_on_with_loop(ticks_to_duration(50), s.clone(), s.clone()))
    };

    assert_messages!(res.messages(),
        RT::on_next(231, 1),
        RT::on_next(232, 2),
        RT::on_completed::<i32>(233),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 230),
    );
}

#[test]
fn take_last_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_error::<i32>(210, ex.clone()),
    ]);

    let res = {
        let xs = xs.clone();
        let s = scheduler.scheduler();
        scheduler.start(move || xs.as_observable().take_last_time_on(ticks_to_duration(50), s.clone()))
    };

    assert_messages!(res.messages(),
        RT::on_error::<i32>(210, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 210),
    );
}

#[test]
fn take_last_error_with_loop_scheduler() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_error::<i32>(210, ex.clone()),
    ]);

    let res = {
        let xs = xs.clone();
        let s = scheduler.scheduler();
        scheduler.start(move || xs.as_observable().take_last_time_on_with_loop(ticks_to_duration(50), s.clone(), s.clone()))
    };

    assert_messages!(res.messages(),
        RT::on_error::<i32>(210, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 210),
    );
}

#[test]
fn take_last_never() {
    let scheduler = TestScheduler::new();

    let _ex = new_error("ex");

    let xs = scheduler.create_hot_observable::<i32>(vec![]);

    let res = {
        let xs = xs.clone();
        let s = scheduler.scheduler();
        scheduler.start(move || xs.as_observable().take_last_time_on(ticks_to_duration(50), s.clone()))
    };

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 1000),
    );
}

#[test]
fn take_last_never_with_loop_scheduler() {
    let scheduler = TestScheduler::new();

    let _ex = new_error("ex");

    let xs = scheduler.create_hot_observable::<i32>(vec![]);

    let res = {
        let xs = xs.clone();
        let s = scheduler.scheduler();
        scheduler.start(move || xs.as_observable().take_last_time_on_with_loop(ticks_to_duration(50), s.clone(), s.clone()))
    };

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 1000),
    );
}

// Not ported: TakeLast_ArgumentChecking - argument validation
// Not ported: TakeLast_LongRunning_Regular - real threads (NewThreadScheduler, a long-running scheduler)
// Not ported: TakeLast_Timed_ArgumentChecking - argument validation
// Not ported: TakeLast_Default1 - real threads
// Not ported: TakeLast_Default2 - real threads
// Not ported: TakeLast_Default3 - real threads
// Not ported: TakeLast_Default4 - real threads
// Not ported: TakeLast_Timed_LongRunning_Regular - real threads (NewThreadScheduler, a long-running scheduler)
