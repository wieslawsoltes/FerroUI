//! `FromEventPattern` (`Linq/Observable/FromEventPattern.cs`), the delegate forms of
//! `QueryLanguage.Events.cs`.
//!
//! The handler of an event pattern takes the sender and the arguments of the event (upstream's
//! `EventHandler<TEventArgs>`, with a sender of the type `S` where upstream has an `object`); the
//! events are those of FerroUI, as for [`from_event`](crate::linq::Observable::from_event).
//!
//! Not ported: the forms that find the event by its name (`FromEventPattern(object target, string
//! eventName)`, `FromEventPattern(Type type, string eventName)` and their typed and scheduled
//! overloads), which need reflection; and the forms that make a delegate of the type `TDelegate`
//! from the handler through reflection (`FromEventPattern<TDelegate, TEventArgs>(Action<TDelegate>,
//! Action<TDelegate>)`, `FromEventPattern<TDelegate, TSender, TEventArgs>(Action<TDelegate>,
//! Action<TDelegate>)`): the handler of the port is the closure itself, which
//! [`from_event_pattern`](crate::linq::Observable::from_event_pattern) hands to the event, and which
//! the conversion overloads turn into the handler type of the event.

use super::event_pattern::EventPattern;
use super::from_event::{classic_add_handler, event_producer, scheduler_for_current_context};
use crate::concurrency::IScheduler;
use crate::linq::Observable;
use ferroui_system::{IObservable, ObservableError};
use std::rc::Rc;

/// The handler of `FromEventPattern.Impl<TDelegate, TEventArgs>.GetHandler`: the action that
/// notifies an event pattern, as a handler of a sender and arguments.
fn pattern_handler<S: 'static, A: 'static>(on_next: Rc<dyn Fn(EventPattern<S, A>)>) -> Rc<dyn Fn(S, A)> {
    Rc::new(move |sender: S, event_args: A| on_next(EventPattern::new(sender, event_args)))
}

impl Observable {
    /// The events of an event whose handlers take a sender and arguments, as [`EventPattern`]s
    /// (`FromEventPattern<TEventArgs>(Action<EventHandler<TEventArgs>>, Action<EventHandler<TEventArgs>>)`,
    /// and `FromEventPattern(Action<EventHandler>, Action<EventHandler>)`): `add_handler` adds a
    /// handler and returns its token, `remove_handler` removes the handler of a token. The
    /// subscriptions share one handler, added at the first subscription and removed when the last
    /// one is disposed.
    pub fn from_event_pattern<S: Clone + 'static, A: Clone + 'static, K: 'static>(
        add_handler: impl Fn(Rc<dyn Fn(S, A)>) -> K + 'static,
        remove_handler: impl Fn(K) + 'static,
    ) -> Rc<dyn IObservable<EventPattern<S, A>>> {
        Self::from_event_pattern_on(add_handler, remove_handler, scheduler_for_current_context())
    }

    /// [`from_event_pattern`](Self::from_event_pattern), which adds and removes the handler on
    /// `scheduler` (`FromEventPattern<TEventArgs>(Action<EventHandler<TEventArgs>>, Action<EventHandler<TEventArgs>>, IScheduler)`).
    pub fn from_event_pattern_on<S: Clone + 'static, A: Clone + 'static, K: 'static>(
        add_handler: impl Fn(Rc<dyn Fn(S, A)>) -> K + 'static,
        remove_handler: impl Fn(K) + 'static,
        scheduler: Rc<dyn IScheduler>,
    ) -> Rc<dyn IObservable<EventPattern<S, A>>> {
        Self::try_from_event_pattern_conversion_on(move |handler| Ok(handler), add_handler, remove_handler, scheduler)
    }

    /// The events of an event whose handlers are of the type `D`, which `conversion` makes from the
    /// handler of a sender and arguments, as [`EventPattern`]s
    /// (`FromEventPattern<TDelegate, TEventArgs>(Func<EventHandler<TEventArgs>, TDelegate>, Action<TDelegate>, Action<TDelegate>)`).
    pub fn from_event_pattern_conversion<D: 'static, S: Clone + 'static, A: Clone + 'static, K: 'static>(
        conversion: impl Fn(Rc<dyn Fn(S, A)>) -> D + 'static,
        add_handler: impl Fn(D) -> K + 'static,
        remove_handler: impl Fn(K) + 'static,
    ) -> Rc<dyn IObservable<EventPattern<S, A>>> {
        Self::from_event_pattern_conversion_on(conversion, add_handler, remove_handler, scheduler_for_current_context())
    }

    /// [`from_event_pattern_conversion`](Self::from_event_pattern_conversion) on `scheduler`
    /// (`FromEventPattern<TDelegate, TEventArgs>(Func<EventHandler<TEventArgs>, TDelegate>, Action<TDelegate>, Action<TDelegate>, IScheduler)`).
    pub fn from_event_pattern_conversion_on<D: 'static, S: Clone + 'static, A: Clone + 'static, K: 'static>(
        conversion: impl Fn(Rc<dyn Fn(S, A)>) -> D + 'static,
        add_handler: impl Fn(D) -> K + 'static,
        remove_handler: impl Fn(K) + 'static,
        scheduler: Rc<dyn IScheduler>,
    ) -> Rc<dyn IObservable<EventPattern<S, A>>> {
        Self::try_from_event_pattern_conversion_on(move |handler| Ok(conversion(handler)), add_handler, remove_handler, scheduler)
    }

    /// [`from_event_pattern_conversion`](Self::from_event_pattern_conversion) with a conversion
    /// that may fail; a failure ends the subscription with its error (a conversion that throws).
    pub fn try_from_event_pattern_conversion<D: 'static, S: Clone + 'static, A: Clone + 'static, K: 'static>(
        conversion: impl Fn(Rc<dyn Fn(S, A)>) -> Result<D, ObservableError> + 'static,
        add_handler: impl Fn(D) -> K + 'static,
        remove_handler: impl Fn(K) + 'static,
    ) -> Rc<dyn IObservable<EventPattern<S, A>>> {
        Self::try_from_event_pattern_conversion_on(conversion, add_handler, remove_handler, scheduler_for_current_context())
    }

    /// [`try_from_event_pattern_conversion`](Self::try_from_event_pattern_conversion) on
    /// `scheduler`.
    pub fn try_from_event_pattern_conversion_on<D: 'static, S: Clone + 'static, A: Clone + 'static, K: 'static>(
        conversion: impl Fn(Rc<dyn Fn(S, A)>) -> Result<D, ObservableError> + 'static,
        add_handler: impl Fn(D) -> K + 'static,
        remove_handler: impl Fn(K) + 'static,
        scheduler: Rc<dyn IScheduler>,
    ) -> Rc<dyn IObservable<EventPattern<S, A>>> {
        event_producer(
            Rc::new(move |on_next: Rc<dyn Fn(EventPattern<S, A>)>| conversion(pattern_handler(on_next))),
            classic_add_handler(add_handler, remove_handler),
            scheduler,
        )
    }
}
