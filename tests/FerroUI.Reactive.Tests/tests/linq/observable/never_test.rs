//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/NeverTest.cs` of Rx.NET.

use ferroui_reactive::{IObservable, Observable};
use ferroui_reactive_testing::{assert_messages, TestScheduler};

#[test]
fn never_basic() {
    let scheduler = TestScheduler::new();

    let xs = Observable::never::<i32>();

    let res = scheduler.create_observer::<i32>();

    xs.subscribe(res.clone());

    scheduler.run();

    assert_messages!(res.messages());
}

#[test]
fn never_basic_witness() {
    let scheduler = TestScheduler::new();

    let xs = Observable::never_with_witness(42);

    let res = scheduler.create_observer::<i32>();

    xs.subscribe(res.clone());

    scheduler.run();

    assert_messages!(res.messages());
}

// Not ported: Never_ArgumentChecking - argument validation
