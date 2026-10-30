//! The registry of event handlers an event trigger subscribes by name (`Events/`).

mod add_event_handler_registry;
mod contract;
mod handlers;

pub use add_event_handler_registry::AddEventHandlerRegistry;
pub use contract::{AddEventHandlerCallback, IAddEventHandler};
pub use handlers::{EventArgsValue, EventHandler, FuncAddEventHandler};
