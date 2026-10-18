//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/TakeTest.cs` of Rx.NET.
//! The operators are applied to the handle of the test observable (`as_observable`); `Take(int, IScheduler)` is `take_on` and `Take(TimeSpan, IScheduler)` is `take_time_on`.

use ferroui_reactive::subjects::BehaviorSubject;
use ferroui_reactive::{IObserver, ObservableExt};
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ticks_to_duration, ReactiveTest as RT, TestScheduler};
use std::time::Duration;

#[test]
fn take_complete_after() {
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
        scheduler.start(move || xs.as_observable().take(20))
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
fn take_complete_same() {
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
        scheduler.start(move || xs.as_observable().take(17))
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
        RT::on_completed::<i32>(630),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 630),
    );
}

#[test]
fn take_complete_before() {
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
        scheduler.start(move || xs.as_observable().take(10))
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
        RT::on_completed::<i32>(415),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 415),
    );
}

#[test]
fn take_error_after() {
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
        scheduler.start(move || xs.as_observable().take(20))
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
        RT::on_error::<i32>(690, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 690),
    );
}

#[test]
fn take_error_same() {
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
        RT::on_error::<i32>(690, new_error("ex")),
    ]);

    let res = {
        let xs = xs.clone();
        scheduler.start(move || xs.as_observable().take(17))
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
        RT::on_completed::<i32>(630),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 630),
    );
}

#[test]
fn take_error_before() {
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
        RT::on_error::<i32>(690, new_error("ex")),
    ]);

    let res = {
        let xs = xs.clone();
        scheduler.start(move || xs.as_observable().take(3))
    };

    assert_messages!(res.messages(),
        RT::on_next(210, 9),
        RT::on_next(230, 13),
        RT::on_next(270, 7),
        RT::on_completed::<i32>(270),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 270),
    );
}

#[test]
fn take_dispose_before() {
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
        scheduler.start_disposed(move || xs.as_observable().take(3), 250)
    };

    assert_messages!(res.messages(),
        RT::on_next(210, 9),
        RT::on_next(230, 13),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 250),
    );
}

#[test]
fn take_dispose_after() {
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
        scheduler.start_disposed(move || xs.as_observable().take(3), 400)
    };

    assert_messages!(res.messages(),
        RT::on_next(210, 9),
        RT::on_next(230, 13),
        RT::on_next(270, 7),
        RT::on_completed::<i32>(270),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 270),
    );
}

#[test]
fn take_0_scheduler() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(70, 6),
        RT::on_next(150, 4),
        RT::on_next(210, 9),
        RT::on_next(230, 13),
    ]);

    let res = {
        let xs = xs.clone();
        let s = scheduler.scheduler();
        scheduler.start(move || xs.as_observable().take_on(0, s.clone()))
    };

    assert_messages!(res.messages(),
        // The extra scheduling of the empty sequence.
        RT::on_completed::<i32>(200 + 1),
    );

    assert_subscriptions!(xs.subscriptions());
}

#[test]
fn take_0_default_scheduler() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(70, 6),
        RT::on_next(150, 4),
        RT::on_next(210, 9),
        RT::on_next(230, 13),
    ]);

    let res = {
        let xs = xs.clone();
        scheduler.start(move || xs.as_observable().take(0))
    };

    assert_messages!(res.messages(),
        // Immediate.
        RT::on_completed::<i32>(200),
    );

    assert_subscriptions!(xs.subscriptions());
}

#[test]
fn take_non0_scheduler() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(70, 6),
        RT::on_next(150, 4),
        RT::on_next(210, 9),
        RT::on_next(230, 13),
    ]);

    let res = {
        let xs = xs.clone();
        let s = scheduler.scheduler();
        scheduler.start(move || xs.as_observable().take_on(1, s.clone()))
    };

    assert_messages!(res.messages(),
        RT::on_next(210, 9),
        RT::on_completed::<i32>(210),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 210),
    );
}

#[test]
fn take_take1() {
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
        scheduler.start(move || xs.as_observable().take(3).take(4))
    };

    assert_messages!(res.messages(),
        RT::on_next(210, 9),
        RT::on_next(230, 13),
        RT::on_next(270, 7),
        RT::on_completed::<i32>(270),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 270),
    );
}

#[test]
fn take_take2() {
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
        scheduler.start(move || xs.as_observable().take(4).take(3))
    };

    assert_messages!(res.messages(),
        RT::on_next(210, 9),
        RT::on_next(230, 13),
        RT::on_next(270, 7),
        RT::on_completed::<i32>(270),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 270),
    );
}

#[test]
fn take_decrements_count_first() {
    let k = BehaviorSubject::new(true);
    let target = k.clone();
    k.as_observable().take(1).subscribe_next(move |b| target.on_next(!b));

    // No assert needed; the test overflows the stack if it fails.
}

#[test]
fn take_zero() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, 1),
        RT::on_next(220, 2),
        RT::on_completed::<i32>(230),
    ]);

    let res = {
        let xs = xs.clone();
        let s = scheduler.scheduler();
        scheduler.start(move || xs.as_observable().take_time_on(Duration::ZERO, s.clone()))
    };

    assert_messages!(res.messages(),
        RT::on_completed::<i32>(201),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 201),
    );
}

#[test]
fn take_some() {
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
        scheduler.start(move || xs.as_observable().take_time_on(ticks_to_duration(25), s.clone()))
    };

    assert_messages!(res.messages(),
        RT::on_next(210, 1),
        RT::on_next(220, 2),
        RT::on_completed::<i32>(225),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 225),
    );
}

#[test]
fn take_late() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, 1),
        RT::on_next(220, 2),
        RT::on_completed::<i32>(230),
    ]);

    let res = {
        let xs = xs.clone();
        let s = scheduler.scheduler();
        scheduler.start(move || xs.as_observable().take_time_on(ticks_to_duration(50), s.clone()))
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
fn take_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_error::<i32>(210, ex.clone()),
    ]);

    let res = {
        let xs = xs.clone();
        let s = scheduler.scheduler();
        scheduler.start(move || xs.as_observable().take_time_on(ticks_to_duration(50), s.clone()))
    };

    assert_messages!(res.messages(),
        RT::on_error::<i32>(210, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 210),
    );
}

#[test]
fn take_never() {
    let scheduler = TestScheduler::new();

    let _ex = new_error("ex");

    let xs = scheduler.create_hot_observable::<i32>(vec![]);

    let res = {
        let xs = xs.clone();
        let s = scheduler.scheduler();
        scheduler.start(move || xs.as_observable().take_time_on(ticks_to_duration(50), s.clone()))
    };

    assert_messages!(res.messages(),
        RT::on_completed::<i32>(250),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 250),
    );
}

#[test]
fn take_twice1() {
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
        scheduler.start(move || xs.as_observable().take_time_on(ticks_to_duration(55), s.clone()).take_time_on(ticks_to_duration(35), s.clone()))
    };

    assert_messages!(res.messages(),
        RT::on_next(210, 1),
        RT::on_next(220, 2),
        RT::on_next(230, 3),
        RT::on_completed::<i32>(235),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 235),
    );
}

#[test]
fn take_twice2() {
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
        scheduler.start(move || xs.as_observable().take_time_on(ticks_to_duration(35), s.clone()).take_time_on(ticks_to_duration(55), s.clone()))
    };

    assert_messages!(res.messages(),
        RT::on_next(210, 1),
        RT::on_next(220, 2),
        RT::on_next(230, 3),
        RT::on_completed::<i32>(235),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 235),
    );
}

// Not ported: Take_ArgumentChecking - argument validation
// Not ported: Take_Timed_ArgumentChecking - argument validation
// Not ported: Take_Default - real threads
