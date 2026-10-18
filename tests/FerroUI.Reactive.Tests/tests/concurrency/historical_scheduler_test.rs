//! Ported from `Tests.System.Reactive/Tests/Concurrency/HistoricalSchedulerTest.cs` of Rx.NET.
//! Adapted: `DateTimeOffset` is `SystemTime` and `TimeSpan` is `Duration`; a new historical
//! scheduler starts at the epoch of the system time (`HistoricalScheduler::historical()`) where
//! upstream starts at `DateTimeOffset.MinValue`, so `Ctor` expects the epoch; the dates the tests
//! construct are the same instants written as `SystemTime::UNIX_EPOCH` plus an offset; the
//! out-of-range and nested advances panic where upstream throws; a negative `TimeSpan` cannot be
//! written as a `Duration`, so the checks that pass one are left out.

use ferroui_reactive::concurrency::{HistoricalScheduler, IScheduler, SchedulerExt};
use ferroui_reactive::{IDisposable, Timestamped};
use ferroui_reactive_testing::assert_throws;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, SystemTime};

const DAY: Duration = Duration::from_secs(24 * 60 * 60);

/// `new DateTimeOffset(1979, 10, 31, 4, 30, 15, TimeSpan.Zero).AddDays(i)`.
fn time(i: u64) -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(310_192_215) + DAY * i as u32
}

type List = Rc<RefCell<Vec<Timestamped<i32>>>>;

fn new_list() -> List {
    Rc::new(RefCell::new(Vec::new()))
}

/// `s.Schedule(due, () => list.Add(new Timestamped<int>(value, s.Now)))`.
fn schedule_add(s: &Rc<HistoricalScheduler>, due: SystemTime, list: &List, value: i32) -> Rc<dyn IDisposable> {
    let scheduler: Rc<dyn IScheduler> = s.clone();
    let (s, list) = (s.clone(), list.clone());
    scheduler.schedule_action_at(due, move || list.borrow_mut().push(Timestamped::new(value, s.now())))
}

/// `s.Schedule(due, action)`.
fn schedule_at(s: &Rc<HistoricalScheduler>, due: SystemTime, action: impl FnOnce() + 'static) -> Rc<dyn IDisposable> {
    let scheduler: Rc<dyn IScheduler> = s.clone();
    scheduler.schedule_action_at(due, action)
}

fn ts(value: i32, timestamp: SystemTime) -> Timestamped<i32> {
    Timestamped::new(value, timestamp)
}

#[test]
fn ctor() {
    let s = HistoricalScheduler::historical();

    assert_eq!(SystemTime::UNIX_EPOCH, s.clock());
    assert_eq!(SystemTime::UNIX_EPOCH, s.now());
    assert!(!s.is_enabled());
}

#[test]
fn start_stop() {
    let s = HistoricalScheduler::historical();

    let list = new_list();

    schedule_add(&s, time(0), &list, 1);
    schedule_add(&s, time(1), &list, 2);
    let s2 = s.clone();
    schedule_at(&s, time(2), move || s2.stop());
    schedule_add(&s, time(3), &list, 3);
    let s4 = s.clone();
    schedule_at(&s, time(4), move || s4.stop());
    let s5 = s.clone();
    schedule_at(&s, time(5), move || s5.start());
    schedule_add(&s, time(6), &list, 4);

    s.start();

    assert_eq!(time(2), s.now());
    assert_eq!(time(2), s.clock());

    s.start();

    assert_eq!(time(4), s.now());
    assert_eq!(time(4), s.clock());

    s.start();

    assert_eq!(time(6), s.now());
    assert_eq!(time(6), s.clock());

    s.start();

    assert_eq!(time(6), s.now());
    assert_eq!(time(6), s.clock());

    assert_eq!(*list.borrow(), vec![ts(1, time(0)), ts(2, time(1)), ts(3, time(3)), ts(4, time(6))]);
}

#[test]
fn order() {
    let s = HistoricalScheduler::historical();

    let list = new_list();

    schedule_add(&s, time(2), &list, 2);

    schedule_add(&s, time(3), &list, 3);

    schedule_add(&s, time(1), &list, 0);
    schedule_add(&s, time(1), &list, 1);

    s.start();

    assert_eq!(*list.borrow(), vec![ts(0, time(1)), ts(1, time(1)), ts(2, time(2)), ts(3, time(3))]);
}

#[test]
fn cancellation() {
    let s = HistoricalScheduler::historical();

    let list = new_list();

    let d = schedule_add(&s, time(2), &list, 2);

    let (s1, l1) = (s.clone(), list.clone());
    schedule_at(&s, time(1), move || {
        l1.borrow_mut().push(ts(0, s1.now()));
        d.dispose();
    });

    s.start();

    assert_eq!(*list.borrow(), vec![ts(0, time(1))]);
}

#[test]
fn advance_to_argument_checking() {
    let now = SystemTime::now();

    let s = HistoricalScheduler::historical_at(now);

    assert_throws(|| s.advance_to(now - Duration::from_secs(1)));
}

#[test]
fn advance_to() {
    let s = HistoricalScheduler::historical();

    let list = new_list();

    schedule_add(&s, time(0), &list, 0);
    schedule_add(&s, time(1), &list, 1);
    schedule_add(&s, time(2), &list, 2);
    schedule_add(&s, time(10), &list, 10);
    schedule_add(&s, time(11), &list, 11);

    s.advance_to(time(8));

    assert_eq!(time(8), s.now());
    assert_eq!(time(8), s.clock());

    assert_eq!(*list.borrow(), vec![ts(0, time(0)), ts(1, time(1)), ts(2, time(2))]);

    s.advance_to(time(8));

    assert_eq!(time(8), s.now());
    assert_eq!(time(8), s.clock());

    assert_eq!(*list.borrow(), vec![ts(0, time(0)), ts(1, time(1)), ts(2, time(2))]);

    schedule_add(&s, time(7), &list, 7);
    schedule_add(&s, time(8), &list, 8);

    assert_throws(|| s.advance_to(time(4)));

    assert_eq!(time(8), s.now());
    assert_eq!(time(8), s.clock());

    assert_eq!(*list.borrow(), vec![ts(0, time(0)), ts(1, time(1)), ts(2, time(2))]);

    s.advance_to(time(10));

    assert_eq!(time(10), s.now());
    assert_eq!(time(10), s.clock());

    assert_eq!(
        *list.borrow(),
        vec![ts(0, time(0)), ts(1, time(1)), ts(2, time(2)), ts(7, time(8)), ts(8, time(8)), ts(10, time(10))]
    );

    s.advance_to(time(100));

    assert_eq!(time(100), s.now());
    assert_eq!(time(100), s.clock());

    assert_eq!(
        *list.borrow(),
        vec![
            ts(0, time(0)),
            ts(1, time(1)),
            ts(2, time(2)),
            ts(7, time(8)),
            ts(8, time(8)),
            ts(10, time(10)),
            ts(11, time(11)),
        ]
    );
}

#[test]
fn advance_by() {
    let s = HistoricalScheduler::historical();

    let list = new_list();

    schedule_add(&s, time(0), &list, 0);
    schedule_add(&s, time(1), &list, 1);
    schedule_add(&s, time(2), &list, 2);
    schedule_add(&s, time(10), &list, 10);
    schedule_add(&s, time(11), &list, 11);

    s.advance_by(time(8).duration_since(s.now()).unwrap());

    assert_eq!(time(8), s.now());
    assert_eq!(time(8), s.clock());

    assert_eq!(*list.borrow(), vec![ts(0, time(0)), ts(1, time(1)), ts(2, time(2))]);

    schedule_add(&s, time(7), &list, 7);
    schedule_add(&s, time(8), &list, 8);

    // Upstream: `ReactiveAssert.Throws<ArgumentOutOfRangeException>(() => s.AdvanceBy(TimeSpan.FromDays(-4)))`;
    // a negative duration cannot be written.

    assert_eq!(time(8), s.now());
    assert_eq!(time(8), s.clock());

    assert_eq!(*list.borrow(), vec![ts(0, time(0)), ts(1, time(1)), ts(2, time(2))]);

    s.advance_by(Duration::ZERO);

    assert_eq!(time(8), s.now());
    assert_eq!(time(8), s.clock());

    assert_eq!(*list.borrow(), vec![ts(0, time(0)), ts(1, time(1)), ts(2, time(2))]);

    s.advance_by(DAY * 2);

    assert_eq!(time(10), s.now());
    assert_eq!(time(10), s.clock());

    assert_eq!(
        *list.borrow(),
        vec![ts(0, time(0)), ts(1, time(1)), ts(2, time(2)), ts(7, time(8)), ts(8, time(8)), ts(10, time(10))]
    );

    s.advance_by(DAY * 90);

    assert_eq!(time(100), s.now());
    assert_eq!(time(100), s.clock());

    assert_eq!(
        *list.borrow(),
        vec![
            ts(0, time(0)),
            ts(1, time(1)),
            ts(2, time(2)),
            ts(7, time(8)),
            ts(8, time(8)),
            ts(10, time(10)),
            ts(11, time(11)),
        ]
    );
}

#[test]
fn is_enabled() {
    let s = HistoricalScheduler::historical();

    assert!(!s.is_enabled());

    let scheduler: Rc<dyn IScheduler> = s.clone();
    let s1 = s.clone();
    scheduler.schedule_action(move || {
        assert!(s1.is_enabled());
        s1.stop();
        assert!(!s1.is_enabled());
    });

    assert!(!s.is_enabled());

    s.start();

    assert!(!s.is_enabled());
}

#[test]
fn no_nested_advance_by() {
    let s = HistoricalScheduler::historical();

    let scheduler: Rc<dyn IScheduler> = s.clone();
    let s1 = s.clone();
    scheduler.schedule_action(move || s1.advance_by(Duration::from_secs(1)));

    assert_throws(|| s.start());
}

#[test]
fn no_nested_advance_to() {
    let s = HistoricalScheduler::historical();

    let scheduler: Rc<dyn IScheduler> = s.clone();
    let s1 = s.clone();
    scheduler.schedule_action(move || s1.advance_to(SystemTime::now() + DAY));

    assert_throws(|| s.start());
}

#[test]
fn sleep1() {
    // new DateTimeOffset(1983, 2, 11, 12, 0, 0, TimeSpan.Zero)
    let now = SystemTime::UNIX_EPOCH + Duration::from_secs(413_812_800);

    let s = HistoricalScheduler::historical_at(now);

    s.sleep(DAY);

    assert_eq!(now + DAY, s.clock());
}

#[test]
fn sleep2() {
    let s = HistoricalScheduler::historical();

    let n = Rc::new(Cell::new(0));

    let scheduler: Rc<dyn IScheduler> = s.clone();
    let (s1, n1) = (s.clone(), n.clone());
    scheduler.schedule_recursive_at(s.now() + Duration::from_secs(60), move |rec| {
        s1.sleep(Duration::from_secs(3 * 60));
        n1.set(n1.get() + 1);

        rec(s1.now() + Duration::from_secs(60));
    });

    s.advance_to(s.now() + Duration::from_secs(5 * 60));

    assert_eq!(2, n.get());
}

// Not ported: AdvanceBy_ArgumentChecking - argument validation (a negative TimeSpan cannot be written as a Duration)
// Not ported: Sleep_ArgumentChecking - argument validation (a negative TimeSpan cannot be written as a Duration)
// Not ported: WithComparer_ArgumentChecking - argument validation
// Not ported: WithComparer - the constructor with a comparer of the due times (HistoricalScheduler(DateTimeOffset, IComparer<DateTimeOffset>)) does not exist
