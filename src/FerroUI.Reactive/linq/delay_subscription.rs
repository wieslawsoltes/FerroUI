//! `DelaySubscription` (`Linq/Observable/DelaySubscription.cs`).

use crate::concurrency::IScheduler;
use crate::internal::{impl_sink_dispose, subscribe_sink, OperatorSink, Sink};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::rc::Rc;
use std::time::{Duration, SystemTime};

/// When the subscription to the source is made: after a time, or at a time.
#[derive(Clone, Copy)]
pub(crate) enum DueTime {
    /// `DelaySubscription<TSource>.Relative`.
    Relative(Duration),
    /// `DelaySubscription<TSource>.Absolute`.
    Absolute(SystemTime),
}

/// The sink of `DelaySubscription` (`DelaySubscription<TSource>._`): forwards everything; its
/// subscription upstream is the scheduled work, which subscribes to the source when it runs.
struct DelaySubscriptionSink<T> {
    sink: Sink<T>,
}

impl<T: 'static> OperatorSink<T, T> for DelaySubscriptionSink<T> {
    fn sink(&self) -> &Sink<T> {
        &self.sink
    }
}

impl_sink_dispose!(DelaySubscriptionSink<T>);

impl<T: 'static> IObserver<T> for DelaySubscriptionSink<T> {
    fn on_next(&self, value: T) {
        self.sink.forward_on_next(value);
    }

    fn on_error(&self, error: ObservableError) {
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        self.sink.forward_on_completed();
    }
}

/// The observable of `DelaySubscription` (`DelaySubscription<TSource>`).
struct DelaySubscriptionObservable<T> {
    source: Rc<dyn IObservable<T>>,
    due_time: DueTime,
    scheduler: Rc<dyn IScheduler>,
}

impl<T: 'static> IObservable<T> for DelaySubscriptionObservable<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let source = self.source.clone();
        let due_time = self.due_time;
        let scheduler = self.scheduler.clone();
        subscribe_sink(
            observer,
            |observer| Rc::new(DelaySubscriptionSink { sink: Sink::new(observer) }),
            move |sink: &Rc<DelaySubscriptionSink<T>>| {
                // The scheduled work returns the subscription to the source, which the scheduled
                // item disposes when it is disposed (`ScheduleAction` with a `Func<TState, IDisposable>`).
                let target = sink.clone();
                let subscribe: crate::concurrency::ScheduledAction = Box::new(move |_| source.subscribe(target));
                let scheduled = match due_time {
                    DueTime::Relative(due) => scheduler.schedule_after(due, subscribe),
                    DueTime::Absolute(due) => scheduler.schedule_at(due, subscribe),
                };
                sink.sink.set_upstream(scheduled);
            },
        )
    }
}

pub(crate) fn delay_subscription<T: 'static>(
    source: Rc<dyn IObservable<T>>,
    due_time: DueTime,
    scheduler: Rc<dyn IScheduler>,
) -> Rc<dyn IObservable<T>> {
    Rc::new(DelaySubscriptionObservable { source, due_time, scheduler })
}

/// The methods of `DelaySubscription` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! delay_subscription_methods {
    () => {
        /// Subscribes to the sequence after `due_time`, on the scheduler of time-based operations
        /// (`DelaySubscription(TimeSpan)`).
        fn delay_subscription(&self, due_time: std::time::Duration) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::delay_subscription::delay_subscription(
                self.observable(),
                crate::linq::delay_subscription::DueTime::Relative(due_time),
                crate::concurrency::SchedulerDefaults::time_based_operations(),
            )
        }

        /// Subscribes to the sequence after `due_time` on `scheduler`
        /// (`DelaySubscription(TimeSpan, IScheduler)`).
        fn delay_subscription_on(
            &self,
            due_time: std::time::Duration,
            scheduler: std::rc::Rc<dyn crate::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::delay_subscription::delay_subscription(
                self.observable(),
                crate::linq::delay_subscription::DueTime::Relative(due_time),
                scheduler,
            )
        }

        /// Subscribes to the sequence at `due_time`, on the scheduler of time-based operations
        /// (`DelaySubscription(DateTimeOffset)`).
        fn delay_subscription_at(&self, due_time: std::time::SystemTime) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::delay_subscription::delay_subscription(
                self.observable(),
                crate::linq::delay_subscription::DueTime::Absolute(due_time),
                crate::concurrency::SchedulerDefaults::time_based_operations(),
            )
        }

        /// Subscribes to the sequence at `due_time` on `scheduler`
        /// (`DelaySubscription(DateTimeOffset, IScheduler)`).
        fn delay_subscription_at_on(
            &self,
            due_time: std::time::SystemTime,
            scheduler: std::rc::Rc<dyn crate::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::delay_subscription::delay_subscription(
                self.observable(),
                crate::linq::delay_subscription::DueTime::Absolute(due_time),
                scheduler,
            )
        }
    };
}
pub(crate) use delay_subscription_methods;
