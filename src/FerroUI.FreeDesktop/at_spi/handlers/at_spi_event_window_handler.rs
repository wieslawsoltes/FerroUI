//! The port of `AtSpiEventWindowHandler.cs`: the signals of
//! `org.a11y.atspi.Event.Window` of a window.

use super::at_spi_event_object_handler::{event_body, EventBody};
use crate::at_spi::at_spi_constants::IFACE_EVENT_WINDOW;
use crate::at_spi::at_spi_server::AtSpiServer;
use crate::at_spi::dbus::descriptions::EVENT_WINDOW;
use crate::at_spi::dbus::interface::{DBusInterface, InterfaceDescription};
use std::rc::Weak;
use zbus::zvariant::Value;

pub(crate) struct AtSpiEventWindowHandler {
    server: Weak<AtSpiServer>,
    path: String,
}

impl AtSpiEventWindowHandler {
    pub(crate) fn new(server: Weak<AtSpiServer>, path: String) -> Self {
        Self { server, path }
    }

    pub(crate) fn emit_activate_signal(&self) {
        self.emit_signal("Activate", event_body("", 0, 0, Value::from("0")));
    }

    pub(crate) fn emit_deactivate_signal(&self) {
        self.emit_signal("Deactivate", event_body("", 0, 0, Value::from("0")));
    }

    fn emit_signal(&self, member: &str, body: EventBody<'_>) {
        let Some(server) = self.server.upgrade() else { return };
        if !server.has_event_listeners() {
            return;
        }

        let Some(connection) = server.a11y_connection() else { return };
        connection.emit_signal(&self.path, IFACE_EVENT_WINDOW, member, &body);
    }
}

impl DBusInterface for AtSpiEventWindowHandler {
    fn description(&self) -> &'static InterfaceDescription {
        &EVENT_WINDOW
    }
}
