use super::{EventRoute, IRoutedEventArgs, RouteHandler, RoutedEvent, RoutingStrategies};
use crate::metadata::MarkupDelegate;
use crate::BoxedValue;
use crate::layout::{Layoutable, LayoutableImpl};
use crate::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, StyledElementImpl, VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Identifies a handler added with [`Interactive::add_handler`], for
/// [`Interactive::remove_handler`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RoutedEventHandlerToken(u64);

struct EventSubscription {
    token: RoutedEventHandlerToken,
    handler: RouteHandler,
    routes: RoutingStrategies,
    handled_events_too: bool,
}

struct EventSubscriptions {
    routed_event: RoutedEvent,
    subscriptions: Vec<EventSubscription>,
}

thread_local! {
    static TOTAL_HANDLERS_COUNT: Cell<usize> = const { Cell::new(0) };
    static NEXT_TOKEN: Cell<u64> = const { Cell::new(1) };
}

/// Base class for objects that raise routed events.
#[repr(C)]
pub struct Interactive {
    base: Layoutable,
    event_handlers: RefCell<Vec<EventSubscriptions>>,
}

ferro_class! {
    Interactive: Layoutable, virtuals InteractiveImpl: LayoutableImpl {
        /// The interactive parent of the object for bubbling and tunneling
        /// events. By default this is the visual parent, when it is an
        /// interactive.
        fn interactive_parent(this) -> Option<Ref<Interactive>>;
    }
}
crate::ferro_class_info!(Interactive { new: Interactive::new });

ferro_impl_classes!(Interactive: FerroObjectImpl, StyledElementImpl, VisualImpl, LayoutableImpl);

impl InteractiveImpl for Interactive {
    fn interactive_parent(this: &Self) -> Option<Ref<Interactive>> {
        this.visual_parent().and_then(|parent| parent.downcast::<Interactive>().ok())
    }
}

impl Interactive {
    /// The routes a handler listens to unless specified otherwise: the
    /// direct and bubble routes.
    pub const DEFAULT_ROUTES: RoutingStrategies = RoutingStrategies::DIRECT.union(RoutingStrategies::BUBBLE);

    /// Creates the class data; see [`crate::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: Layoutable::construct(), event_handlers: RefCell::new(Vec::new()) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The number of handlers currently attached to all interactives of the
    /// current thread.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn total_handlers_count() -> usize {
        TOTAL_HANDLERS_COUNT.get()
    }

    /// Adds a handler for a routed event, listening to the direct and
    /// bubble routes of unhandled events.
    ///
    /// Returns a token for [`remove_handler`](Self::remove_handler).
    pub fn add_handler<TEventArgs: IRoutedEventArgs>(
        &self,
        routed_event: &RoutedEvent<TEventArgs>,
        handler: impl Fn(&Interactive, &TEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler_with(routed_event, handler, Self::DEFAULT_ROUTES, false)
    }

    /// Adds a handler for a routed event.
    ///
    /// `routes` selects the routing strategies to listen to, and
    /// `handled_events_too` whether the handler is also invoked for events
    /// that are already handled. The handler receives the element it is
    /// attached to and the event args.
    pub fn add_handler_with<TEventArgs: IRoutedEventArgs>(
        &self,
        routed_event: &RoutedEvent<TEventArgs>,
        handler: impl Fn(&Interactive, &TEventArgs) + 'static,
        routes: RoutingStrategies,
        handled_events_too: bool,
    ) -> RoutedEventHandlerToken {
        // The event's args type is `TEventArgs` by construction, so the
        // conversion below cannot fail for args accepted by `raise_event`.
        self.add_handler_as::<TEventArgs, TEventArgs>(routed_event, handler, routes, handled_events_too)
    }

    /// Adds a handler written against args type `THandlerArgs` to any
    /// routed event whose args type is `THandlerArgs` or derives from it.
    ///
    /// This is how a handler taking the base
    /// [`RoutedEventArgs`](super::RoutedEventArgs) is attached to an event
    /// with more specific args, and how handlers are attached to an untyped
    /// [`RoutedEvent`].
    ///
    /// Panics if the event's args type does not derive from `THandlerArgs`.
    pub fn add_handler_as<THandlerArgs: IRoutedEventArgs, TEventArgs: ?Sized>(
        &self,
        routed_event: &RoutedEvent<TEventArgs>,
        handler: impl Fn(&Interactive, &THandlerArgs) + 'static,
        routes: RoutingStrategies,
        handled_events_too: bool,
    ) -> RoutedEventHandlerToken {
        assert!(
            routed_event.event_args_is::<THandlerArgs>(),
            "A handler taking {} cannot be attached to {}, whose args are {}.",
            std::any::type_name::<THandlerArgs>(),
            routed_event,
            routed_event.event_args_type_name()
        );

        let adapter: RouteHandler = Rc::new(move |sender: &Interactive, args: &dyn IRoutedEventArgs| {
            match args.downcast_ref::<THandlerArgs>() {
                Some(typed_args) => handler(sender, typed_args),
                None => panic!(
                    "Event args of the wrong type were raised: expected {}.",
                    std::any::type_name::<THandlerArgs>()
                ),
            }
        });

        self.add_route_handler(routed_event, adapter, routes, handled_events_too)
    }

    /// Adds an untyped handler for a routed event: what markup attaches for
    /// an event attribute.
    ///
    /// The handler is called with two untyped values: the element it is
    /// attached to (a `Ref<Interactive>`) and a shared handle to the event
    /// args (an `Rc<dyn IRoutedEventArgs>`, see
    /// [`IRoutedEventArgs::share`]). The handle refers to the live args:
    /// marking it as handled marks the event as handled for the handlers
    /// further along the route.
    pub fn add_handler_untyped<TEventArgs: ?Sized>(
        &self,
        routed_event: &RoutedEvent<TEventArgs>,
        handler: MarkupDelegate,
        routes: RoutingStrategies,
        handled_events_too: bool,
    ) -> RoutedEventHandlerToken {
        let adapter: RouteHandler = Rc::new(move |sender: &Interactive, args: &dyn IRoutedEventArgs| {
            let sender: BoxedValue = Rc::new(sender.to_ref());
            let args: BoxedValue = Rc::new(args.share());
            handler.invoke(&[Some(sender), Some(args)]);
        });

        self.add_route_handler(routed_event, adapter, routes, handled_events_too)
    }

    /// Adds an already adapted handler for a routed event.
    pub fn add_route_handler<TEventArgs: ?Sized>(
        &self,
        routed_event: &RoutedEvent<TEventArgs>,
        handler: RouteHandler,
        routes: RoutingStrategies,
        handled_events_too: bool,
    ) -> RoutedEventHandlerToken {
        let token = RoutedEventHandlerToken(NEXT_TOKEN.get());
        NEXT_TOKEN.set(token.0 + 1);

        let subscription = EventSubscription { token, handler, routes, handled_events_too };
        self.add_event_subscription(routed_event.as_routed_event(), subscription);
        token
    }

    /// Removes a handler for a routed event. Returns whether the handler was
    /// attached.
    pub fn remove_handler<TEventArgs: ?Sized>(
        &self,
        routed_event: &RoutedEvent<TEventArgs>,
        handler: RoutedEventHandlerToken,
    ) -> bool {
        let routed_event = routed_event.as_routed_event();
        // The removed subscription is dropped after the borrow is released:
        // dropping a handler can run arbitrary code.
        let removed = {
            let mut event_handlers = self.event_handlers.borrow_mut();
            event_handlers.iter_mut().find(|entry| entry.routed_event == routed_event).and_then(|entry| {
                let index = entry.subscriptions.iter().rposition(|s| s.token == handler)?;
                Some(entry.subscriptions.remove(index))
            })
        };

        if removed.is_some() {
            TOTAL_HANDLERS_COUNT.set(TOTAL_HANDLERS_COUNT.get() - 1);
        }
        removed.is_some()
    }

    /// Raises a routed event.
    ///
    /// Panics if the args have no routed event.
    pub fn raise_event(&self, e: &dyn IRoutedEventArgs) {
        let Some(routed_event) = e.routed_event() else {
            panic!("Cannot raise an event whose RoutedEvent is null.");
        };

        let route = self.build_event_route(&routed_event);
        route.raise_event(&self.to_ref(), e);
    }

    /// Builds an event route for a routed event: the handlers the event
    /// would visit if raised on this object, in bubbling order.
    pub fn build_event_route<TEventArgs: ?Sized>(&self, e: &RoutedEvent<TEventArgs>) -> EventRoute {
        let e = e.as_routed_event();
        let mut result = EventRoute::new(&e);
        let has_class_handlers = e.has_raised_subscriptions();
        let strategies = e.routing_strategies();
        let this = self.to_ref();

        if strategies.contains(RoutingStrategies::BUBBLE) || strategies.contains(RoutingStrategies::TUNNEL) {
            let mut element = Some(this);

            while let Some(current) = element {
                if has_class_handlers {
                    result.add_class_handler(&current);
                }

                current.add_to_event_route(&current, e, &mut result);
                element = current.interactive_parent();
            }
        } else {
            if has_class_handlers {
                result.add_class_handler(&this);
            }

            self.add_to_event_route(&this, e, &mut result);
        }

        result
    }

    fn add_event_subscription(&self, routed_event: RoutedEvent, subscription: EventSubscription) {
        let mut event_handlers = self.event_handlers.borrow_mut();

        match event_handlers.iter_mut().find(|entry| entry.routed_event == routed_event) {
            Some(entry) => entry.subscriptions.push(subscription),
            None => event_handlers.push(EventSubscriptions { routed_event, subscriptions: vec![subscription] }),
        }

        TOTAL_HANDLERS_COUNT.set(TOTAL_HANDLERS_COUNT.get() + 1);
    }

    fn add_to_event_route(&self, this: &Ref<Interactive>, routed_event: RoutedEvent, route: &mut EventRoute) {
        // The route takes its own references to the handlers, so handlers
        // may add or remove handlers while the event is being raised; no
        // user code runs while the list is borrowed here.
        let event_handlers = self.event_handlers.borrow();

        if let Some(entry) = event_handlers.iter().find(|entry| entry.routed_event == routed_event) {
            for sub in &entry.subscriptions {
                route.add(this, sub.handler.clone(), sub.routes, sub.handled_events_too);
            }
        }
    }
}
