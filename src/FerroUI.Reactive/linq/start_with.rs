//! `StartWith` (`QueryLanguage.Single.cs`: `values.ToObservable(scheduler).Concat(source)`).
//!
//! Upstream composes `ToObservable` (`Linq/Observable/ToObservable.cs`, `ToObservableRecursive`)
//! and `Concat`; the port runs the same steps in one sink: each value is notified by its own work on
//! the scheduler, and the work after the last value subscribes to the source.

use crate::concurrency::IScheduler;
use crate::disposables::Disposable;
use crate::internal::{subscribe_sink, Sink};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::Cell;
use std::rc::Rc;

/// The sink of `StartWith`: the sink of `ToObservableRecursive<TSource>._` for the values, then
/// the sink of `Concat` for the source.
struct StartWithSink<T> {
    sink: Sink<T>,
    source: Rc<dyn IObservable<T>>,
    values: Rc<[T]>,
    index: Cell<usize>,
}

impl<T: Clone + 'static> StartWithSink<T> {
    /// Notifies the next value, or subscribes to the source after the last one (`LoopRec`, then
    /// the completion of the values that `Concat` follows with the source). The work is never
    /// cancelled: a disposed sink stops here.
    fn loop_rec(this: Rc<Self>, scheduler: &Rc<dyn IScheduler>) -> Rc<dyn IDisposable> {
        if this.sink.is_disposed() {
            return Disposable::empty();
        }
        let index = this.index.get();
        if index == this.values.len() {
            let subscription = this.source.subscribe(this.clone());
            this.sink.set_upstream(subscription);
            return Disposable::empty();
        }
        this.index.set(index + 1);
        this.sink.forward_on_next(this.values[index].clone());
        let next = this.clone();
        scheduler.schedule(Box::new(move |inner| StartWithSink::loop_rec(next, inner)));
        Disposable::empty()
    }
}

impl<T: 'static> IDisposable for StartWithSink<T> {
    fn dispose(&self) {
        self.sink.dispose();
    }
}

impl<T: 'static> IObserver<T> for StartWithSink<T> {
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

/// The observable of `StartWith`.
struct StartWithObservable<T> {
    source: Rc<dyn IObservable<T>>,
    values: Rc<[T]>,
    scheduler: Rc<dyn IScheduler>,
}

impl<T: Clone + 'static> IObservable<T> for StartWithObservable<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let source = self.source.clone();
        let values = self.values.clone();
        let scheduler = self.scheduler.clone();
        subscribe_sink(
            observer,
            move |observer| Rc::new(StartWithSink { sink: Sink::new(observer), source, values, index: Cell::new(0) }),
            move |sink: &Rc<StartWithSink<T>>| {
                let sink = sink.clone();
                scheduler.schedule(Box::new(move |inner| StartWithSink::loop_rec(sink, inner)));
            },
        )
    }
}

pub(crate) fn start_with<T: Clone + 'static>(source: Rc<dyn IObservable<T>>, values: Vec<T>, scheduler: Rc<dyn IScheduler>) -> Rc<dyn IObservable<T>> {
    Rc::new(StartWithObservable { source, values: values.into(), scheduler })
}

/// The methods of `StartWith` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! start_with_methods {
    () => {
        /// The values, then the values of the sequence; the values are notified on the immediate
        /// scheduler (`StartWith(params TSource[])`, `StartWith(IEnumerable<TSource>)`).
        fn start_with(&self, values: impl IntoIterator<Item = T>) -> std::rc::Rc<dyn ferroui_system::IObservable<T>>
        where
            T: Clone,
        {
            crate::linq::start_with::start_with(
                self.observable(),
                values.into_iter().collect(),
                crate::concurrency::SchedulerDefaults::constant_time_operations(),
            )
        }

        /// The values, notified on `scheduler`, then the values of the sequence
        /// (`StartWith(IScheduler, params TSource[])`, `StartWith(IScheduler, IEnumerable<TSource>)`;
        /// the scheduler is the last argument, as in the other operators of the port).
        fn start_with_on(
            &self,
            values: impl IntoIterator<Item = T>,
            scheduler: std::rc::Rc<dyn crate::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>>
        where
            T: Clone,
        {
            crate::linq::start_with::start_with(self.observable(), values.into_iter().collect(), scheduler)
        }
    };
}
pub(crate) use start_with_methods;
