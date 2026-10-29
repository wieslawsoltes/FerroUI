//! Port of `Events/Contract/IAddEventHandler.cs`.

use ferroui_base::reactive::IDisposable;
use ferroui_base::BoxedValue;
use std::rc::Rc;

/// The handler an [`IAddEventHandler`] subscribes: it receives the sender (if the event has one)
/// and the event args, untyped (`Action<object?, object>`).
pub type AddEventHandlerCallback = Rc<dyn Fn(Option<&BoxedValue>, &BoxedValue)>;

/// A service that provides a way to register event handlers for specific events on various
/// controls.
pub trait IAddEventHandler {
    /// Checks if the given source object and event name match the criteria for this event
    /// handler.
    fn matches(&self, source: &BoxedValue, event_name: &str) -> bool;

    /// Registers an event handler for a specific event on the given source object. Disposing the
    /// returned handle removes it; `None` when the source or the event does not match.
    fn add_handler(
        &self,
        source: &BoxedValue,
        event_name: &str,
        handler: AddEventHandlerCallback,
    ) -> Option<Rc<dyn IDisposable>>;
}
