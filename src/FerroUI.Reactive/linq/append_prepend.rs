//! `Append` and `Prepend` (`Linq/Observable/AppendPrepend.cs`).
//!
//! Deviation (DEVIATIONS.md, Reactive): upstream fuses a chain of `Append` and `Prepend` on the
//! same scheduler into one operator (`AppendPrepend<TSource>.Recursive` and `LongRunning`, reached
//! through `IAppendPrepend`); the handle of a sequence in the port does not reveal the type of the
//! sequence, so each call wraps the previous one (`SingleImmediate` or `SingleValue`). The values
//! and their virtual times are the same: each value scheduled on a scheduler is its own work.

use crate::concurrency::{IScheduler, ImmediateScheduler, SchedulerExt};
use crate::disposables::SingleAssignmentDisposableValue;
use crate::internal::{subscribe_sink, Sink};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::rc::{Rc, Weak};

/// The sink of `Append` and `Prepend` on the immediate scheduler
/// (`AppendPrepend<TSource>.SingleImmediate._`): the prepended value is notified before the
/// subscription to the source, the appended one after the source completes.
struct SingleImmediateSink<T> {
    sink: Sink<T>,
    value: T,
    append: bool,
}

impl<T: Clone + 'static> SingleImmediateSink<T> {
    fn run(this: &Rc<Self>, source: &Rc<dyn IObservable<T>>) {
        if !this.append {
            this.sink.forward_on_next(this.value.clone());
        }
        let subscription = source.subscribe(this.clone());
        this.sink.set_upstream(subscription);
    }
}

impl<T: 'static> IDisposable for SingleImmediateSink<T> {
    fn dispose(&self) {
        self.sink.dispose();
    }
}

impl<T: Clone + 'static> IObserver<T> for SingleImmediateSink<T> {
    fn on_next(&self, value: T) {
        self.sink.forward_on_next(value);
    }

    fn on_error(&self, error: ObservableError) {
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        if self.append {
            self.sink.forward_on_next(self.value.clone());
        }
        self.sink.forward_on_completed();
    }
}

/// The sink of `Append` and `Prepend` on a scheduler (`AppendPrepend<TSource>.SingleValue._`): the
/// prepended value is notified by work on the scheduler, which then subscribes to the source; the
/// appended one by work scheduled when the source completes.
struct SingleValueSink<T> {
    this: Weak<SingleValueSink<T>>,
    sink: Sink<T>,
    source: Rc<dyn IObservable<T>>,
    value: T,
    scheduler: Rc<dyn IScheduler>,
    append: bool,
    scheduler_disposable: SingleAssignmentDisposableValue,
}

impl<T: Clone + 'static> SingleValueSink<T> {
    fn run(this: &Rc<Self>) {
        let subscription = if this.append {
            this.source.subscribe(this.clone())
        } else {
            let sink = this.clone();
            this.scheduler.schedule(Box::new(move |_| {
                // `PrependValue`.
                sink.sink.forward_on_next(sink.value.clone());
                sink.source.subscribe(sink.clone())
            }))
        };
        this.sink.set_upstream(subscription);
    }
}

impl<T: 'static> IDisposable for SingleValueSink<T> {
    fn dispose(&self) {
        self.scheduler_disposable.dispose();
        self.sink.dispose();
    }
}

impl<T: Clone + 'static> IObserver<T> for SingleValueSink<T> {
    fn on_next(&self, value: T) {
        self.sink.forward_on_next(value);
    }

    fn on_error(&self, error: ObservableError) {
        // `ForwardOnError` disposes the sink through `Dispose(bool)`, which disposes the work of
        // the appended value too.
        self.sink.forward_on_error(error);
        self.dispose();
    }

    fn on_completed(&self) {
        if self.append {
            let Some(sink) = self.this.upgrade() else { return };
            let disposable = self.scheduler.schedule_action(move || {
                // `AppendValue`.
                sink.sink.forward_on_next(sink.value.clone());
                sink.sink.forward_on_completed();
                sink.dispose();
            });
            self.scheduler_disposable.set_disposable(Some(disposable));
        } else {
            self.sink.forward_on_completed();
            self.dispose();
        }
    }
}

/// The observable of `Append` and `Prepend` of one value (`AppendPrepend<TSource>.SingleImmediate`
/// and `SingleValue`).
struct AppendPrependObservable<T> {
    source: Rc<dyn IObservable<T>>,
    value: T,
    scheduler: Rc<dyn IScheduler>,
    append: bool,
}

impl<T: Clone + 'static> IObservable<T> for AppendPrependObservable<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let source = self.source.clone();
        let value = self.value.clone();
        let append = self.append;
        let immediate = ImmediateScheduler::instance();
        if std::ptr::addr_eq(Rc::as_ptr(&self.scheduler), Rc::as_ptr(&immediate)) {
            subscribe_sink(
                observer,
                move |observer| Rc::new(SingleImmediateSink { sink: Sink::new(observer), value, append }),
                move |sink: &Rc<SingleImmediateSink<T>>| SingleImmediateSink::run(sink, &source),
            )
        } else {
            let scheduler = self.scheduler.clone();
            subscribe_sink(
                observer,
                move |observer| {
                    Rc::new_cyclic(|this| SingleValueSink {
                        this: this.clone(),
                        sink: Sink::new(observer),
                        source,
                        value,
                        scheduler,
                        append,
                        scheduler_disposable: SingleAssignmentDisposableValue::new(),
                    })
                },
                move |sink: &Rc<SingleValueSink<T>>| SingleValueSink::run(sink),
            )
        }
    }
}

pub(crate) fn append_prepend<T: Clone + 'static>(
    source: Rc<dyn IObservable<T>>,
    value: T,
    scheduler: Rc<dyn IScheduler>,
    append: bool,
) -> Rc<dyn IObservable<T>> {
    Rc::new(AppendPrependObservable { source, value, scheduler, append })
}

/// The methods of `Append` and `Prepend` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! append_prepend_methods {
    () => {
        /// The values of the sequence, then `value` when it completes (`Append(TSource)`).
        fn append(&self, value: T) -> std::rc::Rc<dyn ferroui_system::IObservable<T>>
        where
            T: Clone,
        {
            crate::linq::append_prepend::append_prepend(
                self.observable(),
                value,
                crate::concurrency::SchedulerDefaults::constant_time_operations(),
                true,
            )
        }

        /// The values of the sequence, then `value`, notified on `scheduler`, when it completes
        /// (`Append(TSource, IScheduler)`).
        fn append_on(&self, value: T, scheduler: std::rc::Rc<dyn crate::concurrency::IScheduler>) -> std::rc::Rc<dyn ferroui_system::IObservable<T>>
        where
            T: Clone,
        {
            crate::linq::append_prepend::append_prepend(self.observable(), value, scheduler, true)
        }

        /// `value`, then the values of the sequence (`Prepend(TSource)`).
        fn prepend(&self, value: T) -> std::rc::Rc<dyn ferroui_system::IObservable<T>>
        where
            T: Clone,
        {
            crate::linq::append_prepend::append_prepend(
                self.observable(),
                value,
                crate::concurrency::SchedulerDefaults::constant_time_operations(),
                false,
            )
        }

        /// `value`, notified on `scheduler`, then the values of the sequence
        /// (`Prepend(TSource, IScheduler)`).
        fn prepend_on(&self, value: T, scheduler: std::rc::Rc<dyn crate::concurrency::IScheduler>) -> std::rc::Rc<dyn ferroui_system::IObservable<T>>
        where
            T: Clone,
        {
            crate::linq::append_prepend::append_prepend(self.observable(), value, scheduler, false)
        }
    };
}
pub(crate) use append_prepend_methods;
