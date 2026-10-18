//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/DelayTest.cs` of Rx.NET.
//! Adapted: `TimeSpan.FromTicks` is `ticks_to_duration` and `new DateTimeOffset(ticks,
//! TimeSpan.Zero)` the time of the test scheduler at those ticks (`ticks_to_time`);
//! `scheduler.DisableOptimizations()` is a scheduler of this file that hands the work to the test
//! scheduler but offers none of its services, so that the operator measures time with the emulated
//! stopwatch (`DisableOptimizationsScheduler`). The overloads are `delay_on` (`Delay(TimeSpan,
//! IScheduler)`), `delay_at_on` (`Delay(DateTimeOffset, IScheduler)`), `delay_by` (a delay
//! selector) and `delay_by_with_subscription_delay` (a subscription delay and a delay selector); a
//! selector that throws is the `try_` form returning `Err`. The values a test collects and the
//! events it waits for are shared cells; `Delay_ErrorHandling1` keeps the work of its scheduler and
//! runs it when the test gives the impulse, where upstream releases a thread of the thread pool.

use ferroui_reactive::concurrency::{IScheduler, ScheduledAction};
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::subjects::Subject;
use ferroui_reactive::{IDisposable, IObserver, Observable, ObservableExt};
use ferroui_reactive_testing::{
    assert_messages, assert_recorded_eq, assert_subscriptions, new_error, ticks_to_duration, ticks_to_time, Expected, ReactiveTest as RT,
    Recorded, TestScheduler,
};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
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

/// `useStopwatch ? scheduler : scheduler.DisableOptimizations()`.
fn scheduler_for(scheduler: &TestScheduler, use_stopwatch: bool) -> Rc<dyn IScheduler> {
    if use_stopwatch {
        scheduler.scheduler()
    } else {
        disable_optimizations(scheduler.scheduler())
    }
}

#[test]
fn delay_time_span_simple1() {
    delay_time_span_simple1_impl(false);
}

#[test]
fn delay_time_span_simple1_stopwatch() {
    delay_time_span_simple1_impl(true);
}

fn delay_time_span_simple1_impl(use_stopwatch: bool) {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(250, 2),
        RT::on_next(350, 3),
        RT::on_next(450, 4),
        RT::on_completed(550),
    ]);

    let (source, s) = (xs.as_observable(), scheduler_for(&scheduler, use_stopwatch));
    let res = scheduler.start(move || source.delay_on(ticks_to_duration(100), s));

    assert_messages!(res.messages(), RT::on_next(350, 2), RT::on_next(450, 3), RT::on_next(550, 4), RT::on_completed::<i32>(650));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 550));
}

#[test]
fn delay_date_time_offset_simple1() {
    delay_date_time_offset_simple1_impl(false);
}

#[test]
fn delay_date_time_offset_simple1_stopwatch() {
    delay_date_time_offset_simple1_impl(true);
}

fn delay_date_time_offset_simple1_impl(use_stopwatch: bool) {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(250, 2),
        RT::on_next(350, 3),
        RT::on_next(450, 4),
        RT::on_completed(550),
    ]);

    let (source, s) = (xs.as_observable(), scheduler_for(&scheduler, use_stopwatch));
    let res = scheduler.start(move || source.delay_at_on(ticks_to_time(300), s));

    assert_messages!(res.messages(), RT::on_next(350, 2), RT::on_next(450, 3), RT::on_next(550, 4), RT::on_completed::<i32>(650));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 550));
}

#[test]
fn delay_time_span_simple2() {
    delay_time_span_simple2_impl(false);
}

#[test]
fn delay_time_span_simple2_stopwatch() {
    delay_time_span_simple2_impl(true);
}

fn delay_time_span_simple2_impl(use_stopwatch: bool) {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(250, 2),
        RT::on_next(350, 3),
        RT::on_next(450, 4),
        RT::on_completed(550),
    ]);

    let (source, s) = (xs.as_observable(), scheduler_for(&scheduler, use_stopwatch));
    let res = scheduler.start(move || source.delay_on(ticks_to_duration(50), s));

    assert_messages!(res.messages(), RT::on_next(300, 2), RT::on_next(400, 3), RT::on_next(500, 4), RT::on_completed::<i32>(600));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 550));
}

#[test]
fn delay_date_time_offset_simple2() {
    delay_date_time_offset_simple2_impl(false);
}

#[test]
fn delay_date_time_offset_simple2_stopwatch() {
    delay_date_time_offset_simple2_impl(true);
}

fn delay_date_time_offset_simple2_impl(use_stopwatch: bool) {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(250, 2),
        RT::on_next(350, 3),
        RT::on_next(450, 4),
        RT::on_completed(550),
    ]);

    let (source, s) = (xs.as_observable(), scheduler_for(&scheduler, use_stopwatch));
    let res = scheduler.start(move || source.delay_at_on(ticks_to_time(250), s));

    assert_messages!(res.messages(), RT::on_next(300, 2), RT::on_next(400, 3), RT::on_next(500, 4), RT::on_completed::<i32>(600));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 550));
}

#[test]
fn delay_time_span_simple3() {
    delay_time_span_simple3_impl(false);
}

#[test]
fn delay_time_span_simple3_stopwatch() {
    delay_time_span_simple3_impl(true);
}

fn delay_time_span_simple3_impl(use_stopwatch: bool) {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(250, 2),
        RT::on_next(350, 3),
        RT::on_next(450, 4),
        RT::on_completed(550),
    ]);

    let (source, s) = (xs.as_observable(), scheduler_for(&scheduler, use_stopwatch));
    let res = scheduler.start(move || source.delay_on(ticks_to_duration(150), s));

    assert_messages!(res.messages(), RT::on_next(400, 2), RT::on_next(500, 3), RT::on_next(600, 4), RT::on_completed::<i32>(700));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 550));
}

#[test]
fn delay_date_time_offset_simple3() {
    delay_date_time_offset_simple3_impl(false);
}

#[test]
fn delay_date_time_offset_simple3_stopwatch() {
    delay_date_time_offset_simple3_impl(true);
}

fn delay_date_time_offset_simple3_impl(use_stopwatch: bool) {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(250, 2),
        RT::on_next(350, 3),
        RT::on_next(450, 4),
        RT::on_completed(550),
    ]);

    let (source, s) = (xs.as_observable(), scheduler_for(&scheduler, use_stopwatch));
    let res = scheduler.start(move || source.delay_at_on(ticks_to_time(350), s));

    assert_messages!(res.messages(), RT::on_next(400, 2), RT::on_next(500, 3), RT::on_next(600, 4), RT::on_completed::<i32>(700));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 550));
}

#[test]
fn delay_time_span_error1() {
    delay_time_span_error1_impl(false);
}

#[test]
fn delay_time_span_error1_stopwatch() {
    delay_time_span_error1_impl(true);
}

fn delay_time_span_error1_impl(use_stopwatch: bool) {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(250, 2),
        RT::on_next(350, 3),
        RT::on_next(450, 4),
        RT::on_error(550, ex.clone()),
    ]);

    let (source, s) = (xs.as_observable(), scheduler_for(&scheduler, use_stopwatch));
    let res = scheduler.start(move || source.delay_on(ticks_to_duration(50), s));

    assert_messages!(
        res.messages(),
        RT::on_next(300, 2),
        RT::on_next(400, 3),
        RT::on_next(500, 4),
        RT::on_error::<i32>(550, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 550));
}

#[test]
fn delay_date_time_offset_error1() {
    delay_date_time_offset_error1_impl(false);
}

#[test]
fn delay_date_time_offset_error1_stopwatch() {
    delay_date_time_offset_error1_impl(true);
}

fn delay_date_time_offset_error1_impl(use_stopwatch: bool) {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(250, 2),
        RT::on_next(350, 3),
        RT::on_next(450, 4),
        RT::on_error(550, ex.clone()),
    ]);

    let (source, s) = (xs.as_observable(), scheduler_for(&scheduler, use_stopwatch));
    let res = scheduler.start(move || source.delay_at_on(ticks_to_time(250), s));

    assert_messages!(
        res.messages(),
        RT::on_next(300, 2),
        RT::on_next(400, 3),
        RT::on_next(500, 4),
        RT::on_error::<i32>(550, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 550));
}

#[test]
fn delay_time_span_error2() {
    delay_time_span_error2_impl(false);
}

#[test]
fn delay_time_span_error2_stopwatch() {
    delay_time_span_error2_impl(true);
}

fn delay_time_span_error2_impl(use_stopwatch: bool) {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(250, 2),
        RT::on_next(350, 3),
        RT::on_next(450, 4),
        RT::on_error(550, ex.clone()),
    ]);

    let (source, s) = (xs.as_observable(), scheduler_for(&scheduler, use_stopwatch));
    let res = scheduler.start(move || source.delay_on(ticks_to_duration(150), s));

    assert_messages!(res.messages(), RT::on_next(400, 2), RT::on_next(500, 3), RT::on_error::<i32>(550, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 550));
}

#[test]
fn delay_date_time_offset_error2() {
    delay_date_time_offset_error2_impl(false);
}

#[test]
fn delay_date_time_offset_error2_stopwatch() {
    delay_date_time_offset_error2_impl(true);
}

fn delay_date_time_offset_error2_impl(use_stopwatch: bool) {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(250, 2),
        RT::on_next(350, 3),
        RT::on_next(450, 4),
        RT::on_error(550, ex.clone()),
    ]);

    let (source, s) = (xs.as_observable(), scheduler_for(&scheduler, use_stopwatch));
    let res = scheduler.start(move || source.delay_at_on(ticks_to_time(350), s));

    assert_messages!(res.messages(), RT::on_next(400, 2), RT::on_next(500, 3), RT::on_error::<i32>(550, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 550));
}

#[test]
fn delay_time_span_positive() {
    let scheduler = TestScheduler::new();

    let msgs = vec![RT::on_next(150, 1), RT::on_next(250, 2), RT::on_next(350, 3), RT::on_next(450, 4), RT::on_completed(550)];

    let xs = scheduler.create_hot_observable(msgs.clone());

    const DELAY: u16 = 42;

    let (source, s) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || source.delay_on(ticks_to_duration(i64::from(DELAY)), s));

    let expected: Vec<Recorded<Expected<i32>>> = msgs
        .iter()
        .filter(|n| n.time > RT::SUBSCRIBED)
        .map(|n| Recorded::new(i64::from((n.time + i64::from(DELAY)) as u16), n.value.clone()).into())
        .collect();

    assert_recorded_eq(&res.messages(), &expected);
}

#[test]
fn delay_empty() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(550)]);

    let (source, s) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || source.delay_on(ticks_to_duration(10), s));

    assert_messages!(res.messages(), RT::on_completed::<i32>(560));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 550));
}

#[test]
fn delay_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_error(550, ex.clone())]);

    let (source, s) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || source.delay_on(ticks_to_duration(10), s));

    assert_messages!(res.messages(), RT::on_error::<i32>(550, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 550));
}

#[test]
fn delay_never() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let (source, s) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || source.delay_on(ticks_to_duration(10), s));

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

/// The default scheduler of time-based operations is the current-thread scheduler until an
/// application sets another, so the whole exchange runs within the first `on_next`, the delays
/// waited for on the thread.
#[test]
fn delay_crossing_messages() {
    let lst = Rc::new(RefCell::new(Vec::new()));

    let evt = Rc::new(Cell::new(false));

    let s = Subject::<i32>::new();
    {
        let (lst, evt, subject) = (lst.clone(), evt.clone(), s.clone());
        s.as_observable().delay(Duration::from_secs_f64(0.01)).subscribe_next_completed(
            move |x| {
                lst.borrow_mut().push(x);
                if x < 9 {
                    subject.on_next(x + 1);
                } else {
                    subject.on_completed();
                }
            },
            move || evt.set(true),
        );
    }
    s.on_next(0);

    assert!(evt.get());

    assert_eq!((0..10).collect::<Vec<i32>>(), *lst.borrow());
}

#[test]
fn delay_duration_simple1() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 10),
        RT::on_next(220, 30),
        RT::on_next(230, 50),
        RT::on_next(240, 35),
        RT::on_next(250, 20),
        RT::on_completed(260),
    ]);

    let (source, sch) = (xs.as_observable(), scheduler.clone());
    let res = scheduler.start(move || {
        source.delay_by(move |x: &i32| sch.create_cold_observable(vec![RT::on_next(i64::from(*x), "!")]).as_observable())
    });

    assert_messages!(
        res.messages(),
        RT::on_next(210 + 10, 10),
        RT::on_next(220 + 30, 30),
        RT::on_next(250 + 20, 20),
        RT::on_next(240 + 35, 35),
        RT::on_next(230 + 50, 50),
        RT::on_completed::<i32>(280),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 260));
}

#[test]
fn delay_duration_simple2() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_next(230, 4),
        RT::on_next(240, 5),
        RT::on_next(250, 6),
        RT::on_completed(300),
    ]);

    let ys = scheduler.create_cold_observable(vec![RT::on_next(10, "!")]);

    let (source, delays) = (xs.as_observable(), ys.as_observable());
    let res = scheduler.start(move || source.delay_by(move |_: &i32| delays.clone()));

    assert_messages!(
        res.messages(),
        RT::on_next(210 + 10, 2),
        RT::on_next(220 + 10, 3),
        RT::on_next(230 + 10, 4),
        RT::on_next(240 + 10, 5),
        RT::on_next(250 + 10, 6),
        RT::on_completed::<i32>(300),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 300));

    assert_subscriptions!(
        ys.subscriptions(),
        RT::subscribe(210, 220),
        RT::subscribe(220, 230),
        RT::subscribe(230, 240),
        RT::subscribe(240, 250),
        RT::subscribe(250, 260),
    );
}

#[test]
fn delay_duration_simple3() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_next(230, 4),
        RT::on_next(240, 5),
        RT::on_next(250, 6),
        RT::on_completed(300),
    ]);

    let ys = scheduler.create_cold_observable(vec![RT::on_next(100, "!")]);

    let (source, delays) = (xs.as_observable(), ys.as_observable());
    let res = scheduler.start(move || source.delay_by(move |_: &i32| delays.clone()));

    assert_messages!(
        res.messages(),
        RT::on_next(210 + 100, 2),
        RT::on_next(220 + 100, 3),
        RT::on_next(230 + 100, 4),
        RT::on_next(240 + 100, 5),
        RT::on_next(250 + 100, 6),
        RT::on_completed::<i32>(350),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 300));

    assert_subscriptions!(
        ys.subscriptions(),
        RT::subscribe(210, 310),
        RT::subscribe(220, 320),
        RT::subscribe(230, 330),
        RT::subscribe(240, 340),
        RT::subscribe(250, 350),
    );
}

#[test]
fn delay_duration_simple4_inner_empty() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_next(230, 4),
        RT::on_next(240, 5),
        RT::on_next(250, 6),
        RT::on_completed(300),
    ]);

    let ys = scheduler.create_cold_observable(vec![RT::on_completed::<i32>(100)]);

    let (source, delays) = (xs.as_observable(), ys.as_observable());
    let res = scheduler.start(move || source.delay_by(move |_: &i32| delays.clone()));

    assert_messages!(
        res.messages(),
        RT::on_next(210 + 100, 2),
        RT::on_next(220 + 100, 3),
        RT::on_next(230 + 100, 4),
        RT::on_next(240 + 100, 5),
        RT::on_next(250 + 100, 6),
        RT::on_completed::<i32>(350),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 300));

    assert_subscriptions!(
        ys.subscriptions(),
        RT::subscribe(210, 310),
        RT::subscribe(220, 320),
        RT::subscribe(230, 330),
        RT::subscribe(240, 340),
        RT::subscribe(250, 350),
    );
}

#[test]
fn delay_duration_dispose1() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_next(230, 4),
        RT::on_next(240, 5),
        RT::on_next(250, 6),
        RT::on_completed(300),
    ]);

    let ys = scheduler.create_cold_observable(vec![RT::on_next(200, "!")]);

    let (source, delays) = (xs.as_observable(), ys.as_observable());
    let res = scheduler.start_disposed(move || source.delay_by(move |_: &i32| delays.clone()), 425);

    assert_messages!(res.messages(), RT::on_next(210 + 200, 2), RT::on_next(220 + 200, 3));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 300));

    assert_subscriptions!(
        ys.subscriptions(),
        RT::subscribe(210, 410),
        RT::subscribe(220, 420),
        RT::subscribe(230, 425),
        RT::subscribe(240, 425),
        RT::subscribe(250, 425),
    );
}

#[test]
fn delay_duration_dispose2() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2), RT::on_next(400, 3), RT::on_completed(500)]);

    let ys = scheduler.create_cold_observable(vec![RT::on_next(50, "!")]);

    let (source, delays) = (xs.as_observable(), ys.as_observable());
    let res = scheduler.start_disposed(move || source.delay_by(move |_: &i32| delays.clone()), 300);

    assert_messages!(res.messages(), RT::on_next(210 + 50, 2));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 300));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(210, 260));
}

#[test]
fn delay_duration_outer_error1() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_next(230, 4),
        RT::on_next(240, 5),
        RT::on_next(250, 6),
        RT::on_error(300, ex.clone()),
    ]);

    let ys = scheduler.create_cold_observable(vec![RT::on_next(100, "!")]);

    let (source, delays) = (xs.as_observable(), ys.as_observable());
    let res = scheduler.start(move || source.delay_by(move |_: &i32| delays.clone()));

    assert_messages!(res.messages(), RT::on_error::<i32>(300, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 300));

    assert_subscriptions!(
        ys.subscriptions(),
        RT::subscribe(210, 300),
        RT::subscribe(220, 300),
        RT::subscribe(230, 300),
        RT::subscribe(240, 300),
        RT::subscribe(250, 300),
    );
}

#[test]
fn delay_duration_outer_error2() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_next(230, 4),
        RT::on_next(240, 5),
        RT::on_next(250, 6),
        RT::on_error(300, ex.clone()),
    ]);

    let ys = scheduler.create_cold_observable(vec![RT::on_next(10, "!")]);

    let (source, delays) = (xs.as_observable(), ys.as_observable());
    let res = scheduler.start(move || source.delay_by(move |_: &i32| delays.clone()));

    assert_messages!(
        res.messages(),
        RT::on_next(210 + 10, 2),
        RT::on_next(220 + 10, 3),
        RT::on_next(230 + 10, 4),
        RT::on_next(240 + 10, 5),
        RT::on_next(250 + 10, 6),
        RT::on_error::<i32>(300, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 300));

    assert_subscriptions!(
        ys.subscriptions(),
        RT::subscribe(210, 220),
        RT::subscribe(220, 230),
        RT::subscribe(230, 240),
        RT::subscribe(240, 250),
        RT::subscribe(250, 260),
    );
}

#[test]
fn delay_duration_inner_error1() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_next(230, 4),
        RT::on_next(240, 5),
        RT::on_next(250, 6),
        RT::on_completed(300),
    ]);

    let ys = scheduler.create_cold_observable(vec![RT::on_next(30, "!")]);

    let zs = scheduler.create_cold_observable(vec![RT::on_error::<&str>(25, ex.clone())]);

    let (source, delays, failing) = (xs.as_observable(), ys.as_observable(), zs.as_observable());
    let res = scheduler.start(move || source.delay_by(move |x: &i32| if *x != 5 { delays.clone() } else { failing.clone() }));

    assert_messages!(
        res.messages(),
        RT::on_next(210 + 30, 2),
        RT::on_next(220 + 30, 3),
        RT::on_next(230 + 30, 4),
        RT::on_error::<i32>(240 + 25, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 265));

    assert_subscriptions!(
        ys.subscriptions(),
        RT::subscribe(210, 240),
        RT::subscribe(220, 250),
        RT::subscribe(230, 260),
        RT::subscribe(250, 265),
    );
}

#[test]
fn delay_duration_inner_error2() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_next(230, 4),
        RT::on_next(240, 5),
        RT::on_next(250, 6),
        RT::on_completed(300),
    ]);

    let ys = scheduler.create_cold_observable(vec![RT::on_error::<&str>(100, ex.clone())]);

    let (source, delays) = (xs.as_observable(), ys.as_observable());
    let res = scheduler.start(move || source.delay_by(move |_: &i32| delays.clone()));

    assert_messages!(res.messages(), RT::on_error::<i32>(210 + 100, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 300));

    assert_subscriptions!(
        ys.subscriptions(),
        RT::subscribe(210, 310),
        RT::subscribe(220, 310),
        RT::subscribe(230, 310),
        RT::subscribe(240, 310),
        RT::subscribe(250, 310),
    );
}

#[test]
fn delay_duration_selector_throws1() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(250, 2),
        RT::on_next(300, 3),
        RT::on_next(350, 4),
        RT::on_next(400, 5),
        RT::on_next(450, 6),
        RT::on_completed(500),
    ]);

    let ys = scheduler.create_cold_observable(vec![RT::on_next(80, "")]);

    let (source, delays, error) = (xs.as_observable(), ys.as_observable(), ex.clone());
    let res = scheduler.start(move || {
        source.try_delay_by(move |x: &i32| {
            if *x == 4 {
                return Err(error.clone());
            }

            Ok(delays.clone())
        })
    });

    assert_messages!(res.messages(), RT::on_next(330, 2), RT::on_error::<i32>(350, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 350));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(250, 330), RT::subscribe(300, 350));
}

#[test]
fn delay_duration_simple() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(250, 2),
        RT::on_next(350, 3),
        RT::on_next(450, 4),
        RT::on_completed(550),
    ]);

    let (source, sch) = (xs.as_observable(), scheduler.clone());
    let res = scheduler.start(move || {
        source.delay_by(move |x: &i32| {
            let x = i64::from(*x);
            sch.create_cold_observable(vec![RT::on_next(x * 10, "Ignore"), RT::on_next(x * 10 + 5, "Aargh!")]).as_observable()
        })
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250 + 2 * 10, 2),
        RT::on_next(350 + 3 * 10, 3),
        RT::on_next(450 + 4 * 10, 4),
        RT::on_completed::<i32>(550),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 550));
}

#[test]
fn delay_duration_defer_on_completed() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(250, 2),
        RT::on_next(350, 3),
        RT::on_next(450, 4),
        RT::on_completed(451),
    ]);

    let (source, sch) = (xs.as_observable(), scheduler.clone());
    let res = scheduler.start(move || {
        source.delay_by(move |x: &i32| {
            let x = i64::from(*x);
            sch.create_cold_observable(vec![RT::on_next(x * 10, "Ignore"), RT::on_next(x * 10 + 5, "Aargh!")]).as_observable()
        })
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250 + 2 * 10, 2),
        RT::on_next(350 + 3 * 10, 3),
        RT::on_next(450 + 4 * 10, 4),
        RT::on_completed::<i32>(450 + 4 * 10),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 451));
}

#[test]
fn delay_duration_inner_error() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(250, 2),
        RT::on_next(350, 3),
        RT::on_next(450, 4),
        RT::on_completed(451),
    ]);

    let ex = new_error("ex");

    let (source, sch, error) = (xs.as_observable(), scheduler.clone(), ex.clone());
    let res = scheduler.start(move || {
        source.delay_by(move |x: &i32| {
            let x = i64::from(*x);
            if x < 4 {
                sch.create_cold_observable(vec![RT::on_next(x * 10, "Ignore"), RT::on_next(x * 10 + 5, "Aargh!")]).as_observable()
            } else {
                sch.create_cold_observable(vec![RT::on_error::<&str>(x * 10, error.clone())]).as_observable()
            }
        })
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250 + 2 * 10, 2),
        RT::on_next(350 + 3 * 10, 3),
        RT::on_error::<i32>(450 + 4 * 10, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 451));
}

#[test]
fn delay_duration_outer_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(250, 2),
        RT::on_next(350, 3),
        RT::on_next(450, 4),
        RT::on_error(460, ex.clone()),
    ]);

    let (source, sch) = (xs.as_observable(), scheduler.clone());
    let res = scheduler.start(move || {
        source.delay_by(move |x: &i32| {
            let x = i64::from(*x);
            sch.create_cold_observable(vec![RT::on_next(x * 10, "Ignore"), RT::on_next(x * 10 + 5, "Aargh!")]).as_observable()
        })
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250 + 2 * 10, 2),
        RT::on_next(350 + 3 * 10, 3),
        RT::on_error::<i32>(460, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 460));
}

#[test]
fn delay_duration_selector_throws2() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(250, 2),
        RT::on_next(350, 3),
        RT::on_next(450, 4),
        RT::on_completed(550),
    ]);

    let ex = new_error("ex");

    let (source, sch, error) = (xs.as_observable(), scheduler.clone(), ex.clone());
    let res = scheduler.start(move || {
        source.try_delay_by(move |x: &i32| {
            let x = i64::from(*x);
            if x < 4 {
                Ok(sch.create_cold_observable(vec![RT::on_next(x * 10, "Ignore"), RT::on_next(x * 10 + 5, "Aargh!")]).as_observable())
            } else {
                Err(error.clone())
            }
        })
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250 + 2 * 10, 2),
        RT::on_next(350 + 3 * 10, 3),
        RT::on_error::<i32>(450, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 450));
}

#[test]
fn delay_duration_selector_immediately() {
    let list = Rc::new(RefCell::new(Vec::new()));

    let target = list.clone();
    Observable::range(1, 5).delay_by(|_: &i32| Observable::return_(1)).subscribe_next(move |x| target.borrow_mut().push(x));

    assert_eq!(vec![1, 2, 3, 4, 5], *list.borrow());
}

#[test]
fn delay_duration_inner_done() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(250, 2),
        RT::on_next(350, 3),
        RT::on_next(450, 4),
        RT::on_completed(550),
    ]);

    let (source, sch) = (xs.as_observable(), scheduler.clone());
    let res = scheduler.start(move || {
        source.delay_by(move |x: &i32| sch.create_cold_observable(vec![RT::on_completed::<&str>(i64::from(*x) * 10)]).as_observable())
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250 + 2 * 10, 2),
        RT::on_next(350 + 3 * 10, 3),
        RT::on_next(450 + 4 * 10, 4),
        RT::on_completed::<i32>(550),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 550));
}

#[test]
fn delay_duration_inner_subscription_times() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, -1),
        RT::on_next(250, 0),
        RT::on_next(350, 1),
        RT::on_next(450, 2),
        RT::on_completed(550),
    ]);

    let ys = vec![
        scheduler.create_cold_observable(vec![RT::on_next(20, 42), RT::on_next(25, 99)]),
        scheduler.create_cold_observable(vec![RT::on_next(10, 43), RT::on_next(15, 99)]),
        scheduler.create_cold_observable(vec![RT::on_next(30, 44), RT::on_next(35, 99)]),
    ];

    let (source, delays) = (xs.as_observable(), ys.clone());
    let res = scheduler.start(move || source.delay_by(move |x: &i32| delays[*x as usize].as_observable()));

    assert_messages!(
        res.messages(),
        RT::on_next(250 + 20, 0),
        RT::on_next(350 + 10, 1),
        RT::on_next(450 + 30, 2),
        RT::on_completed::<i32>(550),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 550));

    assert_subscriptions!(ys[0].subscriptions(), RT::subscribe(250, 250 + 20));
    assert_subscriptions!(ys[1].subscriptions(), RT::subscribe(350, 350 + 10));
    assert_subscriptions!(ys[2].subscriptions(), RT::subscribe(450, 450 + 30));
}

#[test]
fn delay_duration_and_subscription_simple1() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 10),
        RT::on_next(220, 30),
        RT::on_next(230, 50),
        RT::on_next(240, 35),
        RT::on_next(250, 20),
        RT::on_completed(260),
    ]);

    let ys = scheduler.create_cold_observable(vec![RT::on_next(10, "!")]);

    let (source, subscription_delay, sch) = (xs.as_observable(), ys.as_observable(), scheduler.clone());
    let res = scheduler.start(move || {
        source.delay_by_with_subscription_delay(subscription_delay, move |x: &i32| {
            sch.create_cold_observable(vec![RT::on_next(i64::from(*x), "!")]).as_observable()
        })
    });

    assert_messages!(
        res.messages(),
        RT::on_next(220 + 30, 30),
        RT::on_next(250 + 20, 20),
        RT::on_next(240 + 35, 35),
        RT::on_next(230 + 50, 50),
        RT::on_completed::<i32>(280),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(210, 260));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn delay_duration_and_subscription_simple2() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 10),
        RT::on_next(220, 30),
        RT::on_next(230, 50),
        RT::on_next(240, 35),
        RT::on_next(250, 20),
        RT::on_completed(260),
    ]);

    let ys = scheduler.create_cold_observable(vec![RT::on_completed::<&str>(10)]);

    let (source, subscription_delay, sch) = (xs.as_observable(), ys.as_observable(), scheduler.clone());
    let res = scheduler.start(move || {
        source.delay_by_with_subscription_delay(subscription_delay, move |x: &i32| {
            sch.create_cold_observable(vec![RT::on_next(i64::from(*x), "!")]).as_observable()
        })
    });

    assert_messages!(
        res.messages(),
        RT::on_next(220 + 30, 30),
        RT::on_next(250 + 20, 20),
        RT::on_next(240 + 35, 35),
        RT::on_next(230 + 50, 50),
        RT::on_completed::<i32>(280),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(210, 260));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn delay_duration_and_subscription_dispose1() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 10),
        RT::on_next(220, 30),
        RT::on_next(230, 50),
        RT::on_next(240, 35),
        RT::on_next(250, 20),
        RT::on_completed(260),
    ]);

    let ys = scheduler.create_cold_observable(vec![RT::on_next(10, "!")]);

    let (source, subscription_delay, sch) = (xs.as_observable(), ys.as_observable(), scheduler.clone());
    let res = scheduler.start_disposed(
        move || {
            source.delay_by_with_subscription_delay(subscription_delay, move |x: &i32| {
                sch.create_cold_observable(vec![RT::on_next(i64::from(*x), "!")]).as_observable()
            })
        },
        255,
    );

    assert_messages!(res.messages(), RT::on_next(220 + 30, 30));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(210, 255));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn delay_duration_and_subscription_dispose2() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 10),
        RT::on_next(220, 30),
        RT::on_next(230, 50),
        RT::on_next(240, 35),
        RT::on_next(250, 20),
        RT::on_completed(260),
    ]);

    let ys = scheduler.create_cold_observable(vec![RT::on_next(100, "!")]);

    let (source, subscription_delay, sch) = (xs.as_observable(), ys.as_observable(), scheduler.clone());
    let res = scheduler.start_disposed(
        move || {
            source.delay_by_with_subscription_delay(subscription_delay, move |x: &i32| {
                sch.create_cold_observable(vec![RT::on_next(i64::from(*x), "!")]).as_observable()
            })
        },
        255,
    );

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions());

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 255));
}

#[test]
fn delay_duration_and_subscription_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 10),
        RT::on_next(220, 30),
        RT::on_next(230, 50),
        RT::on_next(240, 35),
        RT::on_next(250, 20),
        RT::on_completed(260),
    ]);

    let ys = scheduler.create_cold_observable(vec![RT::on_error::<&str>(10, ex.clone())]);

    let (source, subscription_delay, sch) = (xs.as_observable(), ys.as_observable(), scheduler.clone());
    let res = scheduler.start(move || {
        source.delay_by_with_subscription_delay(subscription_delay, move |x: &i32| {
            sch.create_cold_observable(vec![RT::on_next(i64::from(*x), "!")]).as_observable()
        })
    });

    assert_messages!(res.messages(), RT::on_error::<i32>(200 + 10, ex.clone()));

    assert_subscriptions!(xs.subscriptions());

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 210));
}

/// A scheduler that keeps the delayed work it receives and runs it when the test gives the impulse
/// (`ImpulseScheduler`, whose work waits on another thread for an event the test sets).
struct ImpulseScheduler {
    this: Weak<ImpulseScheduler>,
    pending: RefCell<Vec<ScheduledAction>>,
}

impl ImpulseScheduler {
    fn new() -> Rc<Self> {
        Rc::new_cyclic(|this| Self { this: this.clone(), pending: RefCell::new(Vec::new()) })
    }

    /// Runs the work kept so far (`Event.Set()`, then `Done.WaitOne()`); returns how much ran.
    fn impulse(&self) -> usize {
        let pending = std::mem::take(&mut *self.pending.borrow_mut());
        let this: Rc<dyn IScheduler> = self.this.upgrade().expect("the scheduler is alive");
        let count = pending.len();
        for action in pending {
            action(&this);
        }
        count
    }
}

impl IScheduler for ImpulseScheduler {
    fn now(&self) -> SystemTime {
        SystemTime::now()
    }

    fn schedule(&self, _action: ScheduledAction) -> Rc<dyn IDisposable> {
        panic!("The method or operation is not implemented.");
    }

    fn schedule_after(&self, _due_time: Duration, action: ScheduledAction) -> Rc<dyn IDisposable> {
        self.pending.borrow_mut().push(action);
        Disposable::empty()
    }

    fn schedule_at(&self, _due_time: SystemTime, _action: ScheduledAction) -> Rc<dyn IDisposable> {
        panic!("The method or operation is not implemented.");
    }
}

/// Checks the race between a value and an error, where the error is forwarded before the value
/// is due: the value is then not forwarded (the first `has_failed` check of the drain loop).
#[test]
fn delay_error_handling1() {
    let xs = Observable::create_with_action(|observer: Rc<dyn IObserver<i32>>| {
        observer.on_next(42);
        observer.on_error(new_error("ex"));
        || {}
    });

    let s = ImpulseScheduler::new();

    let called = Rc::new(Cell::new(false));
    let failed = Rc::new(Cell::new(false));
    {
        let (called, failed) = (called.clone(), failed.clone());
        let scheduler: Rc<dyn IScheduler> = s.clone();
        xs.delay_on(Duration::from_secs(24 * 60 * 60), scheduler).subscribe_next_error(move |_| called.set(true), move |_| failed.set(true));
    }

    assert!(failed.get());
    assert_eq!(1, s.impulse());

    assert!(!called.get());
}

#[test]
fn delay_selector_immediate() {
    let result = Rc::new(Cell::new(0));
    let source = Observable::return_(1);
    let delayed = source.delay_by(|_: &i32| Observable::return_(2));
    let target = result.clone();
    delayed.subscribe_next(move |v| target.set(v));

    assert_eq!(1, result.get());
}

// Not ported: Delay_ArgumentChecking - argument validation (and a negative TimeSpan, which a Duration cannot be)
// Not ported: Delay_TimeSpan_Real_Simple1 - real threads (ThreadPoolScheduler, Task.Run)
// Not ported: Delay_TimeSpan_Real_Simple1_Stopwatch - real threads (ThreadPoolScheduler, Task.Run)
// Not ported: Delay_TimeSpan_Real_Error1 - real threads (ThreadPoolScheduler, Task.Run)
// Not ported: Delay_TimeSpan_Real_Error1_Stopwatch - real threads (ThreadPoolScheduler, Task.Run)
// Not ported: Delay_TimeSpan_Real_Error2 - real threads (ThreadPoolScheduler, Task.Run)
// Not ported: Delay_TimeSpan_Real_Error2_Stopwatch - real threads (ThreadPoolScheduler, Task.Run)
// Not ported: Delay_TimeSpan_Real_Error3 - real threads (ThreadPoolScheduler, Task.Run)
// Not ported: Delay_TimeSpan_Real_Error3_Stopwatch - real threads (ThreadPoolScheduler, Task.Run)
// Not ported: Delay_TimeSpan_DefaultScheduler - needs operator ToEnumerable (the blocking operators, phase 3, later)
// Not ported: Delay_DateTimeOffset_DefaultScheduler - needs operator ToEnumerable (the blocking operators, phase 3, later)
// Not ported: Delay_Duration_ArgumentChecking - argument validation
// Not ported: Delay_LongRunning_CancelEarly - real threads (ISchedulerLongRunning, which the port does not have)
// Not ported: Delay_LongRunning_CancelLate - real threads (ISchedulerLongRunning, which the port does not have)
