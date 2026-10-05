use super::{IRoutedEventArgs, Interactive, RoutedEvent, RoutingStrategies};
use crate::Ref;
use std::cell::RefCell;
use std::rc::Rc;

/// An event handler as stored on a route: invoked with the element the
/// handler is attached to and the args being routed.
pub type RouteHandler = Rc<dyn Fn(&Interactive, &dyn IRoutedEventArgs)>;

struct RouteItem {
    target: Ref<Interactive>,
    handler: Option<RouteHandler>,
    routes: RoutingStrategies,
    handled_events_too: bool,
}

/// The number of route lists kept for reuse. Raises nest (a handler raising
/// another event), so more than one list can be in use at a time.
const POOL_SIZE: usize = 8;

/// The initial capacity of a route list.
const INITIAL_CAPACITY: usize = 16;

thread_local! {
    static POOL: RefCell<Vec<Vec<RouteItem>>> = const { RefCell::new(Vec::new()) };
}

fn rent() -> Vec<RouteItem> {
    POOL.with(|pool| pool.borrow_mut().pop()).unwrap_or_else(|| Vec::with_capacity(INITIAL_CAPACITY))
}

fn give_back(mut route: Vec<RouteItem>) {
    // Dropping the items releases element references and can therefore run
    // arbitrary drop code: do it before touching the pool.
    route.clear();
    POOL.with(|pool| {
        let mut pool = pool.borrow_mut();
        if pool.len() < POOL_SIZE {
            pool.push(route);
        }
    });
}

/// Holds the route for a routed event and supports raising an event on that
/// route.
///
/// The list of handlers is taken from a per-thread pool and returned to it
/// when the route is dropped (or [disposed](Self::dispose)), so raising an
/// event does not allocate once the pool is warm.
pub struct EventRoute {
    event: RoutedEvent,
    route: Option<Vec<RouteItem>>,
}

impl EventRoute {
    /// Creates a route for a routed event.
    pub fn new<T: ?Sized>(e: &RoutedEvent<T>) -> Self {
        Self { event: e.as_routed_event(), route: None }
    }

    /// Whether the route has any handlers.
    pub fn has_handlers(&self) -> bool {
        self.route.as_ref().is_some_and(|route| !route.is_empty())
    }

    /// Adds a handler to the route.
    pub fn add(
        &mut self,
        target: &Ref<Interactive>,
        handler: RouteHandler,
        routes: RoutingStrategies,
        handled_events_too: bool,
    ) {
        self.route.get_or_insert_with(rent).push(RouteItem {
            target: target.clone(),
            handler: Some(handler),
            routes,
            handled_events_too,
        });
    }

    /// Adds a class handler to the route: a marker that makes the class
    /// handlers of the event run for `target`.
    pub fn add_class_handler(&mut self, target: &Ref<Interactive>) {
        self.route.get_or_insert_with(rent).push(RouteItem {
            target: target.clone(),
            handler: None,
            routes: RoutingStrategies::empty(),
            handled_events_too: false,
        });
    }

    /// Raises an event along the route.
    pub fn raise_event(&self, source: &Ref<Interactive>, e: &dyn IRoutedEventArgs) {
        e.set_source(source);

        let strategies = self.event.routing_strategies();

        if strategies == RoutingStrategies::DIRECT {
            e.set_route(RoutingStrategies::DIRECT);
            self.raise_event_impl(e);
            self.event.invoke_route_finished(e);
        } else {
            if strategies.contains(RoutingStrategies::TUNNEL) {
                e.set_route(RoutingStrategies::TUNNEL);
                self.raise_event_impl(e);
                self.event.invoke_route_finished(e);
            }

            if strategies.contains(RoutingStrategies::BUBBLE) {
                e.set_route(RoutingStrategies::BUBBLE);
                self.raise_event_impl(e);
                self.event.invoke_route_finished(e);
            }
        }
    }

    /// Releases the route's handler list back to the pool.
    pub fn dispose(&mut self) {
        if let Some(route) = self.route.take() {
            give_back(route);
        }
    }

    fn raise_event_impl(&self, e: &dyn IRoutedEventArgs) {
        let Some(route) = &self.route else { return };

        let route_strategy = e.route();
        let count = route.len();
        let mut last_target: Option<&Ref<Interactive>> = None;

        for step in 0..count {
            let index = if route_strategy == RoutingStrategies::TUNNEL { count - 1 - step } else { step };
            let entry = &route[index];

            // If we've got to a new control then call any `raised` listeners
            // (the class handlers).
            if last_target != Some(&entry.target) {
                self.event.invoke_raised(&entry.target, e);

                // If this is a direct event and we've already raised events
                // then we're finished.
                if e.route() == RoutingStrategies::DIRECT && last_target.is_some() {
                    return;
                }

                last_target = Some(&entry.target);
            }

            // Raise the event handler.
            if let Some(handler) = &entry.handler {
                if entry.routes.contains(e.route()) && (!e.handled() || entry.handled_events_too) {
                    handler(&entry.target, e);
                }
            }
        }
    }
}

impl Drop for EventRoute {
    fn drop(&mut self) {
        self.dispose();
    }
}
