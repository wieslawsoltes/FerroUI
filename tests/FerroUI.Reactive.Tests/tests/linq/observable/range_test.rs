//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/RangeTest.cs` of Rx.NET.
//! Adapted: the count is a `usize`, so a negative count cannot be written; a count whose last value
//! is beyond `i32::MAX` panics where upstream throws `ArgumentOutOfRangeException`, checked with
//! `assert_throws`.

use ferroui_reactive::Observable;
use ferroui_reactive_testing::{assert_messages, assert_throws, ReactiveTest as RT, TestScheduler};

#[test]
fn range_argument_checking() {
    let scheduler = TestScheduler::new();
    assert_throws(|| {
        Observable::range_on(i32::MAX, 2, scheduler.scheduler());
    });
}

#[test]
fn range_zero() {
    let scheduler = TestScheduler::new();

    let s = scheduler.scheduler();
    let res = scheduler.start(move || Observable::range_on(0, 0, s));

    assert_messages!(res.messages(), RT::on_completed::<i32>(201));
}

#[test]
fn range_one() {
    let scheduler = TestScheduler::new();

    let s = scheduler.scheduler();
    let res = scheduler.start(move || Observable::range_on(0, 1, s));

    assert_messages!(res.messages(), RT::on_next(201, 0), RT::on_completed::<i32>(202));
}

#[test]
fn range_five() {
    let scheduler = TestScheduler::new();

    let s = scheduler.scheduler();
    let res = scheduler.start(move || Observable::range_on(10, 5, s));

    assert_messages!(
        res.messages(),
        RT::on_next(201, 10),
        RT::on_next(202, 11),
        RT::on_next(203, 12),
        RT::on_next(204, 13),
        RT::on_next(205, 14),
        RT::on_completed::<i32>(206)
    );
}

#[test]
fn range_boundaries() {
    let scheduler = TestScheduler::new();

    let s = scheduler.scheduler();
    let res = scheduler.start(move || Observable::range_on(i32::MAX, 1, s));

    assert_messages!(res.messages(), RT::on_next(201, i32::MAX), RT::on_completed::<i32>(202));
}

#[test]
fn range_dispose() {
    let scheduler = TestScheduler::new();

    let s = scheduler.scheduler();
    let res = scheduler.start_disposed(move || Observable::range_on(-10, 5, s), 204);

    assert_messages!(res.messages(), RT::on_next(201, -10), RT::on_next(202, -9), RT::on_next(203, -8));
}

#[test]
fn range_default_argument_checking() {
    assert_throws(|| {
        Observable::range(i32::MAX, 2);
    });
}

// Not ported (in part): Range_ArgumentChecking - the null scheduler and the negative count are argument validation; the count beyond `i32::MAX` is ported
// Not ported (in part): Range_Default_ArgumentChecking - the negative count is argument validation (`usize`); the count beyond `i32::MAX` is ported
// Not ported: Range_Default - real threads (compares with the sequence on DefaultScheduler.Instance, through the blocking ToEnumerable of AssertEqual)
// Not ported: Range_LongRunning1 - real threads (TestLongRunningScheduler; the port has no ISchedulerLongRunning)
// Not ported: Range_LongRunning2 - real threads (TestLongRunningScheduler; the port has no ISchedulerLongRunning)
// Not ported: Range_LongRunning_Empty - needs ISchedulerLongRunning (not in the port); blocking operator ForEach (waived: one thread)
// Not ported: Range_LongRunning_Regular - needs ISchedulerLongRunning (not in the port); blocking operator ForEach (waived: one thread)
// Not ported: Range_LongRunning_Boundaries - needs ISchedulerLongRunning (not in the port); blocking operator ForEach (waived: one thread)
