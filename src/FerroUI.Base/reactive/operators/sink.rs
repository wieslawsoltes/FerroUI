// Code based on https://github.com/dotnet/reactive/blob/main/Rx.NET/Source/src/System.Reactive/Internal/Sink.cs

use crate::reactive::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::RefCell;
use std::rc::Rc;

/// The state of an operator's subscription: the observer it forwards to and
/// the subscription to its upstream source.
///
/// The original is an abstract class with a virtual `Dispose`; here an
/// operator embeds the state and implements [`ISink`], whose
/// [`dispose`](ISink::dispose) is the virtual.
pub(crate) struct Sink<TTarget> {
    upstream: RefCell<Option<Rc<dyn IDisposable>>>,
    observer: Rc<dyn IObserver<TTarget>>,
}

impl<TTarget> Sink<TTarget> {
    pub fn new(observer: Rc<dyn IObserver<TTarget>>) -> Self {
        Self { upstream: RefCell::new(None), observer }
    }

    /// The base `Dispose`: disposes the upstream subscription.
    pub fn dispose(&self) {
        self.dispose_upstream();
    }

    pub fn forward_on_next(&self, value: TTarget) {
        self.observer.on_next(value);
    }

    pub fn set_upstream(&self, upstream: Rc<dyn IDisposable>) {
        *self.upstream.borrow_mut() = Some(upstream);
    }

    pub fn dispose_upstream(&self) {
        let upstream = self.upstream.borrow().clone();
        if let Some(upstream) = upstream {
            upstream.dispose();
        }
    }
}

/// Implemented by the subscription of an operator; see [`Sink`].
pub(crate) trait ISink<TTarget> {
    /// The embedded state.
    fn sink(&self) -> &Sink<TTarget>;

    /// Ends the subscription. An operator that holds subscriptions of its
    /// own disposes them and then calls [`Sink::dispose`].
    fn dispose(&self) {
        self.sink().dispose();
    }

    fn forward_on_next(&self, value: TTarget) {
        self.sink().forward_on_next(value);
    }

    fn forward_on_completed(&self) {
        self.sink().observer.on_completed();
        self.dispose();
    }

    fn forward_on_error(&self, error: ObservableError) {
        self.sink().observer.on_error(error);
        self.dispose();
    }
}

/// Subscribes a sink that observes `source` to it and keeps the
/// subscription as its upstream (the original's `Run` of the sink with a
/// source type).
#[allow(dead_code)] // upstream member without a user yet
pub(crate) fn run<TSource, TTarget, S>(sink: &Rc<S>, source: &dyn IObservable<TSource>)
where
    S: ISink<TTarget> + IObserver<TSource> + 'static,
{
    let observer: Rc<dyn IObserver<TSource>> = sink.clone();
    sink.sink().set_upstream(source.subscribe(observer));
}

/// An observer that forwards to a sink (the original's `GetForwarder`).
#[allow(dead_code)] // upstream member without a user yet
pub(crate) fn get_forwarder<TTarget: 'static>(sink: Rc<dyn ISink<TTarget>>) -> Rc<dyn IObserver<TTarget>> {
    Rc::new(Forwarder { forward: sink })
}

struct Forwarder<TTarget> {
    forward: Rc<dyn ISink<TTarget>>,
}

impl<TTarget> IObserver<TTarget> for Forwarder<TTarget> {
    fn on_next(&self, value: TTarget) {
        self.forward.forward_on_next(value)
    }

    fn on_error(&self, error: ObservableError) {
        self.forward.forward_on_error(error)
    }

    fn on_completed(&self) {
        self.forward.forward_on_completed()
    }
}

/// A sink whose source and target type are the same: it forwards what it
/// observes.
#[allow(dead_code)] // upstream type whose subclasses here embed `Sink` directly
pub(crate) struct IdentitySink<T> {
    sink: Sink<T>,
}

#[allow(dead_code)]
impl<T> IdentitySink<T> {
    pub fn new(observer: Rc<dyn IObserver<T>>) -> Self {
        Self { sink: Sink::new(observer) }
    }
}

impl<T> ISink<T> for IdentitySink<T> {
    fn sink(&self) -> &Sink<T> {
        &self.sink
    }
}

impl<T> IObserver<T> for IdentitySink<T> {
    fn on_next(&self, value: T) {
        self.forward_on_next(value)
    }

    fn on_error(&self, error: ObservableError) {
        self.forward_on_error(error)
    }

    fn on_completed(&self) {
        self.forward_on_completed()
    }
}

/// A subscription that is assigned after the observer that ends it exists.
pub(crate) struct SubscriptionSlot {
    disposable: RefCell<Option<Rc<dyn IDisposable>>>,
}

impl SubscriptionSlot {
    pub fn new() -> Rc<Self> {
        Rc::new(Self { disposable: RefCell::new(None) })
    }

    pub fn set(&self, disposable: Rc<dyn IDisposable>) {
        *self.disposable.borrow_mut() = Some(disposable);
    }
}

impl IDisposable for SubscriptionSlot {
    fn dispose(&self) {
        let disposable = self.disposable.borrow().clone();
        if let Some(disposable) = disposable {
            disposable.dispose();
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream: the original has no tests of these types.
    use super::*;
    use crate::reactive::{AnonymousObserver, Disposable, LightweightSubject};
    use std::cell::Cell;

    #[test]
    fn identity_sink_forwards_and_disposes_its_upstream_on_completion() {
        let received = Rc::new(RefCell::new(Vec::new()));
        let completed = Rc::new(Cell::new(false));
        let (r, c) = (received.clone(), completed.clone());
        let observer: Rc<dyn IObserver<i32>> = Rc::new(AnonymousObserver::new_with_completed(
            move |value: i32| r.borrow_mut().push(value),
            move || c.set(true),
        ));

        let source = LightweightSubject::<i32>::new();
        let sink = Rc::new(IdentitySink::new(observer));
        run::<i32, i32, _>(&sink, &source);
        assert!(source.has_observers());

        source.on_next(1);
        get_forwarder::<i32>(sink.clone()).on_next(2);
        assert_eq!(vec![1, 2], *received.borrow());

        sink.on_completed();
        assert!(completed.get());
        assert!(!source.has_observers());
    }

    #[test]
    fn subscription_slot_disposes_what_it_was_given() {
        let disposed = Rc::new(Cell::new(0));
        let d = disposed.clone();
        let slot = SubscriptionSlot::new();

        slot.dispose();
        slot.set(Disposable::create(move || d.set(d.get() + 1)));
        slot.dispose();
        assert_eq!(1, disposed.get());
    }
}
