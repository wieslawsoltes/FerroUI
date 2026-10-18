//! `Create`, `Empty`, `Never`, `Return` and `Throw` (`Linq/Observable/Empty.cs`,
//! `Linq/Observable/Never.cs`, `Linq/Observable/Return.cs`, `Linq/Observable/Throw.cs`, and the
//! `Create` of `QueryLanguage.Creation.cs`), the identity sink the factories of the family share,
//! and the static members of `Observable`, which the modules of the other factories extend.

use crate::concurrency::{IScheduler, ImmediateScheduler, SchedulerDefaults, SchedulerExt};
use crate::disposables::{Disposable, MultipleAssignmentDisposableValue};
use crate::internal::{impl_sink_dispose, subscribe_auto_detach, subscribe_basic, subscribe_sink, OperatorSink, Sink};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::marker::PhantomData;
use std::rc::Rc;

/// The static members of `Observable`.
pub struct Observable;

/// The sink of an operator whose values are those of its observer (`IdentitySink<T>`): it forwards
/// what it observes, and runs on a source by subscribing to it.
pub(crate) struct IdentitySink<T> {
    pub(crate) sink: Sink<T>,
}

impl<T: 'static> IdentitySink<T> {
    pub(crate) fn new(observer: Rc<dyn IObserver<T>>) -> Rc<Self> {
        Rc::new(Self { sink: Sink::new(observer) })
    }

    /// Subscribes the sink to `source` (`IdentitySink<T>.Run(IObservable<T>)`).
    pub(crate) fn run(self: &Rc<Self>, source: Rc<dyn IObservable<T>>) {
        let upstream = source.subscribe(self.clone());
        self.sink.set_upstream(upstream);
    }
}

impl<T: 'static> OperatorSink<T, T> for IdentitySink<T> {
    fn sink(&self) -> &Sink<T> {
        &self.sink
    }
}

impl_sink_dispose!(IdentitySink<T>);

impl<T: 'static> IObserver<T> for IdentitySink<T> {
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

/// Assigns the first scheduled work of a loop unless the loop has assigned its next work already
/// (`MultipleAssignmentDisposableValue.TrySetFirst`); a slot disposed meanwhile disposes it.
pub(crate) fn try_set_first(task: &MultipleAssignmentDisposableValue, first: Rc<dyn IDisposable>) {
    if task.disposable().is_none() {
        task.set_disposable(Some(first));
    }
}

/// Whether `scheduler` is the immediate scheduler (`scheduler == ImmediateScheduler.Instance`).
pub(crate) fn is_immediate(scheduler: &Rc<dyn IScheduler>) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(scheduler), Rc::as_ptr(&ImmediateScheduler::instance()))
}

/// An observable whose subscription is a function (`CreateWithDisposableObservable<T>`, an
/// `ObservableBase<T>`).
struct AnonymousObservable<T> {
    subscribe: Rc<dyn Fn(Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable>>,
}

impl<T: 'static> IObservable<T> for AnonymousObservable<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let subscribe = self.subscribe.clone();
        subscribe_auto_detach(observer, move |observer| subscribe(observer))
    }
}

/// `Empty<T>(IScheduler)` (`Empty<TResult>`): the completion, scheduled.
struct EmptyObservable<T> {
    scheduler: Rc<dyn IScheduler>,
    _marker: PhantomData<fn() -> T>,
}

impl<T: 'static> IObservable<T> for EmptyObservable<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let scheduler = self.scheduler.clone();
        subscribe_sink(observer, IdentitySink::new, move |sink: &Rc<IdentitySink<T>>| {
            let target = sink.clone();
            sink.sink.set_upstream(scheduler.schedule_action(move || target.on_completed()));
        })
    }
}

/// `Empty<T>()` (`EmptyDirect<TResult>`): the completion, at once.
struct EmptyDirect<T> {
    _marker: PhantomData<fn() -> T>,
}

impl<T: 'static> IObservable<T> for EmptyDirect<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        subscribe_basic(observer, |observer| {
            observer.on_completed();
            Disposable::empty()
        })
    }
}

/// `Never<T>()` (`Never<TResult>`): no notification, and nothing to dispose.
struct NeverObservable<T> {
    _marker: PhantomData<fn() -> T>,
}

impl<T: 'static> IObservable<T> for NeverObservable<T> {
    fn subscribe(&self, _observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        Disposable::empty()
    }
}

/// `Return<T>(T, IScheduler)` (`Return<TResult>`): the value and the completion, scheduled.
struct ReturnObservable<T> {
    value: T,
    scheduler: Rc<dyn IScheduler>,
}

impl<T: Clone + 'static> IObservable<T> for ReturnObservable<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let value = self.value.clone();
        let scheduler = self.scheduler.clone();
        subscribe_sink(observer, IdentitySink::new, move |sink: &Rc<IdentitySink<T>>| {
            let target = sink.clone();
            sink.sink.set_upstream(scheduler.schedule_action(move || {
                target.sink.forward_on_next(value);
                target.sink.forward_on_completed();
            }));
        })
    }
}

/// `Return<T>(T)` on the immediate scheduler (`ReturnImmediate<TSource>`): the value and the
/// completion, at once.
struct ReturnImmediate<T> {
    value: T,
}

impl<T: Clone + 'static> IObservable<T> for ReturnImmediate<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let value = self.value.clone();
        subscribe_basic(observer, move |observer| {
            observer.on_next(value);
            observer.on_completed();
            Disposable::empty()
        })
    }
}

/// `Throw<T>(Exception, IScheduler)` (`Throw<TResult>`): the error, scheduled.
struct ThrowObservable<T> {
    error: ObservableError,
    scheduler: Rc<dyn IScheduler>,
    _marker: PhantomData<fn() -> T>,
}

impl<T: 'static> IObservable<T> for ThrowObservable<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let error = self.error.clone();
        let scheduler = self.scheduler.clone();
        subscribe_sink(observer, IdentitySink::new, move |sink: &Rc<IdentitySink<T>>| {
            let target = sink.clone();
            sink.sink.set_upstream(scheduler.schedule_action(move || target.sink.forward_on_error(error)));
        })
    }
}

/// `Throw<T>(Exception)` on the immediate scheduler (`ThrowImmediate<TSource>`): the error, at once.
struct ThrowImmediate<T> {
    error: ObservableError,
    _marker: PhantomData<fn() -> T>,
}

impl<T: 'static> IObservable<T> for ThrowImmediate<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let error = self.error.clone();
        subscribe_basic(observer, move |observer| {
            observer.on_error(error);
            Disposable::empty()
        })
    }
}

impl Observable {
    /// A sequence whose subscription is `subscribe`, which returns the disposable that ends it
    /// (`Create<TResult>(Func<IObserver<TResult>, IDisposable>)`). The observer it receives ignores
    /// everything after a terminal notification, and a terminal notification disposes the
    /// subscription.
    pub fn create<T: 'static>(subscribe: impl Fn(Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> + 'static) -> Rc<dyn IObservable<T>> {
        Rc::new(AnonymousObservable { subscribe: Rc::new(subscribe) })
    }

    /// A sequence whose subscription is `subscribe`, which returns the action that ends it
    /// (`Create<TResult>(Func<IObserver<TResult>, Action>)`).
    pub fn create_with_action<T: 'static, A: FnOnce() + 'static>(
        subscribe: impl Fn(Rc<dyn IObserver<T>>) -> A + 'static,
    ) -> Rc<dyn IObservable<T>> {
        Self::create(move |observer| Disposable::create(subscribe(observer)))
    }

    /// A sequence that ends at once (`Empty<TResult>()`).
    pub fn empty<T: 'static>() -> Rc<dyn IObservable<T>> {
        Rc::new(EmptyDirect { _marker: PhantomData })
    }

    /// [`empty`](Self::empty), with a value that only names the type of the sequence
    /// (`Empty<TResult>(TResult witness)`).
    pub fn empty_with_witness<T: 'static>(_witness: T) -> Rc<dyn IObservable<T>> {
        Self::empty()
    }

    /// A sequence that ends on `scheduler` (`Empty<TResult>(IScheduler)`).
    pub fn empty_on<T: 'static>(scheduler: Rc<dyn IScheduler>) -> Rc<dyn IObservable<T>> {
        Rc::new(EmptyObservable { scheduler, _marker: PhantomData })
    }

    /// [`empty_on`](Self::empty_on), with a value that only names the type of the sequence
    /// (`Empty<TResult>(IScheduler, TResult witness)`).
    pub fn empty_on_with_witness<T: 'static>(scheduler: Rc<dyn IScheduler>, _witness: T) -> Rc<dyn IObservable<T>> {
        Self::empty_on(scheduler)
    }

    /// A sequence that never notifies (`Never<TResult>()`).
    pub fn never<T: 'static>() -> Rc<dyn IObservable<T>> {
        Rc::new(NeverObservable { _marker: PhantomData })
    }

    /// [`never`](Self::never), with a value that only names the type of the sequence
    /// (`Never<TResult>(TResult witness)`).
    pub fn never_with_witness<T: 'static>(_witness: T) -> Rc<dyn IObservable<T>> {
        Self::never()
    }

    /// A sequence of one value, on the scheduler of constant-time operations, the immediate one
    /// (`Return<TResult>(TResult)`).
    pub fn return_<T: Clone + 'static>(value: T) -> Rc<dyn IObservable<T>> {
        Self::return_on(value, SchedulerDefaults::constant_time_operations())
    }

    /// A sequence of one value, on `scheduler` (`Return<TResult>(TResult, IScheduler)`); on the
    /// immediate scheduler the value and the end are notified at once.
    pub fn return_on<T: Clone + 'static>(value: T, scheduler: Rc<dyn IScheduler>) -> Rc<dyn IObservable<T>> {
        if is_immediate(&scheduler) {
            return Rc::new(ReturnImmediate { value });
        }
        Rc::new(ReturnObservable { value, scheduler })
    }

    /// A sequence that ends with `error`, on the scheduler of constant-time operations, the
    /// immediate one (`Throw<TResult>(Exception)`).
    pub fn throw<T: 'static>(error: ObservableError) -> Rc<dyn IObservable<T>> {
        Self::throw_on(error, SchedulerDefaults::constant_time_operations())
    }

    /// [`throw`](Self::throw), with a value that only names the type of the sequence
    /// (`Throw<TResult>(Exception, TResult witness)`).
    pub fn throw_with_witness<T: 'static>(error: ObservableError, _witness: T) -> Rc<dyn IObservable<T>> {
        Self::throw(error)
    }

    /// A sequence that ends with `error`, on `scheduler` (`Throw<TResult>(Exception, IScheduler)`);
    /// on the immediate scheduler the error is notified at once.
    pub fn throw_on<T: 'static>(error: ObservableError, scheduler: Rc<dyn IScheduler>) -> Rc<dyn IObservable<T>> {
        if is_immediate(&scheduler) {
            return Rc::new(ThrowImmediate { error, _marker: PhantomData });
        }
        Rc::new(ThrowObservable { error, scheduler, _marker: PhantomData })
    }

    /// [`throw_on`](Self::throw_on), with a value that only names the type of the sequence
    /// (`Throw<TResult>(Exception, IScheduler, TResult witness)`).
    pub fn throw_on_with_witness<T: 'static>(error: ObservableError, scheduler: Rc<dyn IScheduler>, _witness: T) -> Rc<dyn IObservable<T>> {
        Self::throw_on(error, scheduler)
    }
}
