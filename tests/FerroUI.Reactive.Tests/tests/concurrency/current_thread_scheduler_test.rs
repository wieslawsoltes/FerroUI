//! Ported from `Tests.System.Reactive/Tests/Concurrency/CurrentThreadSchedulerTest.cs` of Rx.NET.
//! Adapted: a thrown exception is a panic with a marker payload, caught with `catch_unwind`; the
//! managed thread id is `std::thread::current().id()`; `EnsureTrampoline`, an extension method of
//! the upstream test project (`Extensions.cs`), is the local function `ensure_trampoline`; the
//! delays the trampoline waits for (1 s and 0.2 s upstream) are 10 ms, which keeps the tests
//! deterministic (the scheduler blocks the thread for the delay) and short.

use ferroui_reactive::concurrency::{CurrentThreadScheduler, IScheduler, Scheduler, SchedulerExt};
use ferroui_reactive::IDisposable;
use std::cell::Cell;
use std::panic::{catch_unwind, panic_any, AssertUnwindSafe};
use std::rc::Rc;
use std::time::{Duration, SystemTime};

/// The exception thrown by the action of `current_thread_schedule_action_error`.
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

/// Runs `action` on the trampoline of the current-thread scheduler: scheduled when no work of the
/// scheduler runs on the thread, else at once (`EnsureTrampoline` of the upstream test project).
fn ensure_trampoline(action: impl FnOnce() + 'static) {
    if CurrentThreadScheduler::is_schedule_required() {
        Scheduler::current_thread().schedule_action(action);
    } else {
        action();
    }
}

#[test]
fn current_thread_now() {
    let res = time_span_seconds(Scheduler::current_thread().now(), SystemTime::now());
    assert!(res < 1);
}

#[test]
fn current_thread_schedule_action() {
    let id = std::thread::current().id();
    let ran = Rc::new(Cell::new(false));
    let r = ran.clone();
    Scheduler::current_thread().schedule_action(move || {
        assert_eq!(id, std::thread::current().id());
        r.set(true);
    });
    assert!(ran.get());
}

#[test]
fn current_thread_schedule_action_error() {
    let result = catch_unwind(AssertUnwindSafe(|| {
        Scheduler::current_thread().schedule_action(|| panic_any(Ex(42)));
    }));
    match result {
        Ok(()) => panic!("the action should have thrown"),
        Err(e) => assert_eq!(Some(&Ex(42)), e.downcast_ref::<Ex>()),
    }
}

#[test]
fn current_thread_schedule_action_nested() {
    let id = std::thread::current().id();
    let ran = Rc::new(Cell::new(false));
    let r = ran.clone();
    Scheduler::current_thread().schedule_action(move || {
        assert_eq!(id, std::thread::current().id());
        Scheduler::current_thread().schedule_action(move || r.set(true));
    });
    assert!(ran.get());
}

#[test]
fn current_thread_schedule_action_nested_time_span() {
    let id = std::thread::current().id();
    let ran = Rc::new(Cell::new(false));
    let r = ran.clone();
    Scheduler::current_thread().schedule_action(move || {
        assert_eq!(id, std::thread::current().id());
        Scheduler::current_thread().schedule_action_after(SHORT_DELAY, move || r.set(true));
    });
    assert!(ran.get());
}

#[test]
fn current_thread_schedule_action_due() {
    let id = std::thread::current().id();
    let ran = Rc::new(Cell::new(false));
    let r = ran.clone();
    Scheduler::current_thread().schedule_action_after(SHORT_DELAY, move || {
        assert_eq!(id, std::thread::current().id());
        r.set(true);
    });
    assert!(ran.get(), "ran");
}

#[test]
fn current_thread_schedule_action_due_nested() {
    let id = std::thread::current().id();
    let ran = Rc::new(Cell::new(false));
    let r = ran.clone();
    Scheduler::current_thread().schedule_action_after(SHORT_DELAY, move || {
        assert_eq!(id, std::thread::current().id());

        Scheduler::current_thread().schedule_action_after(SHORT_DELAY, move || {
            assert_eq!(id, std::thread::current().id());
            r.set(true);
        });
    });
    assert!(ran.get(), "ran");
}

#[test]
fn current_thread_ensure_trampoline() {
    let ran1 = Rc::new(Cell::new(false));
    let ran2 = Rc::new(Cell::new(false));
    let (r1, r2) = (ran1.clone(), ran2.clone());
    ensure_trampoline(move || {
        Scheduler::current_thread().schedule_action(move || r1.set(true));
        Scheduler::current_thread().schedule_action(move || r2.set(true));
    });
    assert!(ran1.get());
    assert!(ran2.get());
}

#[test]
fn current_thread_ensure_trampoline_nested() {
    let ran1 = Rc::new(Cell::new(false));
    let ran2 = Rc::new(Cell::new(false));
    let (r1, r2) = (ran1.clone(), ran2.clone());
    ensure_trampoline(move || {
        ensure_trampoline(move || r1.set(true));
        ensure_trampoline(move || r2.set(true));
    });
    assert!(ran1.get());
    assert!(ran2.get());
}

#[test]
fn current_thread_ensure_trampoline_and_cancel() {
    let ran1 = Rc::new(Cell::new(false));
    let ran2 = Rc::new(Cell::new(false));
    let (r1, r2) = (ran1.clone(), ran2.clone());
    ensure_trampoline(move || {
        Scheduler::current_thread().schedule_action(move || {
            r1.set(true);
            let d = Scheduler::current_thread().schedule_action(move || r2.set(true));
            d.dispose();
        });
    });
    assert!(ran1.get());
    assert!(!ran2.get());
}

#[test]
fn current_thread_ensure_trampoline_and_cancel_timed() {
    let ran1 = Rc::new(Cell::new(false));
    let ran2 = Rc::new(Cell::new(false));
    let (r1, r2) = (ran1.clone(), ran2.clone());
    ensure_trampoline(move || {
        Scheduler::current_thread().schedule_action(move || {
            r1.set(true);
            let d = Scheduler::current_thread().schedule_action_after(Duration::from_secs(1), move || r2.set(true));
            d.dispose();
        });
    });
    assert!(ran1.get());
    assert!(!ran2.get());
}

// Not ported: CurrentThread_ArgumentChecking - argument validation
