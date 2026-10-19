//! `ReactiveSchedulerExtensions`: the bindings that name the scheduler their writes are delivered
//! on.
//!
//! Upstream's overloads (`BindOneWay`, `BindTwoWay`, `OneWayBind`, `Bind` with an `ISequencer`, and
//! with `IBindingTypeConverter`s, a conversion hint and a scheduler) are the `_on` and `_with`
//! methods of [`SourceBindingMixins`](crate::SourceBindingMixins) and
//! [`PropertyBindingMixins`](crate::PropertyBindingMixins); this module holds the choice of the
//! delivery they share.

use crate::binding::bindings::BindingSchedulers;
use ferroui_reactive::concurrency::{IScheduler, Scheduler};
use ferroui_reactive::ObservableExt;
use ferroui_system::IObservable;
use std::any::Any;
use std::rc::Rc;

/// Whether `scheduler` is the immediate scheduler of the thread, which runs work at once
/// (`scheduler == Scheduler.Immediate`).
fn is_immediate(scheduler: &Rc<dyn IScheduler>) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(scheduler), Rc::as_ptr(&Scheduler::immediate()))
}

/// The values of a one-way write delivered on `scheduler`, each in turn; on the thread that owns
/// `target` when no scheduler is named; at once on the immediate scheduler (`Schedule`).
pub(crate) fn schedule_write<T: 'static>(values: Rc<dyn IObservable<T>>, scheduler: Option<Rc<dyn IScheduler>>, target: Option<Rc<dyn Any>>) -> Rc<dyn IObservable<T>> {
    match scheduler {
        None => BindingSchedulers::observe_on_view_thread(values, target),
        Some(scheduler) if is_immediate(&scheduler) => values,
        Some(scheduler) => values.observe_on(scheduler),
    }
}

/// The values of one direction of a two-way write delivered on `scheduler`, only the latest one
/// that waits; on the thread that owns `target` when no scheduler is named; at once on the
/// immediate scheduler (`ScheduleLatest`).
///
/// Each write raises the other side's change. A queue that replayed every value would write an
/// older value back over a newer one, and the two sides would hand each other stale values without
/// end.
pub(crate) fn schedule_latest_write<T: 'static>(values: Rc<dyn IObservable<T>>, scheduler: Option<Rc<dyn IScheduler>>, target: Option<Rc<dyn Any>>) -> Rc<dyn IObservable<T>> {
    match scheduler {
        None => BindingSchedulers::observe_on_view_thread(values, target),
        Some(scheduler) if is_immediate(&scheduler) => values,
        Some(scheduler) => BindingSchedulers::observe_on_sequencer(values, scheduler),
    }
}
