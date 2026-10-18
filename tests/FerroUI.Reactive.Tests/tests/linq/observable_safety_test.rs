//! Ported from `Tests.System.Reactive/Tests/Linq/ObservableSafetyTest.cs` of Rx.NET.
//! Adapted: an observer that throws is an observer that panics, and the exception that reaches the
//! caller of `scheduler.Start()` is the panic, caught with `catch_unwind` (which cannot check that
//! it carries the exception of the observer, as `Assert.Same(ex, err)` does: a panic cannot carry
//! an `Rc`); the observer `MyObserver`, a subclass of `ObserverBase<int>`, is an `ObserverBase`
//! of a core with its handlers (`ObserverCore`).

use ferroui_reactive::{IObserver, ObservableError, ObservableExt, ObserverBase, ObserverCore};
use ferroui_reactive_testing::{assert_subscriptions, ReactiveTest as RT, TestScheduler};
use std::rc::Rc;

#[test]
fn producer_observer_throws() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(210, 1), RT::on_next(220, 2), RT::on_next(230, 3)]);

    let ys = scheduler.create_hot_observable(vec![RT::on_next(215, 1), RT::on_next(225, 2), RT::on_next(235, 3)]);

    // This creates a producer.
    let res = xs.as_observable().combine_latest(ys.as_observable(), |x, y| x + y);

    scheduler.schedule_absolute_action(200, move || {
        res.subscribe_next(|z| {
            if z == 4 {
                panic!("ex");
            }
        });
    });

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| scheduler.run()));
    assert!(result.is_err(), "the panic of the observer reaches the caller");

    assert_eq!(225, scheduler.clock());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 225));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 225));
}

#[test]
fn producer_observer_throws_custom_observer() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(210, 1), RT::on_next(220, 2), RT::on_next(230, 3)]);

    let ys = scheduler.create_hot_observable(vec![RT::on_next(215, 1), RT::on_next(225, 2), RT::on_next(235, 3)]);

    // This creates a producer.
    let res = xs.as_observable().combine_latest(ys.as_observable(), |x, y| x + y);

    scheduler.schedule_absolute_action(200, move || {
        let observer: Rc<dyn IObserver<i32>> = Rc::new(ObserverBase::new(MyObserver { predicate: Box::new(|x: i32| x == 4), exception: "ex" }));
        res.subscribe(observer);
    });

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| scheduler.run()));
    assert!(result.is_err(), "the panic of the observer reaches the caller");

    assert_eq!(225, scheduler.clock());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 225));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(200, 225));
}

/// An observer that throws on the values its predicate selects (`MyObserver`, an
/// `ObserverBase<int>`): it panics with `exception`.
struct MyObserver {
    predicate: Box<dyn Fn(i32) -> bool>,
    exception: &'static str,
}

impl ObserverCore<i32> for MyObserver {
    fn on_next_core(&self, value: i32) {
        if (self.predicate)(value) {
            panic!("{}", self.exception);
        }
    }

    fn on_error_core(&self, _error: ObservableError) {}

    fn on_completed_core(&self) {}
}

// Not ported: SubscribeSafe_ArgumentChecking - argument validation
// Not ported: Safety_Subscription1 - a subscription that throws (RogueObservable) is a panic in the port, which the subscription of an operator (SubscribeSafe) does not deliver to the observer as an error
// Not ported: Safety_Subscription2 - a subscription that throws (RogueObservable) is a panic in the port, which the subscription of an operator (SubscribeSafe) does not deliver to the observer as an error
// Not ported: ObservableBase_ObserverThrows - real threads (Scheduler.Default, ManualResetEvent)
// Not ported: ObservableBase_ObserverThrows_CustomObserver - real threads (Scheduler.Default, ManualResetEvent)
