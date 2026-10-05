use super::{AnonymousObserver, Disposable, IDisposable, IObservable, IObserver, ObservableError};
use std::marker::PhantomData;
use std::rc::Rc;

/// Factory functions for observables.
pub struct Observable;

struct CreateObservable<T, F> {
    subscribe: F,
    _marker: PhantomData<fn(T)>,
}

impl<T, F> IObservable<T> for CreateObservable<T, F>
where
    F: Fn(Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable>,
{
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        (self.subscribe)(observer)
    }
}

impl Observable {
    /// Creates an observable from a subscribe function.
    pub fn create<T: 'static>(
        subscribe: impl Fn(Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> + 'static,
    ) -> Rc<dyn IObservable<T>> {
        Rc::new(CreateObservable { subscribe, _marker: PhantomData })
    }

    /// An observable that produces a single value and completes.
    pub fn return_<T: Clone + 'static>(value: T) -> Rc<dyn IObservable<T>> {
        Self::create(move |observer| {
            observer.on_next(value.clone());
            observer.on_completed();
            Disposable::empty()
        })
    }

    /// An observable that produces a single value and never completes.
    pub fn single_value<T: Clone + 'static>(value: T) -> Rc<dyn IObservable<T>> {
        Self::create(move |observer| {
            observer.on_next(value.clone());
            Disposable::empty()
        })
    }

    /// An observable that never produces a value.
    pub fn never<T: 'static>() -> Rc<dyn IObservable<T>> {
        Self::create(|_| Disposable::empty())
    }

    /// An observable that completes immediately.
    pub fn empty<T: 'static>() -> Rc<dyn IObservable<T>> {
        Self::create(|observer| {
            observer.on_completed();
            Disposable::empty()
        })
    }
}

struct MapObserver<T, U, F> {
    target: Rc<dyn IObserver<U>>,
    f: Rc<F>,
    _marker: PhantomData<fn(T)>,
}

impl<T, U, F: Fn(T) -> U> IObserver<T> for MapObserver<T, U, F> {
    fn on_next(&self, value: T) {
        self.target.on_next((self.f)(value))
    }
    fn on_error(&self, error: ObservableError) {
        self.target.on_error(error)
    }
    fn on_completed(&self) {
        self.target.on_completed()
    }
}

struct FilterObserver<T, F> {
    target: Rc<dyn IObserver<T>>,
    f: Rc<F>,
}

impl<T, F: Fn(&T) -> bool> IObserver<T> for FilterObserver<T, F> {
    fn on_next(&self, value: T) {
        if (self.f)(&value) {
            self.target.on_next(value)
        }
    }
    fn on_error(&self, error: ObservableError) {
        self.target.on_error(error)
    }
    fn on_completed(&self) {
        self.target.on_completed()
    }
}

/// Operators available on every shared observable.
pub trait ObservableExt<T: 'static> {
    /// Subscribes with a closure invoked for every value.
    fn subscribe_fn(&self, on_next: impl Fn(T) + 'static) -> Rc<dyn IDisposable>;

    /// Projects each value.
    fn select<U: 'static>(&self, f: impl Fn(T) -> U + 'static) -> Rc<dyn IObservable<U>>;

    /// Filters values.
    fn where_(&self, f: impl Fn(&T) -> bool + 'static) -> Rc<dyn IObservable<T>>;
}

impl<T: 'static> ObservableExt<T> for Rc<dyn IObservable<T>> {
    fn subscribe_fn(&self, on_next: impl Fn(T) + 'static) -> Rc<dyn IDisposable> {
        self.subscribe(Rc::new(AnonymousObserver::new(on_next)))
    }

    fn select<U: 'static>(&self, f: impl Fn(T) -> U + 'static) -> Rc<dyn IObservable<U>> {
        let source = self.clone();
        let f = Rc::new(f);
        Observable::create(move |observer| {
            source.subscribe(Rc::new(MapObserver { target: observer, f: f.clone(), _marker: PhantomData }))
        })
    }

    fn where_(&self, f: impl Fn(&T) -> bool + 'static) -> Rc<dyn IObservable<T>> {
        let source = self.clone();
        let f = Rc::new(f);
        Observable::create(move |observer| {
            source.subscribe(Rc::new(FilterObserver { target: observer, f: f.clone() }))
        })
    }
}
