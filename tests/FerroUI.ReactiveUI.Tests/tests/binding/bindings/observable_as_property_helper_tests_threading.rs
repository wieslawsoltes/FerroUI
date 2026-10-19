//! `ObservableAsPropertyHelperTests`, its partial file `ObservableAsPropertyHelperTests.Threading.cs`
//! (`ReactiveUI.Binding.Tests/Bindings`).
//!
//! Upstream's `ManualSequencer.RunPending` is `TestScheduler::run`, which runs the work queued on
//! the scheduler. A callback that throws is a callback that panics; the test catches the panic.

use super::observable_as_property_helper_tests::{recording, SUPPLIED_VALUE};
use ferroui_reactive::concurrency::ImmediateScheduler;
use ferroui_reactive::subjects::Subject;
use ferroui_reactive_testing::TestScheduler;
use ferroui_reactiveui::*;
use ferroui_system::{IDisposable, IObserver};
use std::cell::{Cell, RefCell};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

#[test]
fn scheduler_custom_sequencer_does_not_deliver_until_sequencer_runs() {
    let scheduler = TestScheduler::new();
    let source = Subject::<i32>::new();
    let (received, on_changed) = recording();
    let fixture = ObservableAsPropertyHelper::with_initial_value_on(source.as_observable(), on_changed, 0, Some(scheduler.scheduler()));

    // The initial value delivery is scheduled too, so nothing has run yet.
    assert!(received.borrow().is_empty());
    scheduler.run();
    assert_eq!(*received.borrow(), vec![0]);
    source.on_next(SUPPLIED_VALUE);
    assert_eq!(*received.borrow(), vec![0]);
    scheduler.run();
    assert_eq!(*received.borrow(), vec![0, SUPPLIED_VALUE]);
    fixture.dispose();
}

#[test]
fn scheduler_immediate_sequencer_is_treated_as_inline_delivery() {
    let source = Subject::<i32>::new();
    let (received, on_changed) = recording();
    let fixture = ObservableAsPropertyHelper::with_initial_value_on(source.as_observable(), on_changed, 0, Some(ImmediateScheduler::instance()));

    assert_eq!(*received.borrow(), vec![0]);
    source.on_next(SUPPLIED_VALUE);
    assert_eq!(*received.borrow(), vec![0, SUPPLIED_VALUE]);
    fixture.dispose();
}

#[test]
fn serialization_reentrant_produce_inside_callback_queues_rather_than_nesting() {
    const REENTRANT_VALUE: i32 = 2;
    let source = Subject::<i32>::new();
    let received: Rc<RefCell<Vec<i32>>> = Rc::new(RefCell::new(Vec::new()));
    let depth = Rc::new(Cell::new(0));
    let max_depth = Rc::new(Cell::new(0));

    let (sink, callback_depth, callback_max_depth, producer) = (received.clone(), depth.clone(), max_depth.clone(), source.clone());
    let fixture = ObservableAsPropertyHelper::with_initial_value(
        source.as_observable(),
        move |value: i32| {
            callback_depth.set(callback_depth.get() + 1);
            callback_max_depth.set(callback_max_depth.get().max(callback_depth.get()));
            sink.borrow_mut().push(value);
            if value == 1 {
                producer.on_next(REENTRANT_VALUE);
            }
            callback_depth.set(callback_depth.get() - 1);
        },
        0,
    );
    received.borrow_mut().clear();
    source.on_next(1);

    assert_eq!(*received.borrow(), vec![1, REENTRANT_VALUE]);
    assert_eq!(max_depth.get(), 1);
    fixture.dispose();
}

// Not ported: Serialization_ValueFromAnotherThreadWhileDeliveryRuns_QueuesAndDeliversAfterOnTheDeliveringThread - real threads

#[test]
fn serialization_callback_throws_propagates_and_drops_queued_values_but_recovers_for_later_values() {
    const QUEUED_VALUE: i32 = 2;
    const RECOVERY_VALUE: i32 = 3;
    let source = Subject::<i32>::new();
    let received: Rc<RefCell<Vec<i32>>> = Rc::new(RefCell::new(Vec::new()));

    let (sink, producer) = (received.clone(), source.clone());
    let fixture = ObservableAsPropertyHelper::with_initial_value(
        source.as_observable(),
        move |value: i32| {
            sink.borrow_mut().push(value);
            if value != 1 {
                return;
            }
            producer.on_next(QUEUED_VALUE);
            panic!("boom");
        },
        0,
    );

    let outcome = catch_unwind(AssertUnwindSafe(|| source.on_next(1)));
    assert!(outcome.is_err());
    // The queued value was queued reentrantly behind value 1's delivery, but that delivery
    // faulted, so it is dropped.
    assert_eq!(*received.borrow(), vec![0, 1]);
    source.on_next(RECOVERY_VALUE);
    assert_eq!(*received.borrow(), vec![0, 1, RECOVERY_VALUE]);
    fixture.dispose();
}
