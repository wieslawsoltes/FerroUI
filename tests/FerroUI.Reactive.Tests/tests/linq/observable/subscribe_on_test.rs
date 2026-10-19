//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/SubscribeOnTest.cs` of Rx.NET.
//! Adapted: the variables that the subscription and its disposal assign are shared cells.
//! `SubscribeOn(SynchronizationContext)` is `subscribe_on_context`, with the context `MyCtx` of
//! `ObserveOnTest.cs` (`observe_on_test.rs`).

use super::observe_on_test::MyCtx;
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::{IObserver, Observable, ObservableExt};
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ReactiveTest as RT, TestScheduler};
use std::cell::Cell;
use std::rc::Rc;

#[test]
fn subscribe_on_scheduler_sleep() {
    let scheduler = TestScheduler::new();

    let s = Rc::new(Cell::new(0i64));
    let d = Rc::new(Cell::new(0i64));

    let xs = {
        let (clock, s, d) = (scheduler.clone(), s.clone(), d.clone());
        Observable::create(move |_observer: Rc<dyn IObserver<i64>>| {
            s.set(clock.clock());
            let (clock, d) = (clock.clone(), d.clone());
            Disposable::create(move || d.set(clock.clock()))
        })
    };

    let sch = scheduler.scheduler();
    let results = scheduler.start(move || xs.subscribe_on(sch));

    assert_messages!(results.messages());

    assert_eq!(201, s.get());
    assert_eq!(1001, d.get());
}

#[test]
fn subscribe_on_scheduler_completed() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_completed::<i64>(300)]);

    let (source, s) = (xs.as_observable(), scheduler.scheduler());
    let results = scheduler.start(move || source.subscribe_on(s));

    assert_messages!(results.messages(), RT::on_completed::<i64>(300));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(201, 301));
}

#[test]
fn subscribe_on_scheduler_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![RT::on_error::<i32>(300, ex.clone())]);

    let (source, s) = (xs.as_observable(), scheduler.scheduler());
    let results = scheduler.start(move || source.subscribe_on(s));

    assert_messages!(results.messages(), RT::on_error::<i32>(300, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(201, 301));
}

#[test]
fn subscribe_on_scheduler_dispose() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable::<i32>(vec![]);

    let (source, s) = (xs.as_observable(), scheduler.scheduler());
    let results = scheduler.start(move || source.subscribe_on(s));

    assert_messages!(results.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(201, 1001));
}

#[test]
fn subscribe_on_synchronization_context_simple() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(90, 1),
        RT::on_next(120, 2),
        RT::on_next(230, 3),
        RT::on_next(240, 4),
        RT::on_next(310, 5),
        RT::on_next(470, 6),
        RT::on_completed(530),
    ]);

    let (source, context) = (xs.as_observable(), MyCtx::new(scheduler.scheduler()));
    let results = scheduler.start(move || source.subscribe_on_context(context));

    assert_messages!(
        results.messages(),
        RT::on_next(230, 3),
        RT::on_next(240, 4),
        RT::on_next(310, 5),
        RT::on_next(470, 6),
        RT::on_completed::<i32>(530)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(201, 531));
}

// Not ported: SubscribeOn_ArgumentChecking - argument validation
// Not ported: SubscribeOn_Control - Windows Forms (ControlScheduler), real threads
// Not ported: SubscribeOn_ControlScheduler - Windows Forms (ControlScheduler), real threads
// Not ported: SubscribeOn_Dispatcher - WPF (DispatcherScheduler), real threads
// Not ported: SubscribeOn_DispatcherScheduler - WPF (DispatcherScheduler), real threads
// Not ported: SubscribeOn_CurrentDispatcher - WPF (DispatcherScheduler), real threads
// Not ported: SubscribeOn_Scheduler_ArgumentChecking - argument validation
