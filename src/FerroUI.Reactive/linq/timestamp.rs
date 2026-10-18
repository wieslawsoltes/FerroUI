//! `Timestamp` (`Linq/Observable/Timestamp.cs`).

use crate::concurrency::IScheduler;
use crate::internal::{impl_sink_dispose, unary, OperatorSink, Sink};
use crate::Timestamped;
use ferroui_system::{IObservable, IObserver, ObservableError};
use std::rc::Rc;

/// The sink of `Timestamp` (`Timestamp<TSource>._`): each value with the time of the scheduler when
/// it arrived.
struct TimestampSink<T> {
    sink: Sink<Timestamped<T>>,
    scheduler: Rc<dyn IScheduler>,
}

impl<T: 'static> OperatorSink<T, Timestamped<T>> for TimestampSink<T> {
    fn sink(&self) -> &Sink<Timestamped<T>> {
        &self.sink
    }
}

impl_sink_dispose!(TimestampSink<T>);

impl<T: 'static> IObserver<T> for TimestampSink<T> {
    fn on_next(&self, value: T) {
        self.sink.forward_on_next(Timestamped::new(value, self.scheduler.now()));
    }

    fn on_error(&self, error: ObservableError) {
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        self.sink.forward_on_completed();
    }
}

pub(crate) fn timestamp<T: 'static>(source: Rc<dyn IObservable<T>>, scheduler: Rc<dyn IScheduler>) -> Rc<dyn IObservable<Timestamped<T>>> {
    unary(source, move |observer| Rc::new(TimestampSink { sink: Sink::new(observer), scheduler: scheduler.clone() }))
}

/// The methods of `Timestamp` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! timestamp_methods {
    () => {
        /// Each value with the time it arrived, on the scheduler of time-based operations
        /// (`Timestamp()`).
        fn timestamp(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::Timestamped<T>>> {
            crate::linq::timestamp::timestamp(self.observable(), crate::concurrency::SchedulerDefaults::time_based_operations())
        }

        /// Each value with the time of `scheduler` when it arrived (`Timestamp(IScheduler)`).
        fn timestamp_on(
            &self,
            scheduler: std::rc::Rc<dyn crate::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::Timestamped<T>>> {
            crate::linq::timestamp::timestamp(self.observable(), scheduler)
        }
    };
}
pub(crate) use timestamp_methods;
