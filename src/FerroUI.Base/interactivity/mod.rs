//! Routed events: events that travel through the visual tree by tunnelling
//! from the root to the source, bubbling from the source to the root, or
//! going directly to the source.

mod cancel_routed_event_args;
mod event_route;
mod interactive;
mod interactive_extensions;
mod routed_event;
mod routed_event_args;
mod routed_event_registry;

pub use cancel_routed_event_args::CancelRoutedEventArgs;
pub use event_route::{EventRoute, RouteHandler};
pub use interactive::{
    Interactive, InteractiveImpl, InteractiveImplExt, InteractiveVTable, RoutedEventHandlerToken,
};
pub use routed_event::{RoutedEvent, RoutedEventObservable, RouteFinishedObservable, RoutingStrategies};
pub use routed_event_args::{IRoutedEventArgs, RoutedEventArgs};
pub use routed_event_registry::RoutedEventRegistry;


#[cfg(test)]
mod interactive_tests;
