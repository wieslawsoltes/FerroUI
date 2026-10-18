//! Ported from `Tests.System.Reactive/Tests/Concurrency/TestSchedulerTest.cs` of Rx.NET.
//! Adapted: the null checks of `Start` (`ArgumentNullException`) cannot be written in Rust and are
//! left out; the out-of-range times panic where upstream throws `ArgumentOutOfRangeException`.

use ferroui_reactive::Observable;
use ferroui_reactive_testing::{assert_throws, TestScheduler};

#[test]
fn test_argument_checking() {
    assert_throws(|| {
        TestScheduler::new().start_disposed(|| Observable::empty::<i32>(), 10);
    });
    assert_throws(|| {
        TestScheduler::new().start_with(|| Observable::empty::<i32>(), 10, 15, 5);
    });
}
