use super::{WeakEventArgs, WeakEventHandler, WeakEventSender};
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ops::Deref;
use std::rc::{Rc, Weak};

/// The events of a type, found by name: what the managed original looks up
/// through reflection on the target type.
pub trait WeakEventHandlerTarget<A: WeakEventArgs + ?Sized> {
    /// Adds `handler` to the event named `event_name` and returns the action
    /// that removes it, or `None` if the type has no such event.
    ///
    /// The action must not hold the target strongly: the target holds the
    /// subscription through the handler, and the subscription holds the
    /// action.
    fn add_event_handler(&self, event_name: &str, handler: Rc<WeakEventHandler<A>>) -> Option<Box<dyn FnOnce()>>;
}

/// The method a subscription calls on its subscriber: the method of the
/// managed delegate, whose target is the subscriber.
pub type WeakEventHandlerMethod<A, TSubscriber> =
    for<'s, 'r, 'a> fn(&TSubscriber, Option<&'s dyn Any>, &'r <A as WeakEventArgs>::Args<'a>);

/// Manages subscriptions to events using weak listeners.
pub struct WeakEventHandlerManager;

impl WeakEventHandlerManager {
    /// Subscribes to an event on an object using a weak subscription.
    ///
    /// The managed delegate is given as its two parts: the subscriber (its
    /// target), which is referenced weakly, and the method called on it.
    ///
    /// # Panics
    /// Panics if the type of `target` has no event named `event_name`.
    pub fn subscribe<TTarget, A, TSubscriber>(
        target: &TTarget,
        event_name: &str,
        subscriber: &Rc<TSubscriber>,
        method: WeakEventHandlerMethod<A, TSubscriber>,
    ) where
        TTarget: WeakEventSender + Deref,
        TTarget::Target: WeakEventHandlerTarget<A>,
        A: WeakEventArgs + ?Sized,
        TSubscriber: 'static,
    {
        let dic = SubscriptionTypeStorage::<A, TSubscriber>::subscribers().get_or_create_value(target);

        let existing = dic.subscriptions.borrow().get(event_name).cloned();
        let sub = match existing {
            Some(sub) => sub,
            None => {
                let sub = Subscription::new(&dic, target, event_name);
                dic.subscriptions.borrow_mut().insert(event_name.to_string(), sub.clone());
                sub
            }
        };

        sub.add(subscriber, method);
    }

    /// Unsubscribes from an event.
    ///
    /// Every subscription of `subscriber` to the event is removed, whatever
    /// method it calls, as the managed original compares the delegate
    /// targets only.
    pub fn unsubscribe<A, TSubscriber>(target: &impl WeakEventSender, event_name: &str, subscriber: &Rc<TSubscriber>)
    where
        A: WeakEventArgs + ?Sized,
        TSubscriber: 'static,
    {
        if let Some(dic) = SubscriptionTypeStorage::<A, TSubscriber>::subscribers().try_get_value(target) {
            let sub = dic.subscriptions.borrow().get(event_name).cloned();
            if let Some(sub) = sub {
                sub.remove(subscriber);
            }
        }
    }
}

thread_local! {
    /// The storage of each pair of event arguments and subscriber types.
    static SUBSCRIPTION_TYPE_STORAGE: RefCell<HashMap<TypeId, Box<dyn Any>>> = RefCell::new(HashMap::new());
}

/// The subscriptions of one pair of event arguments and subscriber types, by
/// target address (the managed conditional weak table). An entry keeps the
/// address of its target from being reused; entries of targets that are
/// gone are swept when the table grows.
struct SubscriptionTypeStorage<A: WeakEventArgs + ?Sized, TSubscriber: 'static> {
    subscribers: RefCell<HashMap<usize, Rc<SubscriptionDic<A, TSubscriber>>>>,
    sweep_at: Cell<usize>,
}

const MIN_SWEEP: usize = 16;

impl<A: WeakEventArgs + ?Sized, TSubscriber: 'static> SubscriptionTypeStorage<A, TSubscriber> {
    fn subscribers() -> Rc<Self> {
        SUBSCRIPTION_TYPE_STORAGE.with(|storage| {
            storage
                .borrow_mut()
                .entry(TypeId::of::<fn(&A) -> TSubscriber>())
                .or_insert_with(|| {
                    Box::new(Rc::new(Self { subscribers: RefCell::new(HashMap::new()), sweep_at: Cell::new(MIN_SWEEP) }))
                })
                .downcast_ref::<Rc<Self>>()
                .expect("the storage of a type pair holds that pair")
                .clone()
        })
    }

    fn try_get_value(&self, target: &impl WeakEventSender) -> Option<Rc<SubscriptionDic<A, TSubscriber>>> {
        self.subscribers.borrow().get(&target.sender_address()).cloned()
    }

    fn get_or_create_value<TTarget: WeakEventSender>(&self, target: &TTarget) -> Rc<SubscriptionDic<A, TSubscriber>> {
        if let Some(dic) = self.try_get_value(target) {
            return dic;
        }
        let weak_target = target.downgrade_sender();
        let dic = Rc::new(SubscriptionDic {
            subscriptions: RefCell::new(HashMap::new()),
            target_alive: Box::new(move || TTarget::upgrade_sender(&weak_target).is_some()),
        });
        let mut subscribers = self.subscribers.borrow_mut();
        subscribers.insert(target.sender_address(), dic.clone());
        if subscribers.len() >= self.sweep_at.get() {
            subscribers.retain(|_, dic| (dic.target_alive)());
            self.sweep_at.set((subscribers.len() * 2).max(MIN_SWEEP));
        }
        dic
    }
}

/// The subscriptions to the events of one target, by event name.
struct SubscriptionDic<A: WeakEventArgs + ?Sized, TSubscriber: 'static> {
    subscriptions: RefCell<HashMap<String, Rc<Subscription<A, TSubscriber>>>>,
    target_alive: Box<dyn Fn() -> bool>,
}

struct Descriptor<A: WeakEventArgs + ?Sized, TSubscriber> {
    subscriber: Option<Weak<TSubscriber>>,
    caller: Option<WeakEventHandlerMethod<A, TSubscriber>>,
}

impl<A: WeakEventArgs + ?Sized, TSubscriber> Default for Descriptor<A, TSubscriber> {
    fn default() -> Self {
        Self { subscriber: None, caller: None }
    }
}

impl<A: WeakEventArgs + ?Sized, TSubscriber> Clone for Descriptor<A, TSubscriber> {
    fn clone(&self) -> Self {
        Self { subscriber: self.subscriber.clone(), caller: self.caller }
    }
}

struct Subscription<A: WeakEventArgs + ?Sized, TSubscriber: 'static> {
    sdic: Weak<SubscriptionDic<A, TSubscriber>>,
    event_name: String,
    remove_handler: RefCell<Option<Box<dyn FnOnce()>>>,
    data: RefCell<Vec<Descriptor<A, TSubscriber>>>,
    count: Cell<usize>,
}

impl<A: WeakEventArgs + ?Sized, TSubscriber: 'static> Subscription<A, TSubscriber> {
    fn new<TTarget>(sdic: &Rc<SubscriptionDic<A, TSubscriber>>, target: &TTarget, event_name: &str) -> Rc<Self>
    where
        TTarget: WeakEventSender + Deref,
        TTarget::Target: WeakEventHandlerTarget<A>,
    {
        let this = Rc::new(Self {
            sdic: Rc::downgrade(sdic),
            event_name: event_name.to_string(),
            remove_handler: RefCell::new(None),
            data: RefCell::new(vec![Descriptor::default(); 2]),
            count: Cell::new(0),
        });

        // The target holds the handler; the table holds the subscription.
        let weak = Rc::downgrade(&this);
        let delegate: Rc<WeakEventHandler<A>> = Rc::new(move |sender: Option<&dyn Any>, event_args: &A::Args<'_>| {
            if let Some(this) = weak.upgrade() {
                this.on_event(sender, event_args);
            }
        });
        match (**target).add_event_handler(event_name, delegate) {
            Some(remove_handler) => *this.remove_handler.borrow_mut() = Some(remove_handler),
            None => panic!("The event {event_name} was not found on {}.", std::any::type_name::<TTarget::Target>()),
        }
        this
    }

    fn destroy(&self) {
        let remove_handler = self.remove_handler.borrow_mut().take();
        if let Some(remove_handler) = remove_handler {
            remove_handler();
        }
        if let Some(sdic) = self.sdic.upgrade() {
            sdic.subscriptions.borrow_mut().remove(&self.event_name);
        }
    }

    fn add(&self, subscriber: &Rc<TSubscriber>, method: WeakEventHandlerMethod<A, TSubscriber>) {
        self.compact(true);
        let mut data = self.data.borrow_mut();
        if self.count.get() == data.len() {
            // Extend capacity
            let len = data.len();
            data.resize(len * 2, Descriptor::default());
        }

        data[self.count.get()] = Descriptor { caller: Some(method), subscriber: Some(Rc::downgrade(subscriber)) };
        self.count.set(self.count.get() + 1);
    }

    fn remove(&self, subscriber: &Rc<TSubscriber>) {
        let mut removed = false;

        {
            let mut data = self.data.borrow_mut();
            for c in 0..self.count.get() {
                let is_subscriber = data[c]
                    .subscriber
                    .as_ref()
                    .and_then(Weak::upgrade)
                    .is_some_and(|instance| Rc::ptr_eq(&instance, subscriber));
                if is_subscriber {
                    data[c] = Descriptor::default();
                    removed = true;
                }
            }
        }

        if removed {
            self.compact(false);
        }
    }

    fn compact(&self, prevent_destroy: bool) {
        let mut empty: Option<usize> = None;
        {
            let mut data = self.data.borrow_mut();
            for c in 0..self.count.get() {
                let r = data[c].clone();

                let target = r.subscriber.as_ref().and_then(Weak::upgrade);

                // Mark current index as first empty
                if target.is_none() && empty.is_none() {
                    empty = Some(c);
                }
                // If current element isn't null and we have an empty one
                if let (Some(_), Some(e)) = (&target, empty) {
                    data[c] = Descriptor::default();
                    data[e] = r;
                    empty = Some(e + 1);
                }
            }
        }
        if let Some(empty) = empty {
            self.count.set(empty);
        }
        if self.count.get() == 0 && !prevent_destroy {
            self.destroy();
        }
    }

    fn on_event(&self, sender: Option<&dyn Any>, event_args: &A::Args<'_>) {
        let mut need_compact = false;
        let mut c = 0;
        while c < self.count.get() {
            let descriptor = self.data.borrow()[c].clone();
            let r = descriptor.subscriber.expect("a counted descriptor has a subscriber");
            match r.upgrade() {
                Some(sub) => (descriptor.caller.expect("a counted descriptor has a caller"))(&sub, sender, event_args),
                None => need_compact = true,
            }
            c += 1;
        }
        if need_compact {
            self.compact(false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utilities::EventArgs;

    struct EventSource {
        event: RefCell<Vec<Rc<WeakEventHandler<EventArgs>>>>,
        this: Weak<EventSource>,
    }

    impl EventSource {
        fn new() -> Rc<Self> {
            Rc::new_cyclic(|this| Self { event: RefCell::new(Vec::new()), this: this.clone() })
        }

        fn fire(&self) {
            let this = self.this.upgrade();
            let handlers = self.event.borrow().clone();
            for handler in handlers {
                handler(this.as_ref().map(|this| this as &dyn Any), &EventArgs::EMPTY);
            }
        }
    }

    impl WeakEventHandlerTarget<EventArgs> for EventSource {
        fn add_event_handler(
            &self,
            event_name: &str,
            handler: Rc<WeakEventHandler<EventArgs>>,
        ) -> Option<Box<dyn FnOnce()>> {
            if event_name != "Event" {
                return None;
            }
            self.event.borrow_mut().push(handler.clone());
            let this = self.this.clone();
            Some(Box::new(move || {
                if let Some(this) = this.upgrade() {
                    let mut event = this.event.borrow_mut();
                    if let Some(index) = event.iter().rposition(|h| Rc::ptr_eq(h, &handler)) {
                        event.remove(index);
                    }
                }
            }))
        }
    }

    struct Subscriber {
        on_event: Option<Box<dyn Fn()>>,
    }

    impl Subscriber {
        fn new(on_event: Box<dyn Fn()>) -> Rc<Self> {
            Rc::new(Self { on_event: Some(on_event) })
        }

        fn on_event(&self, _sender: Option<&dyn Any>, _ev: &EventArgs) {
            if let Some(on_event) = &self.on_event {
                on_event();
            }
        }
    }

    #[test]
    fn event_should_be_passed_to_subscriber() {
        let handled = Rc::new(Cell::new(false));
        let h = handled.clone();
        let subscriber = Subscriber::new(Box::new(move || h.set(true)));
        let source = EventSource::new();
        WeakEventHandlerManager::subscribe::<_, EventArgs, Subscriber>(&source, "Event", &subscriber, Subscriber::on_event);
        source.fire();
        assert!(handled.get());
    }

    #[test]
    fn event_should_not_be_raised_after_unsubscribe() {
        let handled = Rc::new(Cell::new(false));
        let h = handled.clone();
        let subscriber = Subscriber::new(Box::new(move || h.set(true)));
        let source = EventSource::new();
        WeakEventHandlerManager::subscribe::<_, EventArgs, Subscriber>(&source, "Event", &subscriber, Subscriber::on_event);

        WeakEventHandlerManager::unsubscribe::<EventArgs, Subscriber>(&source, "Event", &subscriber);

        source.fire();

        assert!(!handled.get());
    }

    #[test]
    fn event_handler_should_not_be_kept_alive() {
        let handled = Rc::new(Cell::new(false));
        let source = EventSource::new();
        let h = handled.clone();
        add_collectable_subscriber(&source, "Event", Box::new(move || h.set(true)));
        source.fire();
        assert!(!handled.get());
    }

    fn add_collectable_subscriber(source: &Rc<EventSource>, name: &str, func: Box<dyn Fn()>) {
        WeakEventHandlerManager::subscribe::<_, EventArgs, Subscriber>(source, name, &Subscriber::new(func), Subscriber::on_event);
    }
}
