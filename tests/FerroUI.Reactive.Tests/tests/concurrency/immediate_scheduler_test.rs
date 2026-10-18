//! Ported from `Tests.System.Reactive/Tests/Concurrency/ImmediateSchedulerTest.cs` of Rx.NET.
//! Adapted: a thrown exception is a panic with a marker payload, caught with `catch_unwind`; the
//! managed thread id is `std::thread::current().id()`; the state of `Schedule(state, ...)` is what
//! the scheduled closure captures; the delays the scheduler blocks for (0.2 s and 100 ms upstream)
//! are 10 ms, which keeps the tests deterministic and short.

use ferroui_reactive::concurrency::{IScheduler, Scheduler, SchedulerExt};
use ferroui_reactive::disposables::Disposable;
use std::cell::Cell;
use std::panic::{catch_unwind, panic_any, AssertUnwindSafe};
use std::rc::Rc;
use std::time::{Duration, SystemTime};

/// The exception thrown by the action of `immediate_schedule_action_error`.
#[derive(Debug, PartialEq)]
struct Ex(u32);

/// The short delay that stands for the delays of upstream.
const SHORT_DELAY: Duration = Duration::from_millis(10);

/// `TimeSpan.Seconds` of `a - b`: the seconds component of the difference, negative when `a` is
/// before `b`.
fn time_span_seconds(a: SystemTime, b: SystemTime) -> i64 {
    match a.duration_since(b) {
        Ok(d) => (d.as_secs() % 60) as i64,
        Err(e) => -((e.duration().as_secs() % 60) as i64),
    }
}

#[test]
fn immediate_now() {
    let res = time_span_seconds(Scheduler::immediate().now(), SystemTime::now());
    assert!(res < 1);
}

#[test]
fn immediate_schedule_action() {
    let id = std::thread::current().id();
    let ran = Rc::new(Cell::new(false));
    let r = ran.clone();
    Scheduler::immediate().schedule_action(move || {
        assert_eq!(id, std::thread::current().id());
        r.set(true);
    });
    assert!(ran.get());
}

#[test]
fn immediate_schedule_action_error() {
    let result = catch_unwind(AssertUnwindSafe(|| {
        Scheduler::immediate().schedule_action(|| panic_any(Ex(42)));
    }));
    match result {
        Ok(()) => panic!("the action should have thrown"),
        Err(e) => assert_eq!(Some(&Ex(42)), e.downcast_ref::<Ex>()),
    }
}

#[test]
fn immediate_simple1() {
    let x = Rc::new(Cell::new(0));
    let xx = x.clone();
    let state = 42;
    Scheduler::immediate().schedule(Box::new(move |_this| {
        xx.set(state);
        Disposable::empty()
    }));
    assert_eq!(42, x.get());
}

#[test]
fn immediate_simple2() {
    let x = Rc::new(Cell::new(0));
    let xx = x.clone();
    let state = 42;
    Scheduler::immediate().schedule_at(
        SystemTime::now(),
        Box::new(move |_this| {
            xx.set(state);
            Disposable::empty()
        }),
    );
    assert_eq!(42, x.get());
}

#[test]
fn immediate_simple3() {
    let x = Rc::new(Cell::new(0));
    let xx = x.clone();
    let state = 42;
    Scheduler::immediate().schedule_after(
        Duration::ZERO,
        Box::new(move |_this| {
            xx.set(state);
            Disposable::empty()
        }),
    );
    assert_eq!(42, x.get());
}

#[test]
fn immediate_recursive1() {
    let x = Rc::new(Cell::new(0));
    let y = Rc::new(Cell::new(0));
    let (xx, yy) = (x.clone(), y.clone());
    Scheduler::immediate().schedule(Box::new(move |this| {
        xx.set(42);
        this.schedule(Box::new(move |_this2| {
            yy.set(43);
            Disposable::empty()
        }))
    }));
    assert_eq!(42, x.get());
    assert_eq!(43, y.get());
}

#[test]
fn immediate_recursive2() {
    let x = Rc::new(Cell::new(0));
    let y = Rc::new(Cell::new(0));
    let (xx, yy) = (x.clone(), y.clone());
    Scheduler::immediate().schedule(Box::new(move |this| {
        xx.set(42);
        this.schedule_at(
            SystemTime::now(),
            Box::new(move |_this2| {
                yy.set(43);
                Disposable::empty()
            }),
        )
    }));
    assert_eq!(42, x.get());
    assert_eq!(43, y.get());
}

#[test]
fn immediate_recursive3() {
    let x = Rc::new(Cell::new(0));
    let y = Rc::new(Cell::new(0));
    let (xx, yy) = (x.clone(), y.clone());
    Scheduler::immediate().schedule(Box::new(move |this| {
        xx.set(42);
        this.schedule_after(
            SHORT_DELAY,
            Box::new(move |_this2| {
                yy.set(43);
                Disposable::empty()
            }),
        )
    }));
    assert_eq!(42, x.get());
    assert_eq!(43, y.get());
}

#[test]
fn immediate_schedule_action_due() {
    let id = std::thread::current().id();
    let ran = Rc::new(Cell::new(false));
    let r = ran.clone();
    Scheduler::immediate().schedule_action_after(SHORT_DELAY, move || {
        assert_eq!(id, std::thread::current().id());
        r.set(true);
    });
    assert!(ran.get(), "ran");
}

// Not ported: Immediate_ArgumentChecking - argument validation
// Not ported: Immediate_ArgumentChecking_More - argument validation
