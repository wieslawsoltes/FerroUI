//! Ported from `Tests.System.Reactive/Tests/Concurrency/VirtualSchedulerTest.cs` of Rx.NET.
//! Adapted: the test subclass `VirtualSchedulerTestScheduler : VirtualTimeScheduler<string, char>`
//! is the clock type `StringClock` of a `VirtualTimeScheduler<StringClock>`; its absolute time is a
//! `&'static str` because the absolute time of a clock must be `Copy` (each `Add` leaks the short
//! string it makes); the null initial clock of the parameterless constructor is the empty string;
//! the comparer argument of the constructor does not exist (strings compare by their order); a
//! `char` cannot be a lone UTF-16 surrogate, so `ToRelative` maps one to U+FFFD; `ToDateTimeOffset`
//! counts its ticks from the epoch of the system time; a thrown exception is a panic with a marker
//! payload, caught with `catch_unwind`; the managed thread id is `std::thread::current().id()`.

use ferroui_reactive::concurrency::{IScheduler, SchedulerExt, VirtualTimeClock, VirtualTimeScheduler};
use ferroui_reactive_testing::{duration_to_ticks, ticks_to_duration};
use std::cell::Cell;
use std::panic::{catch_unwind, panic_any, AssertUnwindSafe};
use std::rc::Rc;
use std::time::{Duration, SystemTime};

/// The exception thrown by the action of `virtual_schedule_action_error`.
#[derive(Debug, PartialEq)]
struct Ex(u32);

/// The time of `VirtualSchedulerTestScheduler`: strings, moved by appending a character.
struct StringClock;

impl VirtualTimeClock for StringClock {
    type Absolute = &'static str;
    type Relative = char;

    fn add(absolute: &'static str, relative: char) -> &'static str {
        let mut s = absolute.to_owned();
        s.push(relative);
        Box::leak(s.into_boxed_str())
    }

    fn to_system_time(absolute: &'static str) -> SystemTime {
        SystemTime::UNIX_EPOCH + ticks_to_duration(absolute.encode_utf16().count() as i64)
    }

    fn to_relative(duration: Duration) -> char {
        char::from_u32((duration_to_ticks(duration) % 65535) as u32).unwrap_or('\u{FFFD}')
    }
}

type VirtualSchedulerTestScheduler = VirtualTimeScheduler<StringClock>;

/// `TimeSpan.Seconds` of `a - b`: the seconds component of the difference, negative when `a` is
/// before `b`.
fn time_span_seconds(a: SystemTime, b: SystemTime) -> i64 {
    match a.duration_since(b) {
        Ok(d) => (d.as_secs() % 60) as i64,
        Err(e) => -((e.duration().as_secs() % 60) as i64),
    }
}

#[test]
fn virtual_now() {
    let res = time_span_seconds(VirtualSchedulerTestScheduler::new("").now(), SystemTime::now());
    assert!(res < 1);
}

#[test]
fn virtual_schedule_action() {
    let id = std::thread::current().id();
    let ran = Rc::new(Cell::new(false));
    let scheduler = VirtualSchedulerTestScheduler::new("");
    let s: Rc<dyn IScheduler> = scheduler.clone();
    let r = ran.clone();
    s.schedule_action(move || {
        assert_eq!(id, std::thread::current().id());
        r.set(true);
    });
    scheduler.start();
    assert!(ran.get());
}

#[test]
fn virtual_schedule_action_error() {
    let result = catch_unwind(AssertUnwindSafe(|| {
        let scheduler = VirtualSchedulerTestScheduler::new("");
        let s: Rc<dyn IScheduler> = scheduler.clone();
        s.schedule_action(|| panic_any(Ex(42)));
        scheduler.start();
    }));
    match result {
        Ok(()) => panic!("the action should have thrown"),
        Err(e) => assert_eq!(Some(&Ex(42)), e.downcast_ref::<Ex>()),
    }
}

#[test]
fn virtual_initial_and_comparer_now() {
    let s = VirtualSchedulerTestScheduler::new("Bar");
    assert_eq!(3, duration_to_ticks(s.now().duration_since(SystemTime::UNIX_EPOCH).unwrap()));
}

#[test]
fn virtual_schedule_action_due() {
    let id = std::thread::current().id();
    let ran = Rc::new(Cell::new(false));
    let scheduler = VirtualSchedulerTestScheduler::new("");
    let s: Rc<dyn IScheduler> = scheduler.clone();
    let r = ran.clone();
    s.schedule_action_after(Duration::from_millis(200), move || {
        assert_eq!(id, std::thread::current().id());
        r.set(true);
    });
    scheduler.start();
    assert!(ran.get(), "ran");
}

// Not ported: Virtual_ArgumentChecking - argument validation
// Not ported: Historical_ArgumentChecking - argument validation
// Not ported: Virtual_ThreadSafety - real threads (Task.Run racing AdvanceBy); also needs operator Timeout (phase 3)
