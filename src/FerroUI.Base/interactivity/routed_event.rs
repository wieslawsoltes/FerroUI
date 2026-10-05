use super::{IRoutedEventArgs, Interactive, RoutedEventRegistry};
use crate::reactive::{Disposable, IDisposable};
use crate::utilities::HandlerList;
use crate::{ObjectType, StaticType, TypeInfo};
use bitflags::bitflags;
use std::any::TypeId;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::rc::Rc;

bitflags! {
    /// The ways a routed event can travel through the tree, and the parts of
    /// a route a handler listens to.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct RoutingStrategies: i32 {
        /// The event is only raised on its source.
        const DIRECT = 0x01;
        /// The event travels from the root of the tree down to its source.
        const TUNNEL = 0x02;
        /// The event travels from its source up to the root of the tree.
        const BUBBLE = 0x04;
    }
}

type RaisedHandler = dyn Fn(&Interactive, &dyn IRoutedEventArgs);
type RouteFinishedHandler = dyn Fn(&dyn IRoutedEventArgs);

/// The stream of `(sender, args)` notifications of a routed event: one for
/// every element on the route of every raise. This is what class handlers
/// are built on.
pub struct RoutedEventObservable {
    handlers: HandlerList<RaisedHandler>,
}

impl RoutedEventObservable {
    /// Subscribes to the notifications. Disposing the returned handle
    /// unsubscribes.
    pub fn subscribe(
        &'static self,
        handler: impl Fn(&Interactive, &dyn IRoutedEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.handlers.add(Rc::new(handler));
        Disposable::create(move || {
            self.handlers.remove(token);
        })
    }

    /// Whether there is any subscriber.
    #[inline]
    pub fn has_observers(&self) -> bool {
        !self.handlers.is_empty()
    }

    #[inline]
    fn notify(&self, sender: &Interactive, e: &dyn IRoutedEventArgs) {
        if self.handlers.is_empty() {
            return;
        }
        for (_, handler) in self.handlers.snapshot().iter() {
            handler(sender, e);
        }
    }
}

/// The stream of notifications raised when a routed event has finished
/// travelling one of its routes (tunnel, bubble or direct).
pub struct RouteFinishedObservable {
    handlers: HandlerList<RouteFinishedHandler>,
}

impl RouteFinishedObservable {
    /// Subscribes to the notifications. Disposing the returned handle
    /// unsubscribes.
    pub fn subscribe(&'static self, handler: impl Fn(&dyn IRoutedEventArgs) + 'static) -> Rc<dyn IDisposable> {
        let token = self.handlers.add(Rc::new(handler));
        Disposable::create(move || {
            self.handlers.remove(token);
        })
    }

    #[inline]
    fn notify(&self, e: &dyn IRoutedEventArgs) {
        if self.handlers.is_empty() {
            return;
        }
        for (_, handler) in self.handlers.snapshot().iter() {
            handler(e);
        }
    }
}

struct RoutedEventInner {
    name: Box<str>,
    routing_strategies: RoutingStrategies,
    event_args_type: TypeId,
    event_args_type_name: &'static str,
    /// Whether the event's args type is, or derives from, a given args type.
    event_args_is: fn(TypeId) -> bool,
    owner_type: &'static TypeInfo,
    raised: RoutedEventObservable,
    route_finished: RouteFinishedObservable,
}

/// The identity and description of a routed event.
///
/// `RoutedEvent<TEventArgs>` is a routed event whose handlers receive
/// `TEventArgs`; plain `RoutedEvent` is the untyped view of any routed
/// event (the base class of the typed ones). Both are cheap copyable
/// handles to the same definition and compare equal when they denote the
/// same event; [`as_routed_event`](Self::as_routed_event) converts a typed
/// handle to the untyped one.
///
/// Like property definitions, routed events are registered once per thread
/// and live for the rest of the program; declare them with
/// [`ferro_routed_event!`](crate::ferro_routed_event).
pub struct RoutedEvent<TEventArgs: ?Sized = dyn IRoutedEventArgs> {
    inner: &'static RoutedEventInner,
    _marker: PhantomData<fn(&TEventArgs)>,
}

impl<T: ?Sized> Clone for RoutedEvent<T> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized> Copy for RoutedEvent<T> {}

impl<T: ?Sized, U: ?Sized> PartialEq<RoutedEvent<U>> for RoutedEvent<T> {
    #[inline]
    fn eq(&self, other: &RoutedEvent<U>) -> bool {
        std::ptr::eq(self.inner, other.inner)
    }
}

impl<T: ?Sized> Eq for RoutedEvent<T> {}

impl<T: ?Sized> Hash for RoutedEvent<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (self.inner as *const RoutedEventInner as usize).hash(state)
    }
}

impl<T: ?Sized> fmt::Debug for RoutedEvent<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl<T: ?Sized> fmt::Display for RoutedEvent<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.inner.owner_type.name(), self.inner.name)
    }
}

fn create_inner<TEventArgs: IRoutedEventArgs>(
    name: &str,
    routing_strategies: RoutingStrategies,
    owner_type: &'static TypeInfo,
) -> &'static RoutedEventInner {
    Box::leak(Box::new(RoutedEventInner {
        name: name.into(),
        routing_strategies,
        event_args_type: TypeId::of::<TEventArgs>(),
        event_args_type_name: std::any::type_name::<TEventArgs>(),
        event_args_is: TEventArgs::is_args_type,
        owner_type,
        raised: RoutedEventObservable { handlers: HandlerList::new() },
        route_finished: RouteFinishedObservable { handlers: HandlerList::new() },
    }))
}

impl RoutedEvent {
    /// Creates an untyped routed event whose args are of type `TEventArgs`,
    /// without registering it.
    pub fn new_untyped<TEventArgs: IRoutedEventArgs>(
        name: &str,
        routing_strategies: RoutingStrategies,
        owner_type: &'static TypeInfo,
    ) -> RoutedEvent {
        RoutedEvent { inner: create_inner::<TEventArgs>(name, routing_strategies, owner_type), _marker: PhantomData }
    }

    /// Registers a routed event owned by class `TOwner`.
    pub fn register<TOwner: StaticType, TEventArgs: IRoutedEventArgs>(
        name: &str,
        routing_strategy: RoutingStrategies,
    ) -> &'static RoutedEvent<TEventArgs> {
        Self::register_with_owner::<TEventArgs>(name, routing_strategy, TOwner::TYPE)
    }

    /// Registers a routed event owned by the given type.
    pub fn register_with_owner<TEventArgs: IRoutedEventArgs>(
        name: &str,
        routing_strategy: RoutingStrategies,
        owner_type: &'static TypeInfo,
    ) -> &'static RoutedEvent<TEventArgs> {
        let routed_event: &'static RoutedEvent<TEventArgs> =
            Box::leak(Box::new(RoutedEvent::<TEventArgs>::new(name, routing_strategy, owner_type)));
        RoutedEventRegistry::instance().register(owner_type, routed_event.as_routed_event());
        routed_event
    }
}

impl<TEventArgs: ?Sized> RoutedEvent<TEventArgs> {
    /// The untyped handle of this routed event.
    #[inline]
    pub fn as_routed_event(&self) -> RoutedEvent {
        RoutedEvent { inner: self.inner, _marker: PhantomData }
    }

    /// The type of the event's args.
    #[inline]
    pub fn event_args_type(&self) -> TypeId {
        self.inner.event_args_type
    }

    /// The Rust type name of the event's args.
    #[inline]
    pub fn event_args_type_name(&self) -> &'static str {
        self.inner.event_args_type_name
    }

    /// Whether the event's args type is `T` or derives from it, i.e. whether
    /// a handler taking `T` can be attached to the event.
    #[inline]
    pub fn event_args_is<T: IRoutedEventArgs>(&self) -> bool {
        (self.inner.event_args_is)(TypeId::of::<T>())
    }

    /// The name of the event.
    #[inline]
    pub fn name(&self) -> &'static str {
        &self.inner.name
    }

    /// The class that registered the event.
    #[inline]
    pub fn owner_type(&self) -> &'static TypeInfo {
        self.inner.owner_type
    }

    /// How the event travels through the tree.
    #[inline]
    pub fn routing_strategies(&self) -> RoutingStrategies {
        self.inner.routing_strategies
    }

    /// Whether anything is subscribed to [`raised`](Self::raised), i.e.
    /// whether the event has class handlers.
    #[inline]
    pub fn has_raised_subscriptions(&self) -> bool {
        self.inner.raised.has_observers()
    }

    /// Notifications of the event being raised on any element.
    #[inline]
    pub fn raised(&self) -> &'static RoutedEventObservable {
        &self.inner.raised
    }

    /// Notifications of the event having finished one of its routes.
    #[inline]
    pub fn route_finished(&self) -> &'static RouteFinishedObservable {
        &self.inner.route_finished
    }

    /// Adds a handler that is invoked for every element of type
    /// `target_type` (or derived from it) on the route of the event, before
    /// the handlers attached to that element.
    ///
    /// The handler is invoked on the direct route, and on the tunnel or
    /// bubble routes selected by `routes`; for events that are already
    /// handled only if `handled_events_too` is set.
    pub fn add_class_handler_untyped(
        &self,
        target_type: &'static TypeInfo,
        handler: impl Fn(&Interactive, &dyn IRoutedEventArgs) + 'static,
        routes: RoutingStrategies,
        handled_events_too: bool,
    ) -> Rc<dyn IDisposable> {
        self.inner.raised.subscribe(move |sender, e| {
            if target_type.is_assignable_from(sender.get_type())
                && (e.route() == RoutingStrategies::DIRECT || e.route().intersects(routes))
                && (!e.handled() || handled_events_too)
            {
                handler(sender, e);
            }
        })
    }

    #[inline]
    pub(crate) fn invoke_raised(&self, sender: &Interactive, e: &dyn IRoutedEventArgs) {
        self.inner.raised.notify(sender, e)
    }

    #[inline]
    pub(crate) fn invoke_route_finished(&self, e: &dyn IRoutedEventArgs) {
        self.inner.route_finished.notify(e)
    }
}

impl<TEventArgs: IRoutedEventArgs> RoutedEvent<TEventArgs> {
    /// Creates a routed event without registering it.
    pub fn new(name: &str, routing_strategies: RoutingStrategies, owner_type: &'static TypeInfo) -> Self {
        RoutedEvent { inner: create_inner::<TEventArgs>(name, routing_strategies, owner_type), _marker: PhantomData }
    }

    /// Adds a class handler for elements of class `TTarget`, listening to
    /// the direct and bubble routes of unhandled events.
    pub fn add_class_handler<TTarget: ObjectType>(
        &self,
        handler: impl Fn(&TTarget, &TEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        self.add_class_handler_with(handler, RoutingStrategies::DIRECT | RoutingStrategies::BUBBLE, false)
    }

    /// Adds a class handler for elements of class `TTarget`.
    ///
    /// See [`add_class_handler_untyped`](Self::add_class_handler_untyped).
    pub fn add_class_handler_with<TTarget: ObjectType>(
        &self,
        handler: impl Fn(&TTarget, &TEventArgs) + 'static,
        routes: RoutingStrategies,
        handled_events_too: bool,
    ) -> Rc<dyn IDisposable> {
        self.add_class_handler_untyped(
            TTarget::TYPE,
            move |sender, e| {
                if let (Some(target), Some(args)) = (sender.downcast_ref::<TTarget>(), e.downcast_ref::<TEventArgs>())
                {
                    handler(target, args);
                }
            },
            routes,
            handled_events_too,
        )
    }
}

/// Declares the accessor of a routed event definition.
///
/// ```ignore
/// impl Button {
///     ferro_routed_event!(pub fn click_event() -> RoutedEvent<RoutedEventArgs> {
///         RoutedEvent::register::<Button, _>("Click", RoutingStrategies::BUBBLE)
///     });
/// }
/// ```
///
/// The event is registered the first time the accessor is called on a
/// thread. Routed event definitions, like the objects that raise them,
/// belong to the thread that uses them.
#[macro_export]
macro_rules! ferro_routed_event {
    ($(#[$meta:meta])* $vis:vis fn $name:ident() -> $ty:ty $body:block) => {
        $(#[$meta])*
        #[inline]
        $vis fn $name() -> &'static $ty {
            ::std::thread_local! {
                static CELL: ::std::cell::Cell<::std::option::Option<&'static $ty>> =
                    const { ::std::cell::Cell::new(::std::option::Option::None) };
            }
            match CELL.get() {
                ::std::option::Option::Some(routed_event) => routed_event,
                ::std::option::Option::None => {
                    let routed_event: &'static $ty = $body;
                    CELL.set(::std::option::Option::Some(routed_event));
                    routed_event
                }
            }
        }
    };
}
