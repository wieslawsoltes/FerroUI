//! `BatchIf<TObject, TKey>` (`Cache/Internal/BatchIf.cs`): the change sets held back while paused,
//! delivered as one when the pause ends, a time-out passes or an interval timer ticks.

use crate::cache::ChangeSet;
use crate::internal::DeliveryQueue;
use ferroui_reactive::concurrency::{IScheduler, SchedulerDefaults};
use ferroui_reactive::disposables::{Disposable, SerialDisposable};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

struct State<T: 'static, K: 'static> {
    batched_changes: RefCell<Vec<ChangeSet<T, K>>>,
    paused: Cell<bool>,
    queue: Rc<DeliveryQueue<ChangeSet<T, K>>>,
}

impl<T: Clone + 'static, K: Clone + 'static> State<T, K> {
    /// `ResumeAction`: the batched change sets as one, then an empty batch. The change set is
    /// delivered once the batch is cleared, so that an edit made by an observer starts a new one.
    fn resume_action(&self) {
        let resulting_batch: ChangeSet<T, K> = {
            let batched_changes = self.batched_changes.borrow();
            if batched_changes.is_empty() {
                return;
            }
            batched_changes.iter().flat_map(|changes| changes.iter().cloned()).collect()
        };
        let access = self.queue.acquire();
        access.enqueue_next(resulting_batch);
        self.batched_changes.borrow_mut().clear();
        drop(access);
    }
}

/// `BatchIf.Run`.
///
/// Deviation (DEVIATIONS.md, DynamicData): upstream serializes the source, the pause selector and
/// the timers through a `SharedDeliveryQueue`; on one thread only the deliveries are queued, so
/// that a change set caused by an observer is delivered after the current one.
pub(crate) fn batch_if<T: Clone + 'static, K: Clone + 'static>(
    source: Rc<dyn IObservable<ChangeSet<T, K>>>,
    pause_if_true_selector: Rc<dyn IObservable<bool>>,
    time_out: Option<Duration>,
    initial_pause_state: bool,
    interval_timer: Option<Rc<dyn IObservable<()>>>,
    scheduler: Option<Rc<dyn IScheduler>>,
) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
    let scheduler = scheduler.unwrap_or_else(SchedulerDefaults::time_based_operations);
    Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<T, K>>>| {
        let state = Rc::new(State { batched_changes: RefCell::new(Vec::new()), paused: Cell::new(initial_pause_state), queue: DeliveryQueue::new(observer) });
        let timeout_disposer = SerialDisposable::new();
        let interval_timer_disposer = SerialDisposable::new();

        if let Some(interval_timer) = &interval_timer {
            let (on_finally, on_tick) = (state.clone(), state.clone());
            let subscription = interval_timer.finally(move || on_finally.paused.set(false)).subscribe_next(move |_| {
                on_tick.paused.set(false);
                on_tick.resume_action();
                on_tick.paused.set(true);
            });
            interval_timer_disposer.set_disposable(Some(subscription));
        }

        let paused_handler = {
            let (state, timeout_disposer, scheduler) = (state.clone(), timeout_disposer.clone(), scheduler.clone());
            pause_if_true_selector.subscribe_next(move |paused: bool| {
                state.paused.set(paused);
                if !paused {
                    // The pause window has closed, so the timer is reset.
                    if time_out.is_some() {
                        timeout_disposer.set_disposable(Some(Disposable::empty()));
                    }
                    state.resume_action();
                } else if let Some(time_out) = time_out {
                    let on_timer = state.clone();
                    let timer = Observable::timer_on(time_out, scheduler.clone()).subscribe_next(move |_| {
                        on_timer.paused.set(false);
                        on_timer.resume_action();
                    });
                    timeout_disposer.set_disposable(Some(timer));
                }
            })
        };

        let publisher = {
            let state = state.clone();
            source.subscribe_next(move |changes: ChangeSet<T, K>| {
                state.batched_changes.borrow_mut().push(changes);
                // Published unless paused.
                if !state.paused.get() {
                    state.resume_action();
                }
            })
        };

        let queue = state.queue.clone();
        Disposable::create(move || {
            publisher.dispose();
            paused_handler.dispose();
            timeout_disposer.dispose();
            interval_timer_disposer.dispose();
            queue.dispose();
        })
    })
}
