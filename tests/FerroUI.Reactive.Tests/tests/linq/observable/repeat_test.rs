//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/RepeatTest.cs` of Rx.NET: the tests of
//! the factories that repeat a value (`Repeat<TResult>(TResult, ...)`, `Observable::repeat`,
//! `repeat_on`, `repeat_count`, `repeat_count_on`).
//! Adapted: the count is a `usize`, so a negative count cannot be written.

use ferroui_reactive::Observable;
use ferroui_reactive_testing::{assert_messages, ReactiveTest as RT, TestScheduler};

#[test]
fn repeat_value_count_zero() {
    let scheduler = TestScheduler::new();

    let s = scheduler.scheduler();
    let res = scheduler.start(move || Observable::repeat_count_on(42, 0, s));

    assert_messages!(res.messages(), RT::on_completed::<i32>(201));
}

#[test]
fn repeat_value_count_one() {
    let scheduler = TestScheduler::new();

    let s = scheduler.scheduler();
    let res = scheduler.start(move || Observable::repeat_count_on(42, 1, s));

    assert_messages!(res.messages(), RT::on_next(201, 42), RT::on_completed::<i32>(201));
}

#[test]
fn repeat_value_count_ten() {
    let scheduler = TestScheduler::new();

    let s = scheduler.scheduler();
    let res = scheduler.start(move || Observable::repeat_count_on(42, 10, s));

    assert_messages!(
        res.messages(),
        RT::on_next(201, 42),
        RT::on_next(202, 42),
        RT::on_next(203, 42),
        RT::on_next(204, 42),
        RT::on_next(205, 42),
        RT::on_next(206, 42),
        RT::on_next(207, 42),
        RT::on_next(208, 42),
        RT::on_next(209, 42),
        RT::on_next(210, 42),
        RT::on_completed::<i32>(210)
    );
}

#[test]
fn repeat_value_count_dispose() {
    let scheduler = TestScheduler::new();

    let s = scheduler.scheduler();
    let res = scheduler.start_disposed(move || Observable::repeat_count_on(42, 10, s), 207);

    assert_messages!(
        res.messages(),
        RT::on_next(201, 42),
        RT::on_next(202, 42),
        RT::on_next(203, 42),
        RT::on_next(204, 42),
        RT::on_next(205, 42),
        RT::on_next(206, 42)
    );
}

#[test]
fn repeat_value() {
    let scheduler = TestScheduler::new();

    let s = scheduler.scheduler();
    let res = scheduler.start_disposed(move || Observable::repeat_on(42, s), 207);

    assert_messages!(
        res.messages(),
        RT::on_next(201, 42),
        RT::on_next(202, 42),
        RT::on_next(203, 42),
        RT::on_next(204, 42),
        RT::on_next(205, 42),
        RT::on_next(206, 42)
    );
}

// Not ported: Repeat_Value_Count_ArgumentChecking - argument validation (a null scheduler, a negative count: the count is a `usize`, a null observer)
// Not ported: Repeat_Value_Count_Default_ArgumentChecking - argument validation (a negative count: the count is a `usize`, a null observer)
// Not ported: Repeat_Value_Count_Default - real threads (compares with the sequence on DefaultScheduler.Instance, through the blocking ToEnumerable of AssertEqual)
// Not ported: Repeat_Value_ArgumentChecking - argument validation
// Not ported: Repeat_Value_Default_ArgumentChecking - argument validation
// Not ported: Repeat_Value_Default - real threads (DefaultScheduler.Instance, the blocking ToEnumerable of AssertEqual) and needs operator Take (phase 3, later)
// Not ported: Repeat_Count_LongRunning1 - real threads (TestLongRunningScheduler; the port has no ISchedulerLongRunning)
// Not ported: Repeat_Count_LongRunning2 - real threads (TestLongRunningScheduler; the port has no ISchedulerLongRunning)
// Not ported: Repeat_Inf_LongRunning - real threads (TestLongRunningScheduler; the port has no ISchedulerLongRunning)
// Not ported: Repeat_Observable_ArgumentChecking - needs operator Repeat of a source (phase 3, later)
// Not ported: Repeat_Observable_Basic - needs operator Repeat of a source (phase 3, later)
// Not ported: Repeat_Observable_Infinite - needs operator Repeat of a source (phase 3, later)
// Not ported: Repeat_Observable_Error - needs operator Repeat of a source (phase 3, later)
// Not ported: Repeat_Observable_Throws - needs operator Repeat of a source (phase 3, later)
// Not ported: Repeat_Observable_Default_ArgumentChecking - needs operator Repeat of a source (phase 3, later)
// Not ported: Repeat_Observable_RepeatCount_ArgumentChecking - needs operator Repeat of a source (phase 3, later)
// Not ported: Repeat_Observable_RepeatCount_Basic - needs operator Repeat of a source (phase 3, later)
// Not ported: Repeat_Observable_RepeatCount_Dispose - needs operator Repeat of a source (phase 3, later)
// Not ported: Repeat_Observable_RepeatCount_Infinite - needs operator Repeat of a source (phase 3, later)
// Not ported: Repeat_Observable_RepeatCount_Error - needs operator Repeat of a source (phase 3, later)
// Not ported: Repeat_Observable_RepeatCount_Throws - needs operator Repeat of a source (phase 3, later)
// Not ported: Repeat_Observable_RepeatCount_Default_ArgumentChecking - needs operator Repeat of a source (phase 3, later)
