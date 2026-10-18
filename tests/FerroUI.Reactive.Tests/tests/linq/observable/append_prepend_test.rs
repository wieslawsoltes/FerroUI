//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/AppendPrependTest.cs` of Rx.NET.
//! Adapted: `Append(value, scheduler)` and `Prepend(value, scheduler)` are `append_on` and
//! `prepend_on`.

use ferroui_reactive::ObservableExt;
use ferroui_reactive_testing::{assert_messages, ReactiveTest as RT, TestScheduler};

#[test]
fn append_prepend() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(250)]);

    let res = scheduler.start(move || xs.as_observable().append(3).prepend(4));

    assert_messages!(
        res.messages(),
        RT::on_next(200, 4),
        RT::on_next(220, 2),
        RT::on_next(250, 3),
        RT::on_completed(250),
    );
}

#[test]
fn prepend_append() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(250)]);

    let res = scheduler.start(move || xs.as_observable().prepend(4).append(3));

    assert_messages!(
        res.messages(),
        RT::on_next(200, 4),
        RT::on_next(220, 2),
        RT::on_next(250, 3),
        RT::on_completed(250),
    );
}

#[test]
fn append_prepend_scheduler() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(250)]);

    let s = scheduler.scheduler();
    let res = scheduler.start(move || xs.as_observable().append_on(3, s.clone()).prepend_on(4, s));

    assert_messages!(
        res.messages(),
        RT::on_next(201, 4),
        RT::on_next(220, 2),
        RT::on_next(251, 3),
        RT::on_completed(251),
    );
}

#[test]
fn prepend_append_scheduler() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(250)]);

    let s = scheduler.scheduler();
    let res = scheduler.start(move || xs.as_observable().prepend_on(4, s.clone()).append_on(3, s));

    assert_messages!(
        res.messages(),
        RT::on_next(201, 4),
        RT::on_next(220, 2),
        RT::on_next(251, 3),
        RT::on_completed(251),
    );
}

#[test]
fn append_prepend_many() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(250)]);

    let res = scheduler.start(move || xs.as_observable().append(3).append(4).append(5).prepend(6).prepend(7).prepend(8));

    assert_messages!(
        res.messages(),
        RT::on_next(200, 8),
        RT::on_next(200, 7),
        RT::on_next(200, 6),
        RT::on_next(220, 2),
        RT::on_next(250, 3),
        RT::on_next(250, 4),
        RT::on_next(250, 5),
        RT::on_completed(250),
    );
}

#[test]
fn prepend_append_many() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(250)]);

    let res = scheduler.start(move || xs.as_observable().prepend(6).prepend(7).prepend(8).append(3).append(4).append(5));

    assert_messages!(
        res.messages(),
        RT::on_next(200, 8),
        RT::on_next(200, 7),
        RT::on_next(200, 6),
        RT::on_next(220, 2),
        RT::on_next(250, 3),
        RT::on_next(250, 4),
        RT::on_next(250, 5),
        RT::on_completed(250),
    );
}

#[test]
fn append_prepend_many_scheduler() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(250)]);

    let s = scheduler.scheduler();
    let res = scheduler.start(move || {
        xs.as_observable()
            .append_on(3, s.clone())
            .append_on(4, s.clone())
            .append_on(5, s.clone())
            .prepend_on(6, s.clone())
            .prepend_on(7, s.clone())
            .prepend_on(8, s)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(201, 8),
        RT::on_next(202, 7),
        RT::on_next(203, 6),
        RT::on_next(220, 2),
        RT::on_next(251, 3),
        RT::on_next(252, 4),
        RT::on_next(253, 5),
        RT::on_completed(253),
    );
}

#[test]
fn prepend_append_many_scheduler() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(250)]);

    let s = scheduler.scheduler();
    let res = scheduler.start(move || {
        xs.as_observable()
            .prepend_on(6, s.clone())
            .prepend_on(7, s.clone())
            .prepend_on(8, s.clone())
            .append_on(3, s.clone())
            .append_on(4, s.clone())
            .append_on(5, s)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(201, 8),
        RT::on_next(202, 7),
        RT::on_next(203, 6),
        RT::on_next(220, 2),
        RT::on_next(251, 3),
        RT::on_next(252, 4),
        RT::on_next(253, 5),
        RT::on_completed(253),
    );
}

#[test]
fn append_prepend_mixed() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(250)]);

    let res = scheduler.start(move || xs.as_observable().append(3).prepend(6).append(4).prepend(7).prepend(8).append(5));

    assert_messages!(
        res.messages(),
        RT::on_next(200, 8),
        RT::on_next(200, 7),
        RT::on_next(200, 6),
        RT::on_next(220, 2),
        RT::on_next(250, 3),
        RT::on_next(250, 4),
        RT::on_next(250, 5),
        RT::on_completed(250),
    );
}

#[test]
fn append_prepend_mixed_scheduler() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(250)]);

    let s = scheduler.scheduler();
    let res = scheduler.start(move || {
        xs.as_observable()
            .append_on(3, s.clone())
            .prepend_on(6, s.clone())
            .append_on(4, s.clone())
            .prepend_on(7, s.clone())
            .prepend_on(8, s.clone())
            .append_on(5, s)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(201, 8),
        RT::on_next(202, 7),
        RT::on_next(203, 6),
        RT::on_next(220, 2),
        RT::on_next(251, 3),
        RT::on_next(252, 4),
        RT::on_next(253, 5),
        RT::on_completed(253),
    );
}

#[test]
fn append_prepend_mixed_schedulers() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(250)]);

    let s = scheduler.scheduler();
    let res = scheduler.start(move || {
        xs.as_observable()
            .append_on(3, s.clone())
            .prepend(6)
            .append_on(4, s.clone())
            .prepend(7)
            .prepend_on(8, s)
            .append(5)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(201, 8),
        RT::on_next(201, 7),
        RT::on_next(201, 6),
        RT::on_next(220, 2),
        RT::on_next(251, 3),
        RT::on_next(252, 4),
        RT::on_next(252, 5),
        RT::on_completed(252),
    );
}

// Not ported: AppendPrepend_SchedulerRecursive - real threads (EventLoopScheduler, ThreadPoolScheduler); blocking operator First (waived: one thread)
// Not ported: AppendPrepend_SchedulerLongRunning - real threads (NewThreadScheduler, ThreadPoolScheduler, ISchedulerLongRunning); blocking operator First (waived: one thread)
