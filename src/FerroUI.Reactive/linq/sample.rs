//! `Sample` (`Linq/Observable/Sample.cs`): the latest value of the source, forwarded at each tick of
//! a period (`Sample<TSource>`) or at each value of a sampler sequence (`Sample<TSource, TSample>`),
//! if a value came since the previous one was forwarded.

use crate::concurrency::{IScheduler, SchedulerExt};
use crate::disposables::SingleAssignmentDisposableValue;
use crate::internal::{subscribe_sink, OperatorSink, Sink};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::marker::PhantomData;
use std::rc::Rc;
use std::time::Duration;

/// The sink of `Sample` with a sampler (`Sample<TSource, TSample>._`).
struct SamplerSink<T> {
    sink: Sink<T>,
    source_disposable: SingleAssignmentDisposableValue,
    sampler_disposable: SingleAssignmentDisposableValue,
    has_value: Cell<bool>,
    value: RefCell<Option<T>>,
    source_at_end: Cell<bool>,
    sampler_at_end: Cell<bool>,
}

impl<T: 'static> SamplerSink<T> {
    /// Forwards the latest value if one came since the previous sample.
    fn forward_latest(&self) {
        if self.has_value.get() {
            self.has_value.set(false);
            let value = self.value.borrow_mut().take();
            if let Some(value) = value {
                self.sink.forward_on_next(value);
            }
        }
    }
}

impl<T: 'static> OperatorSink<T, T> for SamplerSink<T> {
    fn sink(&self) -> &Sink<T> {
        &self.sink
    }
}

impl<T: 'static> IDisposable for SamplerSink<T> {
    fn dispose(&self) {
        self.source_disposable.dispose();
        self.sampler_disposable.dispose();
        self.sink.dispose();
    }
}

impl<T: 'static> IObserver<T> for SamplerSink<T> {
    fn on_next(&self, value: T) {
        self.has_value.set(true);
        *self.value.borrow_mut() = Some(value);
    }

    fn on_error(&self, error: ObservableError) {
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        self.source_at_end.set(true);
        if self.sampler_at_end.get() {
            self.sink.forward_on_completed();
        } else {
            self.source_disposable.dispose();
        }
    }
}

/// The observer of the sampler (`Sample<TSource, TSample>._.SampleObserver`).
struct SampleObserver<T, S> {
    parent: Rc<SamplerSink<T>>,
    _marker: PhantomData<fn(S)>,
}

impl<T: 'static, S: 'static> IObserver<S> for SampleObserver<T, S> {
    fn on_next(&self, _value: S) {
        self.parent.forward_latest();
        if self.parent.source_at_end.get() {
            self.parent.sink.forward_on_completed();
        }
    }

    fn on_error(&self, error: ObservableError) {
        // BREAKING CHANGE v2 > v1.x of upstream: the error of the sampler ends the sequence.
        self.parent.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        self.parent.sampler_at_end.set(true);
        self.parent.forward_latest();
        if self.parent.source_at_end.get() {
            self.parent.sink.forward_on_completed();
        } else {
            self.parent.sampler_disposable.dispose();
        }
    }
}

/// The observable of `Sample` with a sampler (`Sample<TSource, TSample>`).
struct SamplerObservable<T, S> {
    source: Rc<dyn IObservable<T>>,
    sampler: Rc<dyn IObservable<S>>,
}

impl<T: 'static, S: 'static> IObservable<T> for SamplerObservable<T, S> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let (source, sampler) = (self.source.clone(), self.sampler.clone());
        subscribe_sink(
            observer,
            |observer| {
                Rc::new(SamplerSink {
                    sink: Sink::new(observer),
                    source_disposable: SingleAssignmentDisposableValue::new(),
                    sampler_disposable: SingleAssignmentDisposableValue::new(),
                    has_value: Cell::new(false),
                    value: RefCell::new(None),
                    source_at_end: Cell::new(false),
                    sampler_at_end: Cell::new(false),
                })
            },
            move |sink: &Rc<SamplerSink<T>>| {
                sink.source_disposable.set_disposable(Some(source.subscribe(sink.clone())));
                let sample_observer: Rc<dyn IObserver<S>> = Rc::new(SampleObserver { parent: sink.clone(), _marker: PhantomData });
                sink.sampler_disposable.set_disposable(Some(sampler.subscribe(sample_observer)));
            },
        )
    }
}

pub(crate) fn sample_by<T: 'static, S: 'static>(source: Rc<dyn IObservable<T>>, sampler: Rc<dyn IObservable<S>>) -> Rc<dyn IObservable<T>> {
    Rc::new(SamplerObservable { source, sampler })
}

/// The sink of `Sample` with a period (`Sample<TSource>._`): the periodic work is its subscription
/// upstream, the subscription to the source is held apart, so that the source can end before the
/// last tick.
struct SampleSink<T> {
    sink: Sink<T>,
    source_disposable: SingleAssignmentDisposableValue,
    has_value: Cell<bool>,
    value: RefCell<Option<T>>,
    at_end: Cell<bool>,
}

impl<T: 'static> SampleSink<T> {
    fn tick(&self) {
        if self.has_value.get() {
            self.has_value.set(false);
            let value = self.value.borrow_mut().take();
            if let Some(value) = value {
                self.sink.forward_on_next(value);
            }
        }
        if self.at_end.get() {
            self.sink.forward_on_completed();
        }
    }
}

impl<T: 'static> OperatorSink<T, T> for SampleSink<T> {
    fn sink(&self) -> &Sink<T> {
        &self.sink
    }
}

impl<T: 'static> IDisposable for SampleSink<T> {
    fn dispose(&self) {
        self.source_disposable.dispose();
        self.sink.dispose();
    }
}

impl<T: 'static> IObserver<T> for SampleSink<T> {
    fn on_next(&self, value: T) {
        self.has_value.set(true);
        *self.value.borrow_mut() = Some(value);
    }

    fn on_error(&self, error: ObservableError) {
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        self.at_end.set(true);
        self.source_disposable.dispose();
    }
}

/// The observable of `Sample` with a period (`Sample<TSource>`).
struct SampleObservable<T> {
    source: Rc<dyn IObservable<T>>,
    interval: Duration,
    scheduler: Rc<dyn IScheduler>,
}

impl<T: 'static> IObservable<T> for SampleObservable<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let (source, interval, scheduler) = (self.source.clone(), self.interval, self.scheduler.clone());
        subscribe_sink(
            observer,
            |observer| {
                Rc::new(SampleSink {
                    sink: Sink::new(observer),
                    source_disposable: SingleAssignmentDisposableValue::new(),
                    has_value: Cell::new(false),
                    value: RefCell::new(None),
                    at_end: Cell::new(false),
                })
            },
            move |sink: &Rc<SampleSink<T>>| {
                sink.source_disposable.set_disposable(Some(source.subscribe(sink.clone())));
                let target = sink.clone();
                let periodic = scheduler.schedule_periodic_action(interval, move || target.tick());
                sink.sink.set_upstream(periodic);
            },
        )
    }
}

pub(crate) fn sample<T: 'static>(source: Rc<dyn IObservable<T>>, interval: Duration, scheduler: Rc<dyn IScheduler>) -> Rc<dyn IObservable<T>> {
    Rc::new(SampleObservable { source, interval, scheduler })
}

/// The methods of `Sample` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! sample_methods {
    () => {
        /// The latest value at each `interval`, if one came since the previous one, on the
        /// scheduler of time-based operations (`Sample(TimeSpan)`).
        fn sample(&self, interval: std::time::Duration) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::sample::sample(self.observable(), interval, crate::concurrency::SchedulerDefaults::time_based_operations())
        }

        /// The latest value at each `interval` on `scheduler`, if one came since the previous one
        /// (`Sample(TimeSpan, IScheduler)`).
        fn sample_on(
            &self,
            interval: std::time::Duration,
            scheduler: std::rc::Rc<dyn crate::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::sample::sample(self.observable(), interval, scheduler)
        }

        /// The latest value at each value of `sampler`, if one came since the previous one
        /// (`Sample(IObservable<TSample>)`).
        fn sample_by<S: 'static>(
            &self,
            sampler: std::rc::Rc<dyn ferroui_system::IObservable<S>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::sample::sample_by(self.observable(), sampler)
        }
    };
}
pub(crate) use sample_methods;
