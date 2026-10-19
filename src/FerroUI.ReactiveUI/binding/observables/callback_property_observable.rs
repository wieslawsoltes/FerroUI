//! `CallbackPropertyObservable<TSource, TValue>`.

use super::DisposeOnPanic;
use crate::binding::observable_for_property::ValueComparer;
use ferroui_reactive::disposables::Disposable;
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::RefCell;
use std::rc::Rc;

/// Attaches a callback to the change notification of a platform and returns what detaches it.
pub type AttachCallback<S> = Rc<dyn Fn(&S, Rc<dyn Fn()>) -> Rc<dyn IDisposable>>;

/// Observes a property through a change callback of its own: the value on subscription, then the
/// value each time the callback runs (`CallbackPropertyObservable<TSource, TValue>`).
///
/// An observer of a native mechanism (a property of a FerroUI object, a platform event) passes
/// what attaches to its notification as `subscribe`. The sequence never completes. Disposing a
/// subscription detaches it once, however many times it is disposed.
pub struct CallbackPropertyObservable<S: 'static, V: 'static> {
    source: S,
    subscribe: AttachCallback<S>,
    getter: Rc<dyn Fn(&S) -> V>,
    comparer: Option<ValueComparer<V>>,
}

impl<S: Clone + 'static, V: Clone + 'static> CallbackPropertyObservable<S, V> {
    /// The values of `getter` on `source`, read again each time the callback that `subscribe`
    /// attaches runs; with `distinct`, a value equal to the last one is dropped.
    pub fn new(
        source: S,
        subscribe: impl Fn(&S, Rc<dyn Fn()>) -> Rc<dyn IDisposable> + 'static,
        getter: impl Fn(&S) -> V + 'static,
        distinct: bool,
    ) -> Self
    where
        V: PartialEq,
    {
        let comparer: Option<ValueComparer<V>> = if distinct { Some(Rc::new(|first: &V, second: &V| first == second)) } else { None };
        Self { source, subscribe: Rc::new(subscribe), getter: Rc::new(getter), comparer }
    }
}

/// One subscriber's attachment (`Subscription`).
struct Subscription<S: 'static, V: 'static> {
    source: S,
    getter: Rc<dyn Fn(&S) -> V>,
    comparer: Option<ValueComparer<V>>,
    observer: RefCell<Option<Rc<dyn IObserver<V>>>>,
    last: RefCell<Option<V>>,
}

impl<S: 'static, V: Clone + 'static> Subscription<S, V> {
    /// Reads the property and delivers it, unless disposed or unchanged under the distinct gate
    /// (`Publish`).
    fn publish(&self) {
        let Some(observer) = self.observer.borrow().clone() else {
            return;
        };
        let value = (self.getter)(&self.source);
        if let Some(comparer) = &self.comparer {
            let repeats = self.last.borrow().as_ref().is_some_and(|last| comparer(last, &value));
            if repeats {
                return;
            }
        }
        *self.last.borrow_mut() = Some(value.clone());
        observer.on_next(value);
    }
}

impl<S: Clone + 'static, V: Clone + 'static> IObservable<V> for CallbackPropertyObservable<S, V> {
    fn subscribe(&self, observer: Rc<dyn IObserver<V>>) -> Rc<dyn IDisposable> {
        let subscription = Rc::new(Subscription {
            source: self.source.clone(),
            getter: self.getter.clone(),
            comparer: self.comparer.clone(),
            observer: RefCell::new(Some(observer)),
            last: RefCell::new(None),
        });
        let callback_subscription = subscription.clone();
        let attachment = (self.subscribe)(&self.source, Rc::new(move || callback_subscription.publish()));
        let state = subscription.clone();
        let disposable = DisposeOnPanic::new(Disposable::create(move || {
            if state.observer.borrow_mut().take().is_some() {
                attachment.dispose();
            }
        }));
        subscription.publish();
        disposable.defuse()
    }
}
