//! `Using` (`Linq/Observable/Using.cs`).

use crate::disposables::{Disposable, SingleAssignmentDisposableValue};
use crate::internal::{subscribe_sink, Sink};
use crate::linq::Observable;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::rc::Rc;

/// `Using<TSource, TResource>`: each subscription creates a resource, subscribes to the sequence
/// made from it, and disposes the resource when the subscription ends.
struct UsingObservable<T, R: ?Sized> {
    resource_factory: Rc<dyn Fn() -> Result<Rc<R>, ObservableError>>,
    observable_factory: Rc<dyn Fn(Rc<R>) -> Result<Rc<dyn IObservable<T>>, ObservableError>>,
}

/// The sink of `Using`: an identity sink that also holds the resource.
struct UsingSink<T> {
    sink: Sink<T>,
    disposable: SingleAssignmentDisposableValue,
}

impl<T: 'static> UsingSink<T> {
    fn run<R: IDisposable + ?Sized + 'static>(
        self: &Rc<Self>,
        resource_factory: &Rc<dyn Fn() -> Result<Rc<R>, ObservableError>>,
        observable_factory: &Rc<dyn Fn(Rc<R>) -> Result<Rc<dyn IObservable<T>>, ObservableError>>,
    ) {
        let mut disposable = Disposable::empty();
        let source = match resource_factory() {
            Ok(resource) => {
                let held = resource.clone();
                disposable = Disposable::create(move || held.dispose());
                match observable_factory(resource) {
                    Ok(source) => source,
                    Err(error) => Observable::throw(error),
                }
            }
            Err(error) => Observable::throw(error),
        };

        // The resource is assigned after the subscription: a sequence that ends while it is
        // subscribed would else dispose the resource before the subscription.
        let upstream = source.subscribe(self.clone());
        self.sink.set_upstream(upstream);
        self.disposable.set_disposable(Some(disposable));
    }

    fn forward_on_error(&self, error: ObservableError) {
        let observer = self.sink.observer();
        observer.on_error(error);
        self.dispose();
    }

    fn forward_on_completed(&self) {
        let observer = self.sink.observer();
        observer.on_completed();
        self.dispose();
    }
}

impl<T: 'static> IDisposable for UsingSink<T> {
    fn dispose(&self) {
        self.sink.dispose();
        self.disposable.dispose();
    }
}

impl<T: 'static> IObserver<T> for UsingSink<T> {
    fn on_next(&self, value: T) {
        self.sink.forward_on_next(value);
    }

    fn on_error(&self, error: ObservableError) {
        self.forward_on_error(error);
    }

    fn on_completed(&self) {
        self.forward_on_completed();
    }
}

impl<T: 'static, R: IDisposable + ?Sized + 'static> IObservable<T> for UsingObservable<T, R> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let resource_factory = self.resource_factory.clone();
        let observable_factory = self.observable_factory.clone();
        subscribe_sink(
            observer,
            |observer| Rc::new(UsingSink { sink: Sink::new(observer), disposable: SingleAssignmentDisposableValue::new() }),
            move |sink: &Rc<UsingSink<T>>| sink.run(&resource_factory, &observable_factory),
        )
    }
}

impl crate::linq::Observable {
    /// A sequence that, on each subscription, creates a resource with `resource_factory`,
    /// subscribes to the sequence `observable_factory` makes from it, and disposes the resource
    /// when the subscription ends
    /// (`Using<TResult, TResource>(Func<TResource>, Func<TResource, IObservable<TResult>>)`).
    pub fn using<T: 'static, R: IDisposable + ?Sized + 'static>(
        resource_factory: impl Fn() -> Rc<R> + 'static,
        observable_factory: impl Fn(Rc<R>) -> Rc<dyn IObservable<T>> + 'static,
    ) -> Rc<dyn IObservable<T>> {
        Rc::new(UsingObservable::<T, R> {
            resource_factory: Rc::new(move || Ok(resource_factory())),
            observable_factory: Rc::new(move |resource| Ok(observable_factory(resource))),
        })
    }

    /// [`using`](Self::using) with factories that may fail; a failure ends the sequence with its
    /// error, after disposing the resource if it was created (`Using` with factories that throw).
    pub fn try_using<T: 'static, R: IDisposable + ?Sized + 'static>(
        resource_factory: impl Fn() -> Result<Rc<R>, ObservableError> + 'static,
        observable_factory: impl Fn(Rc<R>) -> Result<Rc<dyn IObservable<T>>, ObservableError> + 'static,
    ) -> Rc<dyn IObservable<T>> {
        Rc::new(UsingObservable::<T, R> { resource_factory: Rc::new(resource_factory), observable_factory: Rc::new(observable_factory) })
    }
}
