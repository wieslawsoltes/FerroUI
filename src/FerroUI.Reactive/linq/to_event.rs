//! `ToEvent` and `ToEventPattern` (`Linq/QueryLanguage.Conversions.cs`), with the event that
//! `ToEvent` returns (`EventSource.cs`).
//!
//! The events are those of FerroUI: a handler is added with `add`, which returns the token that
//! `remove` takes (see [`IEventSource`](super::event_pattern::IEventSource)).

use super::event_pattern::{HandlerSubscriptions, IEventSource};
use crate::linq::ObservableExt;
use ferroui_system::IObservable;
use std::cell::Cell;
use std::rc::Rc;

/// The event of `ToEvent` (`EventSource<T>`): each handler added subscribes to the sequence and
/// receives its values; it leaves when the sequence ends, and an error of the sequence panics after
/// it has left (upstream rethrows the error).
struct EventSource<T> {
    source: Rc<dyn IObservable<T>>,
    subscriptions: Rc<HandlerSubscriptions>,
}

/// Whether the handler has entered the table, or the sequence ended while it was subscribed (the
/// `isAdded` and `isDone` of `EventSource<T>.OnNext.add`).
#[derive(Default)]
struct AddState {
    is_added: Cell<bool>,
    is_done: Cell<bool>,
}

impl<T: 'static> IEventSource<T> for EventSource<T> {
    fn add(&self, handler: Rc<dyn Fn(T)>) -> u64 {
        let token = self.subscriptions.next_token();
        let state = Rc::new(AddState::default());

        let remove: Rc<dyn Fn()> = {
            let (state, subscriptions) = (state.clone(), self.subscriptions.clone());
            Rc::new(move || {
                if state.is_added.get() {
                    subscriptions.remove(token);
                } else {
                    state.is_done.set(true);
                }
            })
        };

        // The `_invokeHandler` of `ToEvent` calls the handler with the value (`h(value)`; with
        // `Unit.Default` for a sequence of `Unit`, which is the value of a sequence of `()`).
        let on_error = remove.clone();
        let subscription = self.source.subscribe_all(
            move |value| handler(value),
            move |error| {
                on_error();
                crate::observer::throw(error);
            },
            move || remove(),
        );

        if !state.is_done.get() {
            self.subscriptions.add(token, subscription);
            state.is_added.set(true);
        }
        token
    }

    fn remove(&self, token: u64) -> bool {
        self.subscriptions.remove(token)
    }
}

pub(crate) fn to_event<T: 'static>(source: Rc<dyn IObservable<T>>) -> Rc<dyn IEventSource<T>> {
    Rc::new(EventSource { source, subscriptions: Rc::new(HandlerSubscriptions::default()) })
}

/// The methods of `ToEvent` and `ToEventPattern` (provided methods of
/// [`ObservableExt`](super::ObservableExt)).
macro_rules! to_event_methods {
    () => {
        /// The event of the values of the sequence (`ToEvent<TSource>(IObservable<TSource>)`, and
        /// `ToEvent(IObservable<Unit>)` for a sequence of `()`). Each handler added subscribes to the
        /// sequence.
        fn to_event(&self) -> std::rc::Rc<dyn crate::linq::event_pattern::IEventSource<T>> {
            crate::linq::to_event::to_event(self.observable())
        }

        /// The event of the patterns of a sequence of [`EventPattern`](crate::linq::event_pattern::EventPattern)s:
        /// a handler receives the sender and the arguments of each pattern
        /// (`ToEventPattern<TEventArgs>(IObservable<EventPattern<TEventArgs>>)`).
        fn to_event_pattern(
            &self,
        ) -> std::rc::Rc<
            dyn crate::linq::event_pattern::IEventPatternSource<
                <T as crate::linq::event_pattern::EventPatternValue>::Sender,
                <T as crate::linq::event_pattern::EventPatternValue>::EventArgs,
            >,
        >
        where
            T: crate::linq::event_pattern::EventPatternValue,
        {
            crate::linq::event_pattern::to_event_pattern(self.observable())
        }
    };
}
pub(crate) use to_event_methods;
