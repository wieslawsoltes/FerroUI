//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/ObserveOnTest.cs` of Rx.NET (the tests
//! of `ObserveOnTest` and `ObserveOnReactiveTest`).
//! Adapted: the context `MyCtx`, which schedules the work it is posted on the test scheduler, keeps
//! the scheduler in a slot of the thread that made it, since a context is shared between threads
//! and a scheduler is not; it is used on that thread only. `ObserveOn(SynchronizationContext)` is
//! `observe_on_context`.

use ferroui_reactive::concurrency::{IScheduler, SchedulerExt};
use ferroui_reactive::ObservableExt;
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ReactiveTest as RT, TestScheduler};
use ferroui_system::threading::SynchronizationContext;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::thread::ThreadId;

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

#[test]
fn observe_on_synchronization_context_simple() {
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
    let results = scheduler.start(move || source.observe_on_context(context));

    assert_messages!(
        results.messages(),
        RT::on_next(231, 3),
        RT::on_next(241, 4),
        RT::on_next(311, 5),
        RT::on_next(471, 6),
        RT::on_completed::<i32>(531)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 531));
}

ferroui_system::ferro_thread_local! {
    /// The scheduler that the `MyCtx` of the thread schedules its work on.
    static MY_CTX_SCHEDULER: RefCell<Option<Rc<dyn IScheduler>>> = const { RefCell::new(None) };
}

/// `MyCtx`: a context that schedules the work it is posted on a scheduler, which runs it. The
/// scheduler is that of the thread that made the context, the last one given to [`MyCtx::new`] on
/// that thread; the context is used on that thread only.
pub(super) struct MyCtx {
    thread: ThreadId,
}

impl MyCtx {
    pub(super) fn new(scheduler: Rc<dyn IScheduler>) -> Arc<MyCtx> {
        MY_CTX_SCHEDULER.with(|slot| *slot.borrow_mut() = Some(scheduler));
        Arc::new(MyCtx { thread: std::thread::current().id() })
    }

    fn scheduler(&self) -> Rc<dyn IScheduler> {
        assert_eq!(std::thread::current().id(), self.thread, "MyCtx is used on the thread that made it");
        MY_CTX_SCHEDULER.with(|slot| slot.borrow().clone()).expect("the scheduler of MyCtx is set")
    }
}

impl SynchronizationContext for MyCtx {
    fn post(&self, work: Box<dyn FnOnce() + Send>) {
        self.scheduler().schedule_action(work);
    }

    /// The `Send` of the base class: the work runs at once.
    fn send(&self, work: Box<dyn FnOnce() + Send>) {
        work();
    }

    fn post_local(&self, work: Box<dyn FnOnce()>) {
        self.scheduler().schedule_action(work);
    }

    fn send_local(&self, work: Box<dyn FnOnce()>) {
        work();
    }
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
// Not ported: ObserveOn_EventLoop_Long - real threads (EventLoopScheduler)
// Not ported: ObserveOn_LongRunning_SameThread - real threads (TaskPoolScheduler)
// Not ported: ObserveOn_LongRunning_DisableOptimizations - real threads (TaskPoolScheduler)
