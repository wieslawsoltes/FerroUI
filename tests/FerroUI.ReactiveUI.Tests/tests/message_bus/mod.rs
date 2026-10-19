//! `ReactiveUI.Tests/MessageBus`: the message bus.

mod message_bus_test;
mod message_bus_test_send_message;
mod skip_first_observer_tests;

use ferroui_reactiveui::{IMessageBus, MessageBus};
use std::rc::Rc;

/// Runs `test` with a bus of its own as the current one, and restores the previous current bus
/// (`WithMessageBusExecutor`).
pub(crate) fn with_message_bus(test: impl FnOnce()) {
    let previous: Rc<dyn IMessageBus> = MessageBus::current();
    MessageBus::set_current(MessageBus::new());
    test();
    MessageBus::set_current(previous);
}
