//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/SkipTest.cs` of Rx.NET.
//! The operators are applied to the handle of the test observable (`as_observable`); `Skip(TimeSpan, IScheduler)` is `skip_time_on`.

use ferroui_reactive::ObservableExt;
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ticks_to_duration, ReactiveTest as RT, TestScheduler};
use std::time::Duration;

#[test]
fn skip_complete_after() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(70, 6),
        RT::on_next(150, 4),
        RT::on_next(210, 9),
        RT::on_next(230, 13),
        RT::on_next(270, 7),
        RT::on_next(280, 1),
        RT::on_next(300, -1),
        RT::on_next(310, 3),
        RT::on_next(340, 8),
        RT::on_next(370, 11),
        RT::on_next(410, 15),
        RT::on_next(415, 16),
        RT::on_next(460, 72),
        RT::on_next(510, 76),
        RT::on_next(560, 32),
        RT::on_next(570, -100),
        RT::on_next(580, -3),
        RT::on_next(590, 5),
        RT::on_next(630, 10),
        RT::on_completed::<i32>(690),
    ]);

    let res = {
        let xs = xs.clone();
        scheduler.start(move || xs.as_observable().skip(20))
    };

    assert_messages!(res.messages(),
        RT::on_completed::<i32>(690),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 690),
    );
}

#[test]
fn skip_complete_same() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(70, 6),
        RT::on_next(150, 4),
        RT::on_next(210, 9),
        RT::on_next(230, 13),
        RT::on_next(270, 7),
        RT::on_next(280, 1),
        RT::on_next(300, -1),
        RT::on_next(310, 3),
        RT::on_next(340, 8),
        RT::on_next(370, 11),
        RT::on_next(410, 15),
        RT::on_next(415, 16),
        RT::on_next(460, 72),
        RT::on_next(510, 76),
        RT::on_next(560, 32),
        RT::on_next(570, -100),
        RT::on_next(580, -3),
        RT::on_next(590, 5),
        RT::on_next(630, 10),
        RT::on_completed::<i32>(690),
    ]);

    let res = {
        let xs = xs.clone();
        scheduler.start(move || xs.as_observable().skip(17))
    };

    assert_messages!(res.messages(),
        RT::on_completed::<i32>(690),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 690),
    );
}

#[test]
fn skip_complete_before() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(70, 6),
        RT::on_next(150, 4),
        RT::on_next(210, 9),
        RT::on_next(230, 13),
        RT::on_next(270, 7),
        RT::on_next(280, 1),
        RT::on_next(300, -1),
        RT::on_next(310, 3),
        RT::on_next(340, 8),
        RT::on_next(370, 11),
        RT::on_next(410, 15),
        RT::on_next(415, 16),
        RT::on_next(460, 72),
        RT::on_next(510, 76),
        RT::on_next(560, 32),
        RT::on_next(570, -100),
        RT::on_next(580, -3),
        RT::on_next(590, 5),
        RT::on_next(630, 10),
        RT::on_completed::<i32>(690),
    ]);

    let res = {
        let xs = xs.clone();
        scheduler.start(move || xs.as_observable().skip(10))
    };

    assert_messages!(res.messages(),
        RT::on_next(460, 72),
        RT::on_next(510, 76),
        RT::on_next(560, 32),
        RT::on_next(570, -100),
        RT::on_next(580, -3),
        RT::on_next(590, 5),
        RT::on_next(630, 10),
        RT::on_completed::<i32>(690),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 690),
    );
}

#[test]
fn skip_complete_zero() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(70, 6),
        RT::on_next(150, 4),
        RT::on_next(210, 9),
        RT::on_next(230, 13),
        RT::on_next(270, 7),
        RT::on_next(280, 1),
        RT::on_next(300, -1),
        RT::on_next(310, 3),
        RT::on_next(340, 8),
        RT::on_next(370, 11),
        RT::on_next(410, 15),
        RT::on_next(415, 16),
        RT::on_next(460, 72),
        RT::on_next(510, 76),
        RT::on_next(560, 32),
        RT::on_next(570, -100),
        RT::on_next(580, -3),
        RT::on_next(590, 5),
        RT::on_next(630, 10),
        RT::on_completed::<i32>(690),
    ]);

    let res = {
        let xs = xs.clone();
        scheduler.start(move || xs.as_observable().skip(0))
    };

    assert_messages!(res.messages(),
        RT::on_next(210, 9),
        RT::on_next(230, 13),
        RT::on_next(270, 7),
        RT::on_next(280, 1),
        RT::on_next(300, -1),
        RT::on_next(310, 3),
        RT::on_next(340, 8),
        RT::on_next(370, 11),
        RT::on_next(410, 15),
        RT::on_next(415, 16),
        RT::on_next(460, 72),
        RT::on_next(510, 76),
        RT::on_next(560, 32),
        RT::on_next(570, -100),
        RT::on_next(580, -3),
        RT::on_next(590, 5),
        RT::on_next(630, 10),
        RT::on_completed::<i32>(690),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 690),
    );
}

#[test]
fn skip_error_after() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(70, 6),
        RT::on_next(150, 4),
        RT::on_next(210, 9),
        RT::on_next(230, 13),
        RT::on_next(270, 7),
        RT::on_next(280, 1),
        RT::on_next(300, -1),
        RT::on_next(310, 3),
        RT::on_next(340, 8),
        RT::on_next(370, 11),
        RT::on_next(410, 15),
        RT::on_next(415, 16),
        RT::on_next(460, 72),
        RT::on_next(510, 76),
        RT::on_next(560, 32),
        RT::on_next(570, -100),
        RT::on_next(580, -3),
        RT::on_next(590, 5),
        RT::on_next(630, 10),
        RT::on_error::<i32>(690, ex.clone()),
    ]);

    let res = {
        let xs = xs.clone();
        scheduler.start(move || xs.as_observable().skip(20))
    };

    assert_messages!(res.messages(),
        RT::on_error::<i32>(690, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 690),
    );
}

#[test]
fn skip_error_same() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(70, 6),
        RT::on_next(150, 4),
        RT::on_next(210, 9),
        RT::on_next(230, 13),
        RT::on_next(270, 7),
        RT::on_next(280, 1),
        RT::on_next(300, -1),
        RT::on_next(310, 3),
        RT::on_next(340, 8),
        RT::on_next(370, 11),
        RT::on_next(410, 15),
        RT::on_next(415, 16),
        RT::on_next(460, 72),
        RT::on_next(510, 76),
        RT::on_next(560, 32),
        RT::on_next(570, -100),
        RT::on_next(580, -3),
        RT::on_next(590, 5),
        RT::on_next(630, 10),
        RT::on_error::<i32>(690, ex.clone()),
    ]);

    let res = {
        let xs = xs.clone();
        scheduler.start(move || xs.as_observable().skip(17))
    };

    assert_messages!(res.messages(),
        RT::on_error::<i32>(690, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 690),
    );
}

#[test]
fn skip_error_before() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(70, 6),
        RT::on_next(150, 4),
        RT::on_next(210, 9),
        RT::on_next(230, 13),
        RT::on_next(270, 7),
        RT::on_next(280, 1),
        RT::on_next(300, -1),
        RT::on_next(310, 3),
        RT::on_next(340, 8),
        RT::on_next(370, 11),
        RT::on_next(410, 15),
        RT::on_next(415, 16),
        RT::on_next(460, 72),
        RT::on_next(510, 76),
        RT::on_next(560, 32),
        RT::on_next(570, -100),
        RT::on_next(580, -3),
        RT::on_next(590, 5),
        RT::on_next(630, 10),
        RT::on_error::<i32>(690, ex.clone()),
    ]);

    let res = {
        let xs = xs.clone();
        scheduler.start(move || xs.as_observable().skip(3))
    };

    assert_messages!(res.messages(),
        RT::on_next(280, 1),
        RT::on_next(300, -1),
        RT::on_next(310, 3),
        RT::on_next(340, 8),
        RT::on_next(370, 11),
        RT::on_next(410, 15),
        RT::on_next(415, 16),
        RT::on_next(460, 72),
        RT::on_next(510, 76),
        RT::on_next(560, 32),
        RT::on_next(570, -100),
        RT::on_next(580, -3),
        RT::on_next(590, 5),
        RT::on_next(630, 10),
        RT::on_error::<i32>(690, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 690),
    );
}

#[test]
fn skip_dispose_before() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(70, 6),
        RT::on_next(150, 4),
        RT::on_next(210, 9),
        RT::on_next(230, 13),
        RT::on_next(270, 7),
        RT::on_next(280, 1),
        RT::on_next(300, -1),
        RT::on_next(310, 3),
        RT::on_next(340, 8),
        RT::on_next(370, 11),
        RT::on_next(410, 15),
        RT::on_next(415, 16),
        RT::on_next(460, 72),
        RT::on_next(510, 76),
        RT::on_next(560, 32),
        RT::on_next(570, -100),
        RT::on_next(580, -3),
        RT::on_next(590, 5),
        RT::on_next(630, 10),
    ]);

    let res = {
        let xs = xs.clone();
        scheduler.start_disposed(move || xs.as_observable().skip(3), 250)
    };

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 250),
    );
}

#[test]
fn skip_dispose_after() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(70, 6),
        RT::on_next(150, 4),
        RT::on_next(210, 9),
        RT::on_next(230, 13),
        RT::on_next(270, 7),
        RT::on_next(280, 1),
        RT::on_next(300, -1),
        RT::on_next(310, 3),
        RT::on_next(340, 8),
        RT::on_next(370, 11),
        RT::on_next(410, 15),
        RT::on_next(415, 16),
        RT::on_next(460, 72),
        RT::on_next(510, 76),
        RT::on_next(560, 32),
        RT::on_next(570, -100),
        RT::on_next(580, -3),
        RT::on_next(590, 5),
        RT::on_next(630, 10),
    ]);

    let res = {
        let xs = xs.clone();
        scheduler.start_disposed(move || xs.as_observable().skip(3), 400)
    };

    assert_messages!(res.messages(),
        RT::on_next(280, 1),
        RT::on_next(300, -1),
        RT::on_next(310, 3),
        RT::on_next(340, 8),
        RT::on_next(370, 11),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 400),
    );
}

#[test]
fn skip_skip1() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(70, 6),
        RT::on_next(150, 4),
        RT::on_next(210, 9),
        RT::on_next(230, 13),
        RT::on_next(270, 7),
        RT::on_next(280, 1),
        RT::on_next(300, -1),
        RT::on_next(310, 3),
        RT::on_next(340, 8),
        RT::on_next(370, 11),
        RT::on_completed::<i32>(400),
    ]);

    let res = {
        let xs = xs.clone();
        scheduler.start(move || xs.as_observable().skip(3).skip(2))
    };

    assert_messages!(res.messages(),
        RT::on_next(310, 3),
        RT::on_next(340, 8),
        RT::on_next(370, 11),
        RT::on_completed::<i32>(400),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 400),
    );
}

#[test]
fn skip_zero() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, 1),
        RT::on_next(220, 2),
        RT::on_completed::<i32>(230),
    ]);

    let res = {
        let xs = xs.clone();
        let s = scheduler.scheduler();
        scheduler.start(move || xs.as_observable().skip_time_on(Duration::ZERO, s.clone()))
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
fn skip_some() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, 1),
        RT::on_next(220, 2),
        RT::on_completed::<i32>(230),
    ]);

    let res = {
        let xs = xs.clone();
        let s = scheduler.scheduler();
        scheduler.start(move || xs.as_observable().skip_time_on(ticks_to_duration(15), s.clone()))
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
fn skip_late() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, 1),
        RT::on_next(220, 2),
        RT::on_completed::<i32>(230),
    ]);

    let res = {
        let xs = xs.clone();
        let s = scheduler.scheduler();
        scheduler.start(move || xs.as_observable().skip_time_on(ticks_to_duration(50), s.clone()))
    };

    assert_messages!(res.messages(),
        RT::on_completed::<i32>(230),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 230),
    );
}

#[test]
fn skip_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_error::<i32>(210, ex.clone()),
    ]);

    let res = {
        let xs = xs.clone();
        let s = scheduler.scheduler();
        scheduler.start(move || xs.as_observable().skip_time_on(ticks_to_duration(50), s.clone()))
    };

    assert_messages!(res.messages(),
        RT::on_error::<i32>(210, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 210),
    );
}

#[test]
fn skip_never() {
    let scheduler = TestScheduler::new();

    let _ex = new_error("ex");

    let xs = scheduler.create_hot_observable::<i32>(vec![]);

    let res = {
        let xs = xs.clone();
        let s = scheduler.scheduler();
        scheduler.start(move || xs.as_observable().skip_time_on(ticks_to_duration(50), s.clone()))
    };

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 1000),
    );
}

#[test]
fn skip_twice1() {
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
        scheduler.start(move || xs.as_observable().skip_time_on(ticks_to_duration(15), s.clone()).skip_time_on(ticks_to_duration(30), s.clone()))
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
fn skip_twice2() {
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
        scheduler.start(move || xs.as_observable().skip_time_on(ticks_to_duration(30), s.clone()).skip_time_on(ticks_to_duration(15), s.clone()))
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

// Not ported: Skip_ArgumentChecking - argument validation
// Not ported: Skip_Timed_ArgumentChecking - argument validation
// Not ported: Skip_Default - real threads
