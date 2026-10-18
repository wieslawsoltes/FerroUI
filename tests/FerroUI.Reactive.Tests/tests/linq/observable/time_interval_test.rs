//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/TimeIntervalTest.cs` of Rx.NET.
//! Adapted: `scheduler.DisableOptimizations()` is a scheduler of this file that hands the work to
//! the test scheduler but offers none of its services, so that the operator measures time with the
//! emulated stopwatch (`DisableOptimizationsScheduler`); `TimeSpan.FromTicks` is
//! `ticks_to_duration`.

use ferroui_reactive::concurrency::{IScheduler, ScheduledAction};
use ferroui_reactive::{IDisposable, ObservableExt, TimeInterval};
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ticks_to_duration, ReactiveTest as RT, TestScheduler};
use std::rc::Rc;
use std::time::{Duration, SystemTime};

/// `scheduler.DisableOptimizations()`: the scheduler without the services it offers (its
/// stopwatch), so that operators fall back to the emulated ones (`DisableOptimizationsScheduler`).
struct DisableOptimizations {
    scheduler: Rc<dyn IScheduler>,
}

fn disable_optimizations(scheduler: Rc<dyn IScheduler>) -> Rc<dyn IScheduler> {
    Rc::new(DisableOptimizations { scheduler })
}

/// The work runs with the wrapped scheduler, so that what it schedules again keeps the wrapper.
fn wrap(action: ScheduledAction) -> ScheduledAction {
    Box::new(move |inner: &Rc<dyn IScheduler>| action(&disable_optimizations(inner.clone())))
}

impl IScheduler for DisableOptimizations {
    fn now(&self) -> SystemTime {
        self.scheduler.now()
    }

    fn schedule(&self, action: ScheduledAction) -> Rc<dyn IDisposable> {
        self.scheduler.schedule(wrap(action))
    }

    fn schedule_after(&self, due_time: Duration, action: ScheduledAction) -> Rc<dyn IDisposable> {
        self.scheduler.schedule_after(due_time, wrap(action))
    }

    fn schedule_at(&self, due_time: SystemTime, action: ScheduledAction) -> Rc<dyn IDisposable> {
        self.scheduler.schedule_at(due_time, wrap(action))
    }
}

fn regular_messages() -> Vec<ferroui_reactive_testing::Recorded<ferroui_reactive::Notification<i32>>> {
    vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_next(230, 3),
        RT::on_next(260, 4),
        RT::on_next(300, 5),
        RT::on_next(350, 6),
        RT::on_completed(400),
    ]
}

#[test]
fn time_interval_regular() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(regular_messages());

    let (source, s) = (xs.as_observable(), disable_optimizations(scheduler.scheduler()));
    let res = scheduler.start(move || source.time_interval_on(s));

    assert_messages!(
        res.messages(),
        RT::on_next(210, TimeInterval::new(2, ticks_to_duration(10))),
        RT::on_next(230, TimeInterval::new(3, ticks_to_duration(20))),
        RT::on_next(260, TimeInterval::new(4, ticks_to_duration(30))),
        RT::on_next(300, TimeInterval::new(5, ticks_to_duration(40))),
        RT::on_next(350, TimeInterval::new(6, ticks_to_duration(50))),
        RT::on_completed::<TimeInterval<i32>>(400),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 400));
}

#[test]
fn time_interval_empty() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(300)]);

    let (source, s) = (xs.as_observable(), disable_optimizations(scheduler.scheduler()));
    let res = scheduler.start(move || source.time_interval_on(s));

    assert_messages!(res.messages(), RT::on_completed::<TimeInterval<i32>>(300));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 300));
}

#[test]
fn time_interval_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_error(300, ex.clone())]);

    let (source, s) = (xs.as_observable(), disable_optimizations(scheduler.scheduler()));
    let res = scheduler.start(move || source.time_interval_on(s));

    assert_messages!(res.messages(), RT::on_error::<TimeInterval<i32>>(300, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 300));
}

#[test]
fn time_interval_never() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let (source, s) = (xs.as_observable(), disable_optimizations(scheduler.scheduler()));
    let res = scheduler.start(move || source.time_interval_on(s));

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn time_interval_with_stopwatch_regular() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(regular_messages());

    let (source, s) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || source.time_interval_on(s));

    assert_messages!(
        res.messages(),
        RT::on_next(210, TimeInterval::new(2, ticks_to_duration(10))),
        RT::on_next(230, TimeInterval::new(3, ticks_to_duration(20))),
        RT::on_next(260, TimeInterval::new(4, ticks_to_duration(30))),
        RT::on_next(300, TimeInterval::new(5, ticks_to_duration(40))),
        RT::on_next(350, TimeInterval::new(6, ticks_to_duration(50))),
        RT::on_completed::<TimeInterval<i32>>(400),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 400));
}

#[test]
fn time_interval_with_stopwatch_empty() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(300)]);

    let (source, s) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || source.time_interval_on(s));

    assert_messages!(res.messages(), RT::on_completed::<TimeInterval<i32>>(300));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 300));
}

#[test]
fn time_interval_with_stopwatch_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_error(300, ex.clone())]);

    let (source, s) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || source.time_interval_on(s));

    assert_messages!(res.messages(), RT::on_error::<TimeInterval<i32>>(300, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 300));
}

#[test]
fn time_interval_with_stopwatch_never() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let (source, s) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || source.time_interval_on(s));

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

// Not ported: TimeInterval_ArgumentChecking - argument validation
// Not ported: TimeInterval_DefaultScheduler - blocking operator First (waived: one thread)
