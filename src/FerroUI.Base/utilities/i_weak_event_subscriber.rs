use super::weak_event::WeakEventArgs;
use super::HandlerList;
use crate::reactive::{Disposable, IDisposable};
use std::any::Any;
use std::rc::{Rc, Weak};

/// Defines a listener to a event subscribed vis the [`WeakEvent`](super::WeakEvent).
///
/// `sender` is the object that raised the event, `ev` the weak event it was
/// raised through (compare it by address with the event, for example
/// `std::ptr::addr_eq(ev, &*WeakEvents::collection_changed())`).
pub trait IWeakEventSubscriber<A: WeakEventArgs + ?Sized> {
    fn on_event(&self, sender: Option<&dyn Any>, ev: &dyn Any, e: &A::Args<'_>);
}

/// The signature of a handler of [`WeakEventSubscriber::event`] and of the
/// dispatch function of a [`TargetWeakEventSubscriber`] (after its target).
pub type WeakEventSubscriberHandler<A> =
    dyn for<'s, 'v, 'r, 'a> Fn(Option<&'s dyn Any>, &'v dyn Any, &'r <A as WeakEventArgs>::Args<'a>);

/// A subscriber that raises its [`event`](Self::event) for every event it
/// receives.
pub struct WeakEventSubscriber<A: WeakEventArgs + ?Sized> {
    this: Weak<WeakEventSubscriber<A>>,
    event: HandlerList<WeakEventSubscriberHandler<A>>,
}

impl<A: WeakEventArgs + ?Sized> WeakEventSubscriber<A> {
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|this| Self { this: this.clone(), event: HandlerList::new() })
    }

    /// Subscribes to the events this subscriber receives. Disposing the
    /// returned handle unsubscribes.
    pub fn event<F>(&self, handler: F) -> Rc<dyn IDisposable>
    where
        F: for<'s, 'v, 'r, 'a> Fn(Option<&'s dyn Any>, &'v dyn Any, &'r A::Args<'a>) + 'static,
    {
        let token = self.event.add(Rc::new(handler));
        let weak = self.this.clone();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.event.remove(token);
            }
        })
    }
}

impl<A: WeakEventArgs + ?Sized> IWeakEventSubscriber<A> for WeakEventSubscriber<A> {
    fn on_event(&self, sender: Option<&dyn Any>, ev: &dyn Any, e: &A::Args<'_>) {
        if self.event.is_empty() {
            return;
        }
        for (_, handler) in self.event.snapshot().iter() {
            handler(sender, ev, e);
        }
    }
}

/// A subscriber that passes every event it receives to a dispatch function,
/// together with its target.
pub struct TargetWeakEventSubscriber<TTarget, A: WeakEventArgs + ?Sized> {
    target: TTarget,
    dispatch_func: Box<TargetDispatchFunc<TTarget, A>>,
}

type TargetDispatchFunc<TTarget, A> =
    dyn for<'t, 's, 'v, 'r, 'a> Fn(&'t TTarget, Option<&'s dyn Any>, &'v dyn Any, &'r <A as WeakEventArgs>::Args<'a>);

impl<TTarget, A: WeakEventArgs + ?Sized> TargetWeakEventSubscriber<TTarget, A> {
    pub fn new<F>(target: TTarget, dispatch_func: F) -> Self
    where
        F: for<'t, 's, 'v, 'r, 'a> Fn(&'t TTarget, Option<&'s dyn Any>, &'v dyn Any, &'r A::Args<'a>) + 'static,
    {
        Self { target, dispatch_func: Box::new(dispatch_func) }
    }
}

impl<TTarget, A: WeakEventArgs + ?Sized> IWeakEventSubscriber<A> for TargetWeakEventSubscriber<TTarget, A> {
    fn on_event(&self, sender: Option<&dyn Any>, ev: &dyn Any, e: &A::Args<'_>) {
        (self.dispatch_func)(&self.target, sender, ev, e);
    }
}
