//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/TakeLastBufferTest.cs` of Rx.NET.
//! The operators are applied to the handle of the test observable (`as_observable`); `TakeLastBuffer(TimeSpan, IScheduler)` is `take_last_buffer_time_on`, and the lists are `Vec`s.

use ferroui_reactive::ObservableExt;
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ticks_to_duration, ReactiveTest as RT, TestScheduler};
use std::time::Duration;

#[test]
fn take_last_buffer_zero_completed() {
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
        scheduler.start(move || xs.as_observable().take_last_buffer(0))
    };

    assert_messages!(res.messages(),
        RT::on_next_where(650, |lst: &Vec<i32>| lst.is_empty()),
        RT::on_completed::<Vec<i32>>(650),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 650),
    );
}

#[test]
fn take_last_buffer_zero_error() {
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
        scheduler.start(move || xs.as_observable().take_last_buffer(0))
    };

    assert_messages!(res.messages(),
        RT::on_error::<Vec<i32>>(650, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 650),
    );
}

#[test]
fn take_last_buffer_zero_disposed() {
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
        scheduler.start(move || xs.as_observable().take_last_buffer(0))
    };

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 1000),
    );
}

#[test]
fn take_last_buffer_one_completed() {
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
        scheduler.start(move || xs.as_observable().take_last_buffer(1))
    };

    assert_messages!(res.messages(),
        RT::on_next_where(650, |lst: &Vec<i32>| *lst == [9]),
        RT::on_completed::<Vec<i32>>(650),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 650),
    );
}

#[test]
fn take_last_buffer_one_error() {
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
        scheduler.start(move || xs.as_observable().take_last_buffer(1))
    };

    assert_messages!(res.messages(),
        RT::on_error::<Vec<i32>>(650, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 650),
    );
}

#[test]
fn take_last_buffer_one_disposed() {
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
        scheduler.start(move || xs.as_observable().take_last_buffer(1))
    };

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 1000),
    );
}

#[test]
fn take_last_buffer_three_completed() {
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
        scheduler.start(move || xs.as_observable().take_last_buffer(3))
    };

    assert_messages!(res.messages(),
        RT::on_next_where(650, |lst: &Vec<i32>| *lst == [7, 8, 9]),
        RT::on_completed::<Vec<i32>>(650),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 650),
    );
}

#[test]
fn take_last_buffer_three_error() {
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
        scheduler.start(move || xs.as_observable().take_last_buffer(3))
    };

    assert_messages!(res.messages(),
        RT::on_error::<Vec<i32>>(650, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 650),
    );
}

#[test]
fn take_last_buffer_three_disposed() {
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
        scheduler.start(move || xs.as_observable().take_last_buffer(3))
    };

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 1000),
    );
}

#[test]
fn take_last_buffer_zero1() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, 1),
        RT::on_next(220, 2),
        RT::on_completed::<i32>(230),
    ]);

    let res = {
        let xs = xs.clone();
        let s = scheduler.scheduler();
        scheduler.start(move || xs.as_observable().take_last_buffer_time_on(Duration::ZERO, s.clone()))
    };

    assert_messages!(res.messages(),
        RT::on_next_where(230, |lst: &Vec<i32>| lst.is_empty()),
        RT::on_completed::<Vec<i32>>(230),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 230),
    );
}

#[test]
fn take_last_buffer_zero2() {
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
        scheduler.start(move || xs.as_observable().take_last_buffer_time_on(Duration::ZERO, s.clone()))
    };

    assert_messages!(res.messages(),
        RT::on_next_where(230, |lst: &Vec<i32>| lst.is_empty()),
        RT::on_completed::<Vec<i32>>(230),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 230),
    );
}

#[test]
fn take_last_buffer_some1() {
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
        scheduler.start(move || xs.as_observable().take_last_buffer_time_on(ticks_to_duration(25), s.clone()))
    };

    assert_messages!(res.messages(),
        RT::on_next_where(240, |lst: &Vec<i32>| *lst == [2, 3]),
        RT::on_completed::<Vec<i32>>(240),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 240),
    );
}

#[test]
fn take_last_buffer_some2() {
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
        scheduler.start(move || xs.as_observable().take_last_buffer_time_on(ticks_to_duration(25), s.clone()))
    };

    assert_messages!(res.messages(),
        RT::on_next_where(300, |lst: &Vec<i32>| lst.is_empty()),
        RT::on_completed::<Vec<i32>>(300),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 300),
    );
}

#[test]
fn take_last_buffer_some3() {
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
        scheduler.start(move || xs.as_observable().take_last_buffer_time_on(ticks_to_duration(45), s.clone()))
    };

    assert_messages!(res.messages(),
        RT::on_next_where(300, |lst: &Vec<i32>| *lst == [6, 7, 8, 9]),
        RT::on_completed::<Vec<i32>>(300),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 300),
    );
}

#[test]
fn take_last_buffer_some4() {
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
        scheduler.start(move || xs.as_observable().take_last_buffer_time_on(ticks_to_duration(25), s.clone()))
    };

    assert_messages!(res.messages(),
        RT::on_next_where(350, |lst: &Vec<i32>| lst.is_empty()),
        RT::on_completed::<Vec<i32>>(350),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 350),
    );
}

#[test]
fn take_last_buffer_all() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, 1),
        RT::on_next(220, 2),
        RT::on_completed::<i32>(230),
    ]);

    let res = {
        let xs = xs.clone();
        let s = scheduler.scheduler();
        scheduler.start(move || xs.as_observable().take_last_buffer_time_on(ticks_to_duration(50), s.clone()))
    };

    assert_messages!(res.messages(),
        RT::on_next_where(230, |lst: &Vec<i32>| *lst == [1, 2]),
        RT::on_completed::<Vec<i32>>(230),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 230),
    );
}

#[test]
fn take_last_buffer_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_error::<i32>(210, ex.clone()),
    ]);

    let res = {
        let xs = xs.clone();
        let s = scheduler.scheduler();
        scheduler.start(move || xs.as_observable().take_last_buffer_time_on(ticks_to_duration(50), s.clone()))
    };

    assert_messages!(res.messages(),
        RT::on_error::<Vec<i32>>(210, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 210),
    );
}

#[test]
fn take_last_buffer_never() {
    let scheduler = TestScheduler::new();

    let _ex = new_error("ex");

    let xs = scheduler.create_hot_observable::<i32>(vec![]);

    let res = {
        let xs = xs.clone();
        let s = scheduler.scheduler();
        scheduler.start(move || xs.as_observable().take_last_buffer_time_on(ticks_to_duration(50), s.clone()))
    };

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 1000),
    );
}

// Not ported: TakeLastBuffer_ArgumentChecking - argument validation
// Not ported: TakeLastBuffer_Timed_ArgumentChecking - argument validation
// Not ported: TakeLastBuffer_Default1 - real threads
// Not ported: TakeLastBuffer_Default2 - real threads
// Not ported: TakeLastBuffer_Default3 - real threads
// Not ported: TakeLastBuffer_Default4 - real threads
