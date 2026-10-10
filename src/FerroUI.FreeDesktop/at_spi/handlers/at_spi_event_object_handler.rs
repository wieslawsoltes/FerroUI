//! The port of `AtSpiEventObjectHandler.cs`: the signals of
//! `org.a11y.atspi.Event.Object` of an object.

use crate::at_spi::at_spi_constants::{EVENT_OBJECT_VERSION, IFACE_EVENT_OBJECT};
use crate::at_spi::at_spi_server::AtSpiServer;
use crate::at_spi::dbus::descriptions::EVENT_OBJECT;
use crate::at_spi::dbus::interface::{DBusInterface, InterfaceDescription};
use std::collections::HashMap;
use std::rc::Weak;
use zbus::zvariant::Value;

/// The body of an event signal (`siiva{sv}`): the detail, two numbers,
/// a value and the properties, which are always empty.
pub(crate) type EventBody<'a> = (&'a str, i32, i32, Value<'a>, HashMap<String, Value<'a>>);

pub(crate) fn event_body<'a>(detail: &'a str, detail1: i32, detail2: i32, value: Value<'a>) -> EventBody<'a> {
    (detail, detail1, detail2, value, HashMap::new())
}

pub(crate) struct AtSpiEventObjectHandler {
    server: Weak<AtSpiServer>,
    path: String,
}

impl AtSpiEventObjectHandler {
    pub(crate) fn new(server: Weak<AtSpiServer>, path: String) -> Self {
        Self { server, path }
    }

    pub(crate) fn emit_children_changed_signal(&self, operation: &str, index_in_parent: i32, child: Value<'_>) {
        self.emit_signal("ChildrenChanged", event_body(operation, index_in_parent, 0, child));
    }

    pub(crate) fn emit_property_change_signal(&self, property_name: &str, value: Value<'_>) {
        self.emit_signal("PropertyChange", event_body(property_name, 0, 0, value));
    }

    pub(crate) fn emit_state_changed_signal(&self, state_name: &str, detail1: i32, value: Value<'_>) {
        self.emit_signal("StateChanged", event_body(state_name, detail1, 0, value));
    }

    pub(crate) fn emit_selection_changed_signal(&self) {
        self.emit_signal("SelectionChanged", event_body("", 0, 0, Value::from(0i32)));
    }

    pub(crate) fn emit_bounds_changed_signal(&self) {
        self.emit_signal("BoundsChanged", event_body("", 0, 0, Value::from(0i32)));
    }

    fn emit_signal(&self, member: &str, body: EventBody<'_>) {
        let Some(server) = self.server.upgrade() else { return };
        if !server.has_event_listeners() {
            return;
        }

        let Some(connection) = server.a11y_connection() else { return };
        connection.emit_signal(&self.path, IFACE_EVENT_OBJECT, member, &body);
    }
}

impl DBusInterface for AtSpiEventObjectHandler {
    fn description(&self) -> &'static InterfaceDescription {
        &EVENT_OBJECT
    }

    fn get_property(&self, name: &str) -> Option<Value<'static>> {
        (name == "version").then(|| Value::from(EVENT_OBJECT_VERSION))
    }
}
