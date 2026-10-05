use super::{EventArgs, IWeakEventSubscriber, WeakHashList};
use crate::threading::{Dispatcher, DispatcherPriority};
use crate::{ObjectType, Ref, WeakRef};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};

/// The arguments of an event that can be subscribed to weakly.
///
/// The arguments type of the managed event is a type parameter of
/// [`WeakEvent`] and [`IWeakEventSubscriber`]. Event arguments are passed by
/// reference and may themselves borrow data for the duration of the
/// notification ([`FerroPropertyChangedEventArgs`](crate::FerroPropertyChangedEventArgs)),
/// so the type parameter names the arguments type and `Args<'a>` is the
/// type that is passed.
pub trait WeakEventArgs: 'static {
    type Args<'a>: ?Sized;
}

impl WeakEventArgs for EventArgs {
    type Args<'a> = EventArgs;
}

/// Property change notifications of a model object: the name of the property.
impl WeakEventArgs for str {
    type Args<'a> = str;
}

/// A handle to an object that raises an event subscribed to through a
/// [`WeakEvent`]: the managed reference type of the sender.
///
/// A [`WeakEvent`] identifies a sender by the address of the object the
/// handle refers to and holds it weakly.
pub trait WeakEventSender: Clone + 'static {
    /// The weak form of the handle.
    type Weak: Clone + 'static;

    fn downgrade_sender(&self) -> Self::Weak;

    fn upgrade_sender(weak: &Self::Weak) -> Option<Self>;

    /// The address of the object the handle refers to.
    fn sender_address(&self) -> usize;
}

impl<T: ?Sized + 'static> WeakEventSender for Rc<T> {
    type Weak = Weak<T>;

    fn downgrade_sender(&self) -> Weak<T> {
        Rc::downgrade(self)
    }

    fn upgrade_sender(weak: &Weak<T>) -> Option<Self> {
        weak.upgrade()
    }

    fn sender_address(&self) -> usize {
        Rc::as_ptr(self) as *const () as usize
    }
}

impl<T: ObjectType> WeakEventSender for Ref<T> {
    type Weak = WeakRef<T>;

    fn downgrade_sender(&self) -> WeakRef<T> {
        self.downgrade()
    }

    fn upgrade_sender(weak: &WeakRef<T>) -> Option<Self> {
        weak.upgrade()
    }

    fn sender_address(&self) -> usize {
        &**self as *const T as *const () as usize
    }
}

/// The signature of the handler a [`WeakEvent`] adds to the event of a
/// sender: the managed `EventHandler<TEventArgs>` (sender and arguments).
pub type WeakEventHandler<A> = dyn for<'s, 'r, 'a> Fn(Option<&'s dyn Any>, &'r <A as WeakEventArgs>::Args<'a>);

/// The function a [`WeakEvent`] subscribes to the event of a sender with:
/// it adds the handler and returns the action that removes it.
type SubscribeFunc<S, A> = dyn Fn(&S, Rc<WeakEventHandler<A>>) -> Box<dyn FnOnce()>;

/// Manages subscriptions to events using weak listeners.
///
/// One instance describes one event (usually a thread-local static, see
/// [`WeakEvents`](super::WeakEvents)). For each sender it adds a single
/// handler to the event, while there are subscribers, and forwards the
/// events to the subscribers that are still alive. The sender holds the
/// subscription; neither the sender nor the subscription holds the
/// subscribers.
pub struct WeakEvent<S: WeakEventSender, A: WeakEventArgs + ?Sized> {
    this: Weak<WeakEvent<S, A>>,
    subscribe: Box<SubscribeFunc<S, A>>,
    subscriptions: RefCell<SubscriptionTable<S, A>>,
}

/// The subscriptions of a [`WeakEvent`] by sender address (the managed
/// conditional weak table). A subscription lives as long as the handler it
/// added to its sender; an entry whose subscription is gone is replaced, and
/// such entries are swept when the table grows.
struct SubscriptionTable<S: WeakEventSender, A: WeakEventArgs + ?Sized> {
    entries: HashMap<usize, Weak<Subscription<S, A>>>,
    sweep_at: usize,
}

const MIN_SWEEP: usize = 16;

impl<S: WeakEventSender, A: WeakEventArgs + ?Sized> WeakEvent<S, A> {
    fn new(subscribe: Box<SubscribeFunc<S, A>>) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            subscribe,
            subscriptions: RefCell::new(SubscriptionTable { entries: HashMap::new(), sweep_at: MIN_SWEEP }),
        })
    }

    /// Registers an event through the functions that add a handler to it
    /// and remove the handler from it.
    ///
    /// The managed original removes the handler through the sender it
    /// captured; here the removal holds the sender weakly (a strong
    /// reference would keep it alive through its own subscription), so the
    /// handler is not removed from a sender that no longer exists.
    pub fn register<Sub, Unsub>(subscribe: Sub, unsubscribe: Unsub) -> Rc<Self>
    where
        Sub: Fn(&S, &Rc<WeakEventHandler<A>>) + 'static,
        Unsub: Fn(&S, &Rc<WeakEventHandler<A>>) + 'static,
    {
        let unsubscribe = Rc::new(unsubscribe);
        Self::new(Box::new(move |t: &S, s: Rc<WeakEventHandler<A>>| {
            subscribe(t, &s);
            let t = t.downgrade_sender();
            let unsubscribe = unsubscribe.clone();
            Box::new(move || {
                if let Some(t) = S::upgrade_sender(&t) {
                    unsubscribe(&t, &s);
                }
            })
        }))
    }

    /// Registers an event through a function that adds a handler to it and
    /// returns the action that removes the handler.
    ///
    /// The action must not hold the sender strongly: the sender holds the
    /// subscription, and the subscription holds the action.
    pub fn register_returning_unsubscribe<Sub>(subscribe: Sub) -> Rc<Self>
    where
        Sub: Fn(&S, Rc<WeakEventHandler<A>>) -> Box<dyn FnOnce()> + 'static,
    {
        Self::new(Box::new(subscribe))
    }

    pub fn subscribe(&self, target: &S, subscriber: &Rc<dyn IWeakEventSubscriber<A>>) {
        loop {
            let subscription = self.get_value(target);
            if subscription.add(subscriber) {
                break;
            }
        }
    }

    pub fn unsubscribe(&self, target: &S, subscriber: &Rc<dyn IWeakEventSubscriber<A>>) {
        if let Some(subscription) = self.try_get_value(target) {
            subscription.remove(subscriber);
        }
    }

    fn try_get_value(&self, target: &S) -> Option<Rc<Subscription<S, A>>> {
        self.subscriptions.borrow().entries.get(&target.sender_address()).and_then(Weak::upgrade)
    }

    fn get_value(&self, target: &S) -> Rc<Subscription<S, A>> {
        if let Some(subscription) = self.try_get_value(target) {
            return subscription;
        }
        let subscription = self.create_subscription(target);
        let mut table = self.subscriptions.borrow_mut();
        table.entries.insert(subscription.address, Rc::downgrade(&subscription));
        if table.entries.len() >= table.sweep_at {
            table.entries.retain(|_, subscription| subscription.strong_count() > 0);
            table.sweep_at = (table.entries.len() * 2).max(MIN_SWEEP);
        }
        subscription
    }

    fn create_subscription(&self, key: &S) -> Rc<Subscription<S, A>> {
        Subscription::new(self.this.upgrade().expect("a weak event is used through its handle"), key)
    }
}

impl<S: WeakEventSender> WeakEvent<S, EventArgs> {
    /// Registers an event whose handlers take the plain event arguments,
    /// through the functions that add a handler to it and remove the
    /// handler from it.
    pub fn register_plain<Sub, Unsub>(subscribe: Sub, unsubscribe: Unsub) -> Rc<Self>
    where
        Sub: Fn(&S, &Rc<WeakEventHandler<EventArgs>>) + 'static,
        Unsub: Fn(&S, &Rc<WeakEventHandler<EventArgs>>) + 'static,
    {
        let unsubscribe = Rc::new(unsubscribe);
        Self::register_returning_unsubscribe(move |s: &S, h: Rc<WeakEventHandler<EventArgs>>| {
            // The handler belongs to the sender, so it refers to it weakly.
            let sender = s.downgrade_sender();
            let handler: Rc<WeakEventHandler<EventArgs>> = Rc::new(move |_: Option<&dyn Any>, e: &EventArgs| {
                if let Some(s) = S::upgrade_sender(&sender) {
                    h(Some(&s as &dyn Any), e);
                }
            });
            subscribe(s, &handler);
            let s = s.downgrade_sender();
            let unsubscribe = unsubscribe.clone();
            Box::new(move || {
                if let Some(s) = S::upgrade_sender(&s) {
                    unsubscribe(&s, &handler);
                }
            })
        })
    }
}

/// The subscription of a [`WeakEvent`] to the event of one sender.
struct Subscription<S: WeakEventSender, A: WeakEventArgs + ?Sized> {
    this: Weak<Subscription<S, A>>,
    ev: Rc<WeakEvent<S, A>>,
    target: S::Weak,
    address: usize,
    list: RefCell<WeakHashList<dyn IWeakEventSubscriber<A>>>,
    unsubscribe: RefCell<Option<Box<dyn FnOnce()>>>,
    compact_scheduled: Cell<bool>,
    destroyed: Cell<bool>,
}

impl<S: WeakEventSender, A: WeakEventArgs + ?Sized> Subscription<S, A> {
    fn new(ev: Rc<WeakEvent<S, A>>, target: &S) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            ev,
            target: target.downgrade_sender(),
            address: target.sender_address(),
            list: RefCell::new(WeakHashList::new()),
            unsubscribe: RefCell::new(None),
            compact_scheduled: Cell::new(false),
            destroyed: Cell::new(false),
        })
    }

    fn destroy(&self) {
        if self.destroyed.get() {
            return;
        }
        self.destroyed.set(true);
        let unsubscribe = self.unsubscribe.borrow_mut().take();
        if let Some(unsubscribe) = unsubscribe {
            unsubscribe();
        }
        let mut table = self.ev.subscriptions.borrow_mut();
        if table.entries.get(&self.address).is_some_and(|entry| std::ptr::eq(entry.as_ptr(), self)) {
            table.entries.remove(&self.address);
        }
    }

    fn add(&self, s: &Rc<dyn IWeakEventSubscriber<A>>) -> bool {
        if self.destroyed.get() {
            return false;
        }

        if self.unsubscribe.borrow().is_none() {
            let Some(target) = S::upgrade_sender(&self.target) else { return false };
            // The handler is held by the sender and holds the subscription.
            let this = self.this.upgrade().expect("a subscription is used through its handle");
            let handler: Rc<WeakEventHandler<A>> =
                Rc::new(move |sender: Option<&dyn Any>, event_args: &A::Args<'_>| this.on_event(sender, event_args));
            let unsubscribe = (self.ev.subscribe)(&target, handler);
            *self.unsubscribe.borrow_mut() = Some(unsubscribe);
        }
        self.list.borrow_mut().add(s);
        true
    }

    fn remove(&self, s: &Rc<dyn IWeakEventSubscriber<A>>) {
        if self.destroyed.get() {
            return;
        }

        let mut list = self.list.borrow_mut();
        list.remove(s);
        if list.is_empty() {
            drop(list);
            self.destroy();
        } else if list.need_compact() && self.compact_scheduled.get() {
            drop(list);
            self.schedule_compact();
        }
    }

    fn schedule_compact(&self) {
        if self.compact_scheduled.get() || self.destroyed.get() {
            return;
        }
        self.compact_scheduled.set(true);
        let Some(this) = self.this.upgrade() else { return };
        // The sender, and so the subscription, belongs to this thread.
        Dispatcher::current_dispatcher().post_local(move || this.compact(), DispatcherPriority::BACKGROUND);
    }

    fn compact(&self) {
        if self.destroyed.get() {
            return;
        }
        if !self.compact_scheduled.get() {
            return;
        }
        self.compact_scheduled.set(false);
        let mut list = self.list.borrow_mut();
        list.compact();
        if list.is_empty() {
            drop(list);
            self.destroy();
        }
    }

    fn on_event(&self, _sender: Option<&dyn Any>, event_args: &A::Args<'_>) {
        let _keep_alive = self.this.upgrade();
        let alive = self.list.borrow_mut().get_alive(None);
        let Some(alive) = alive else {
            self.destroy();
            return;
        };

        let target = S::upgrade_sender(&self.target);
        let sender = target.as_ref().map(|target| target as &dyn Any);
        let ev: &dyn Any = &*self.ev;
        for item in &alive {
            item.on_event(sender, ev, event_args);
        }

        WeakHashList::<dyn IWeakEventSubscriber<A>>::return_to_shared_pool(alive);
        if self.list.borrow().need_compact() && !self.compact_scheduled.get() {
            self.schedule_compact();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EventSource {
        this: Weak<EventSource>,
        event: RefCell<Vec<Rc<WeakEventHandler<EventArgs>>>>,
    }

    impl EventSource {
        fn new() -> Rc<Self> {
            Rc::new_cyclic(|this| Self { this: this.clone(), event: RefCell::new(Vec::new()) })
        }

        fn add_event(&self, handler: &Rc<WeakEventHandler<EventArgs>>) {
            self.event.borrow_mut().push(handler.clone());
        }

        fn remove_event(&self, handler: &Rc<WeakEventHandler<EventArgs>>) {
            let mut event = self.event.borrow_mut();
            if let Some(index) = event.iter().rposition(|h| Rc::ptr_eq(h, handler)) {
                event.remove(index);
            }
        }

        fn fire(&self) {
            let this = self.this.upgrade();
            let handlers = self.event.borrow().clone();
            for handler in handlers {
                handler(this.as_ref().map(|this| this as &dyn Any), &EventArgs::EMPTY);
            }
        }

        fn weak_ev() -> Rc<WeakEvent<Rc<EventSource>, EventArgs>> {
            thread_local! {
                static WEAK_EV: Rc<WeakEvent<Rc<EventSource>, EventArgs>> = WeakEvent::register_plain(
                    |t: &Rc<EventSource>, s| t.add_event(s),
                    |t: &Rc<EventSource>, s| t.remove_event(s));
            }
            WEAK_EV.with(Rc::clone)
        }
    }

    struct Subscriber {
        on_event: Option<Box<dyn Fn()>>,
    }

    impl Subscriber {
        fn new(on_event: Option<Box<dyn Fn()>>) -> Rc<dyn IWeakEventSubscriber<EventArgs>> {
            Rc::new(Self { on_event })
        }
    }

    impl IWeakEventSubscriber<EventArgs> for Subscriber {
        fn on_event(&self, _sender: Option<&dyn Any>, _ev: &dyn Any, _args: &EventArgs) {
            if let Some(on_event) = &self.on_event {
                on_event();
            }
        }
    }

    #[test]
    fn event_should_be_passed_to_subscriber() {
        let handled = Rc::new(Cell::new(false));
        let h = handled.clone();
        let subscriber = Subscriber::new(Some(Box::new(move || h.set(true))));
        let source = EventSource::new();
        EventSource::weak_ev().subscribe(&source, &subscriber);

        source.fire();
        assert!(handled.get());
    }

    #[test]
    fn event_handler_should_not_be_kept_alive() {
        let handled = Rc::new(Cell::new(false));
        let source = EventSource::new();
        let h = handled.clone();
        add_subscriber(&source, Box::new(move || h.set(true)));
        source.fire();
        assert!(!handled.get());
    }

    fn add_subscriber(source: &Rc<EventSource>, func: Box<dyn Fn()>) {
        EventSource::weak_ev().subscribe(source, &Subscriber::new(Some(func)));
    }
}
