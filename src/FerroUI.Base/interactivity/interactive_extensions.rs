//! Extension methods for [`Interactive`].

use super::{IRoutedEventArgs, Interactive, RoutedEvent, RoutingStrategies};
use crate::reactive::{Disposable, IDisposable, IObservable, Observable};
use crate::Ref;
use std::rc::Rc;

impl Interactive {
    /// Adds a handler for a routed event (direct and bubble routes,
    /// unhandled events) and returns a disposable that removes the handler
    /// when disposed.
    pub fn add_disposable_handler<TEventArgs: IRoutedEventArgs>(
        &self,
        routed_event: &RoutedEvent<TEventArgs>,
        handler: impl Fn(&Interactive, &TEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        self.add_disposable_handler_with(routed_event, handler, Self::DEFAULT_ROUTES, false)
    }

    /// Adds a handler for a routed event and returns a disposable that
    /// removes the handler when disposed.
    pub fn add_disposable_handler_with<TEventArgs: IRoutedEventArgs>(
        &self,
        routed_event: &RoutedEvent<TEventArgs>,
        handler: impl Fn(&Interactive, &TEventArgs) + 'static,
        routes: RoutingStrategies,
        handled_events_too: bool,
    ) -> Rc<dyn IDisposable> {
        let token = self.add_handler_with(routed_event, handler, routes, handled_events_too);
        let instance = self.to_ref();
        let routed_event = routed_event.as_routed_event();

        Disposable::create(move || {
            instance.remove_handler(&routed_event, token);
        })
    }

    /// The interactive parent of the object for bubbling and tunneling
    /// events.
    pub fn get_interactive_parent(&self) -> Option<Ref<Interactive>> {
        self.interactive_parent()
    }

    /// Gets an observable for a routed event (direct and bubble routes,
    /// unhandled events).
    ///
    /// See [`get_observable_with`](Self::get_observable_with).
    pub fn get_observable<TEventArgs: IRoutedEventArgs + Clone>(
        &self,
        routed_event: &RoutedEvent<TEventArgs>,
    ) -> Rc<dyn IObservable<TEventArgs>> {
        self.get_observable_with(routed_event, Self::DEFAULT_ROUTES, false)
    }

    /// Gets an observable for a routed event.
    ///
    /// Observers receive a copy of the args of every raise. The copy
    /// reflects the routing state at the time of the notification; marking
    /// the copy as handled does not affect the event being routed. Use a
    /// handler for that.
    pub fn get_observable_with<TEventArgs: IRoutedEventArgs + Clone>(
        &self,
        routed_event: &RoutedEvent<TEventArgs>,
        routes: RoutingStrategies,
        handled_events_too: bool,
    ) -> Rc<dyn IObservable<TEventArgs>> {
        let o = self.to_ref();
        let routed_event = *routed_event;

        Observable::create(move |observer| {
            o.add_disposable_handler_with(
                &routed_event,
                move |_, e: &TEventArgs| observer.on_next(e.clone()),
                routes,
                handled_events_too,
            )
        })
    }
}
