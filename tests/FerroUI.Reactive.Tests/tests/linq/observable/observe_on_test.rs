//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/ObserveOnTest.cs` of Rx.NET (the tests
//! of `ObserveOnTest` and `ObserveOnReactiveTest`).

use ferroui_reactive::ObservableExt;
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ReactiveTest as RT, TestScheduler};

#[test]
fn observe_on_scheduler_completed() {
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

    let (source, s) = (xs.as_observable(), scheduler.scheduler());
    let results = scheduler.start(move || source.observe_on(s));

    assert_messages!(
        results.messages(),
        RT::on_next(231, 3),
        RT::on_next(241, 4),
        RT::on_next(311, 5),
        RT::on_next(471, 6),
        RT::on_completed::<i32>(531),
    );

    // BREAKING CHANGE v2 > v1.x of upstream: more aggressive disposal.
    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 531));
}

#[test]
fn observe_on_scheduler_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(90, 1),
        RT::on_next(120, 2),
        RT::on_next(230, 3),
        RT::on_next(240, 4),
        RT::on_next(310, 5),
        RT::on_next(470, 6),
        RT::on_error(530, ex.clone()),
    ]);

    let (source, s) = (xs.as_observable(), scheduler.scheduler());
    let results = scheduler.start(move || source.observe_on(s));

    assert_messages!(
        results.messages(),
        RT::on_next(231, 3),
        RT::on_next(241, 4),
        RT::on_next(311, 5),
        RT::on_next(471, 6),
        RT::on_error::<i32>(531, ex.clone()),
    );

    // BREAKING CHANGE v2 > v1.x of upstream: more aggressive disposal.
    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 531));
}

#[test]
fn observe_on_scheduler_dispose() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(90, 1),
        RT::on_next(120, 2),
        RT::on_next(230, 3),
        RT::on_next(240, 4),
        RT::on_next(310, 5),
        RT::on_next(470, 6),
    ]);

    let (source, s) = (xs.as_observable(), scheduler.scheduler());
    let results = scheduler.start(move || source.observe_on(s));

    assert_messages!(results.messages(), RT::on_next(231, 3), RT::on_next(241, 4), RT::on_next(311, 5), RT::on_next(471, 6));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn observe_on_scheduler_same_time() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(210, 1), RT::on_next(210, 2)]);

    let (source, s) = (xs.as_observable(), scheduler.scheduler());
    let results = scheduler.start(move || source.observe_on(s));

    assert_messages!(results.messages(), RT::on_next(211, 1), RT::on_next(212, 2));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

// Not ported: ObserveOn_ArgumentChecking - argument validation
// Not ported: ObserveOn_Control - Windows Forms (ControlScheduler), real threads
// Not ported: ObserveOn_ControlScheduler - Windows Forms (ControlScheduler), real threads
// Not ported: ObserveOn_Dispatcher - WPF (DispatcherScheduler), real threads
// Not ported: ObserveOn_DispatcherScheduler - WPF (DispatcherScheduler), real threads
// Not ported: ObserveOn_CurrentDispatcher - WPF (DispatcherScheduler), real threads
// Not ported: ObserveOn_Error - WPF (DispatcherScheduler), real threads
// Not ported: ObserveOn_Scheduler_ArgumentChecking - argument validation
// Not ported: ObserveOn_Scheduler_OnNextThrows - real threads (Scheduler.Default, ManualResetEvent)
// Not ported: ObserveOn_LongRunning_Simple - real threads (ISchedulerLongRunning, which the port does not have)
// Not ported: ObserveOn_LongRunning_Error - real threads (ISchedulerLongRunning, which the port does not have)
// Not ported: ObserveOn_LongRunning_TimeVariance - real threads (ISchedulerLongRunning, which the port does not have)
// Not ported: ObserveOn_LongRunning_HoldUpDuringDispatchAndFail - real threads (ISchedulerLongRunning, which the port does not have)
// Not ported: ObserveOn_LongRunning_Cancel - real threads (ISchedulerLongRunning, which the port does not have)
// Not ported: ObserveOn_LongRunning_OnNextThrows - real threads (ISchedulerLongRunning, which the port does not have)
// Not ported: ObserveOn_SynchronizationContext_Simple - needs ObserveOn(SynchronizationContext) (phase 4)
// Not ported: ObserveOn_EventLoop_Long - real threads (EventLoopScheduler)
// Not ported: ObserveOn_LongRunning_SameThread - real threads (TaskPoolScheduler)
// Not ported: ObserveOn_LongRunning_DisableOptimizations - real threads (TaskPoolScheduler)
