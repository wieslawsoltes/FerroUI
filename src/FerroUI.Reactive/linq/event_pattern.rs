//! `EventPattern<TSender, TEventArgs>` (`EventPattern.cs`, `IEventPattern.cs`), the event sources
//! that `ToEvent` and `ToEventPattern` return (`IEventSource.cs`, `IEventPatternSource.cs`), and the
//! base of the event pattern sources (`EventPatternSourceBase.cs`, `EventPatternSource.cs`).
//!
//! Deviation (DEVIATIONS.md, Reactive): an event of upstream is a delegate that `+=` adds and `-=`
//! removes, known by its value. The events of the port are those of FerroUI
//! (`ferroui_system::events::Event`): `add` takes a handler (`Rc<dyn Fn(..)>`) and returns the token
//! that `remove` takes. Upstream keeps a stack of subscriptions per delegate, so that a delegate
//! added twice is removed once per `-=`; each `add` of the port has a token of its own. The sender
//! of upstream's `EventPattern<TEventArgs>` is an `object`; the port has the sender's type as a
//! parameter everywhere (`EventPattern<S, A>`, `IEventPatternSource<S, A>`).

use crate::linq::ObservableExt;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

/// The sender and the arguments of an event (`EventPattern<TSender, TEventArgs>` and
/// `IEventPattern<TSender, TEventArgs>`). Two patterns are equal when their senders and their
/// arguments are.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct EventPattern<S, A> {
    /// The sender of the event (`Sender`).
    pub sender: S,
    /// The arguments of the event (`EventArgs`).
    pub event_args: A,
}

impl<S, A> EventPattern<S, A> {
    /// The pattern of an event of `sender` with `event_args` (`EventPattern(TSender, TEventArgs)`).
    pub fn new(sender: S, event_args: A) -> Self {
        Self { sender, event_args }
    }

    /// The sender and the arguments (`Deconstruct`).
    pub fn into_parts(self) -> (S, A) {
        (self.sender, self.event_args)
    }
}

/// A value that is an [`EventPattern`]: implemented for `EventPattern<S, A>`, so that
/// `to_event_pattern` is a method of the sequences of event patterns (upstream's `ToEventPattern`
/// extends `IObservable<EventPattern<TEventArgs>>`).
pub trait EventPatternValue: 'static {
    /// The type of the sender.
    type Sender: 'static;
    /// The type of the arguments.
    type EventArgs: 'static;

    /// The pattern.
    fn into_event_pattern(self) -> EventPattern<Self::Sender, Self::EventArgs>;
}

impl<S: 'static, A: 'static> EventPatternValue for EventPattern<S, A> {
    type Sender = S;
    type EventArgs = A;

    fn into_event_pattern(self) -> EventPattern<S, A> {
        self
    }
}

/// An event that notifies the values of a sequence (`IEventSource<T>`), returned by `to_event`.
pub trait IEventSource<T> {
    /// Adds a handler of the values (`OnNext += handler`); returns the token that removes it. The
    /// handler is subscribed to the sequence until it is removed or the sequence ends.
    fn add(&self, handler: Rc<dyn Fn(T)>) -> u64;

    /// Removes the handler of `token` (`OnNext -= handler`) and disposes its subscription. False
    /// when the token is unknown: removed already, or its subscription ended with the sequence.
    fn remove(&self, token: u64) -> bool;
}

/// An event that notifies the event patterns of a sequence as a sender and arguments
/// (`IEventPatternSource<TEventArgs>`), returned by `to_event_pattern`.
pub trait IEventPatternSource<S, A> {
    /// Adds a handler of the events (`OnNext += handler`); returns the token that removes it.
    fn add(&self, handler: Rc<dyn Fn(S, A)>) -> u64;

    /// Removes the handler of `token` (`OnNext -= handler`). False when the token is unknown.
    fn remove(&self, token: u64) -> bool;
}

/// The subscriptions of the handlers of an event source, by token (`Dictionary<Delegate,
/// Stack<IDisposable>> _subscriptions`).
#[derive(Default)]
pub(crate) struct HandlerSubscriptions {
    subscriptions: RefCell<BTreeMap<u64, Rc<dyn IDisposable>>>,
    next_token: Cell<u64>,
}

impl HandlerSubscriptions {
    /// The token of a new handler.
    pub(crate) fn next_token(&self) -> u64 {
        let token = self.next_token.get() + 1;
        self.next_token.set(token);
        token
    }

    /// `Add(Delegate, IDisposable)`.
    pub(crate) fn add(&self, token: u64, subscription: Rc<dyn IDisposable>) {
        self.subscriptions.borrow_mut().insert(token, subscription);
    }

    /// `Remove(Delegate)`: the subscription is disposed after it has left the table.
    pub(crate) fn remove(&self, token: u64) -> bool {
        let subscription = self.subscriptions.borrow_mut().remove(&token);
        match subscription {
            Some(subscription) => {
                subscription.dispose();
                true
            }
            None => false,
        }
    }
}

type InvokeHandler<S, A> = Rc<dyn Fn(&dyn Fn(S, A), EventPattern<S, A>)>;

/// The base of an event that notifies the event patterns of a sequence
/// (`EventPatternSourceBase<TSender, TEventArgs>`): each handler added subscribes to the sequence,
/// and `invoke_handler` hands it each pattern; the handler leaves when the sequence ends, and an
/// error of the sequence panics after it has left (upstream rethrows the error).
pub struct EventPatternSourceBase<S, A> {
    source: Rc<dyn IObservable<EventPattern<S, A>>>,
    invoke_handler: InvokeHandler<S, A>,
    subscriptions: Rc<HandlerSubscriptions>,
}

impl<S: 'static, A: 'static> EventPatternSourceBase<S, A> {
    /// The event of the patterns of `source`, which `invoke_handler` hands to a handler
    /// (`EventPatternSourceBase(IObservable<EventPattern<TSender, TEventArgs>>, Action<Action<TSender, TEventArgs>, EventPattern<TSender, TEventArgs>>)`).
    pub fn new(
        source: Rc<dyn IObservable<EventPattern<S, A>>>,
        invoke_handler: impl Fn(&dyn Fn(S, A), EventPattern<S, A>) + 'static,
    ) -> Self {
        Self { source, invoke_handler: Rc::new(invoke_handler), subscriptions: Rc::new(HandlerSubscriptions::default()) }
    }

    /// Adds a handler, `invoke` (`Add(Delegate, Action<TSender, TEventArgs>)`), subscribed to the
    /// sequence; returns the token that removes it.
    pub fn add(&self, invoke: Rc<dyn Fn(S, A)>) -> u64 {
        let token = self.subscriptions.next_token();
        let observer = Rc::new(PatternObserver {
            token,
            invoke,
            invoke_handler: self.invoke_handler.clone(),
            subscriptions: self.subscriptions.clone(),
            is_done: Cell::new(false),
            is_added: Cell::new(false),
            is_stopped: Cell::new(false),
        });
        let subscription = self.source.subscribe(observer.clone());
        observer.set_resource(subscription);
        token
    }

    /// Removes the handler of `token` and disposes its subscription (`Remove(Delegate)`). False when
    /// the token is unknown.
    pub fn remove(&self, token: u64) -> bool {
        self.subscriptions.remove(token)
    }
}

/// The observer of a handler (`EventPatternSourceBase<TSender, TEventArgs>.Observer`, an
/// `ObserverBase<EventPattern<TSender, TEventArgs>>`).
struct PatternObserver<S, A> {
    token: u64,
    invoke: Rc<dyn Fn(S, A)>,
    invoke_handler: InvokeHandler<S, A>,
    subscriptions: Rc<HandlerSubscriptions>,
    is_done: Cell<bool>,
    is_added: Cell<bool>,
    is_stopped: Cell<bool>,
}

impl<S, A> PatternObserver<S, A> {
    /// `Remove`: the handler leaves the table, or will not enter it when the sequence ends while it
    /// is subscribed.
    fn remove(&self) {
        if self.is_added.get() {
            self.subscriptions.remove(self.token);
        } else {
            self.is_done.set(true);
        }
    }

    /// `SetResource`.
    fn set_resource(&self, resource: Rc<dyn IDisposable>) {
        if !self.is_done.get() {
            self.subscriptions.add(self.token, resource);
            self.is_added.set(true);
        }
    }
}

impl<S: 'static, A: 'static> IObserver<EventPattern<S, A>> for PatternObserver<S, A> {
    fn on_next(&self, value: EventPattern<S, A>) {
        if !self.is_stopped.get() {
            (self.invoke_handler)(&*self.invoke, value);
        }
    }

    fn on_error(&self, error: ObservableError) {
        if !self.is_stopped.replace(true) {
            self.remove();
            crate::observer::throw(error);
        }
    }

    fn on_completed(&self) {
        if !self.is_stopped.replace(true) {
            self.remove();
        }
    }
}

/// The event of `ToEventPattern` (`EventPatternSource<TEventArgs>`): a handler receives the sender
/// and the arguments of each pattern.
struct EventPatternSource<S, A> {
    base: EventPatternSourceBase<S, A>,
}

impl<S: 'static, A: 'static> IEventPatternSource<S, A> for EventPatternSource<S, A> {
    fn add(&self, handler: Rc<dyn Fn(S, A)>) -> u64 {
        self.base.add(handler)
    }

    fn remove(&self, token: u64) -> bool {
        self.base.remove(token)
    }
}

/// `ToEventPattern` (`QueryLanguage.Conversions.cs`): the event of the patterns of `source`. (The
/// values of `source` are its patterns already; the projection only names their type.)
pub(crate) fn to_event_pattern<P: EventPatternValue>(source: Rc<dyn IObservable<P>>) -> Rc<dyn IEventPatternSource<P::Sender, P::EventArgs>> {
    let source = source.select(P::into_event_pattern);
    Rc::new(EventPatternSource {
        base: EventPatternSourceBase::new(source, |handler: &dyn Fn(P::Sender, P::EventArgs), pattern: EventPattern<P::Sender, P::EventArgs>| {
            let (sender, event_args) = pattern.into_parts();
            handler(sender, event_args);
        }),
    })
}
