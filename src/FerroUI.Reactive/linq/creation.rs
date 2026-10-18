//! The factories of `Observable` that the core needs (`Observable.Creation.cs`): `Create`, `Empty`,
//! `Never`, `Return`, `Throw`. The other operators are in the modules of their families.

use crate::concurrency::{IScheduler, Scheduler, SchedulerExt};
use crate::disposables::Disposable;
use crate::internal::subscribe_auto_detach;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::rc::Rc;

/// The static members of `Observable`.
pub struct Observable;

/// An observable whose subscription is a function (`AnonymousObservable<T>`, the observable of
/// `Observable.Create`).
struct AnonymousObservable<T> {
    subscribe: Rc<dyn Fn(Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable>>,
}

impl<T: 'static> IObservable<T> for AnonymousObservable<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let subscribe = self.subscribe.clone();
        subscribe_auto_detach(observer, move |observer| subscribe(observer))
    }
}

impl Observable {
    /// A sequence whose subscription is `subscribe`, which returns the disposable that ends it
    /// (`Create(Func<IObserver<T>, IDisposable>)`). The observer it receives ignores everything after
    /// a terminal notification, and a terminal notification disposes the subscription.
    pub fn create<T: 'static>(subscribe: impl Fn(Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> + 'static) -> Rc<dyn IObservable<T>> {
        Rc::new(AnonymousObservable { subscribe: Rc::new(subscribe) })
    }

    /// A sequence whose subscription is `subscribe`, which returns the action that ends it
    /// (`Create(Func<IObserver<T>, Action>)`).
    pub fn create_with_action<T: 'static, A: FnOnce() + 'static>(
        subscribe: impl Fn(Rc<dyn IObserver<T>>) -> A + 'static,
    ) -> Rc<dyn IObservable<T>> {
        Self::create(move |observer| Disposable::create(subscribe(observer)))
    }

    /// A sequence that ends at once, on the immediate scheduler (`Empty<T>()`).
    pub fn empty<T: 'static>() -> Rc<dyn IObservable<T>> {
        Self::empty_on(Scheduler::immediate())
    }

    /// A sequence that ends on `scheduler` (`Empty<T>(IScheduler)`).
    pub fn empty_on<T: 'static>(scheduler: Rc<dyn IScheduler>) -> Rc<dyn IObservable<T>> {
        Self::create(move |observer: Rc<dyn IObserver<T>>| scheduler.schedule_action(move || observer.on_completed()))
    }

    /// A sequence that never notifies (`Never<T>()`).
    pub fn never<T: 'static>() -> Rc<dyn IObservable<T>> {
        Self::create(|_observer: Rc<dyn IObserver<T>>| Disposable::empty())
    }

    /// A sequence of one value, on the immediate scheduler (`Return<T>(T)`).
    pub fn return_<T: Clone + 'static>(value: T) -> Rc<dyn IObservable<T>> {
        Self::return_on(value, Scheduler::immediate())
    }

    /// A sequence of one value, on `scheduler` (`Return<T>(T, IScheduler)`).
    pub fn return_on<T: Clone + 'static>(value: T, scheduler: Rc<dyn IScheduler>) -> Rc<dyn IObservable<T>> {
        Self::create(move |observer: Rc<dyn IObserver<T>>| {
            let value = value.clone();
            scheduler.schedule_action(move || {
                observer.on_next(value);
                observer.on_completed();
            })
        })
    }

    /// A sequence that ends with `error`, on the immediate scheduler (`Throw<T>(Exception)`).
    pub fn throw<T: 'static>(error: ObservableError) -> Rc<dyn IObservable<T>> {
        Self::throw_on(error, Scheduler::immediate())
    }

    /// A sequence that ends with `error`, on `scheduler` (`Throw<T>(Exception, IScheduler)`).
    pub fn throw_on<T: 'static>(error: ObservableError, scheduler: Rc<dyn IScheduler>) -> Rc<dyn IObservable<T>> {
        Self::create(move |observer: Rc<dyn IObserver<T>>| {
            let error = error.clone();
            scheduler.schedule_action(move || observer.on_error(error))
        })
    }
}
