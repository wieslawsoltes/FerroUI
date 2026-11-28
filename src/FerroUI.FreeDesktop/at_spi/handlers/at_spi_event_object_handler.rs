//! The port of `AtSpiEventObjectHandler.cs`: the signals of
//! `org.a11y.atspi.Event.Object` of an object.

use crate::at_spi::at_spi_constants::{EVENT_OBJECT_VERSION, IFACE_EVENT_OBJECT};
use crate::at_spi::at_spi_server::AtSpiServer;
use crate::at_spi::dbus::descriptions::EVENT_OBJECT;
use crate::at_spi::dbus::interface::{DBusInterface, InterfaceDescription};
use std::borrow::Cow;
use std::collections::HashMap;
use std::rc::Weak;
use zbus::Variant;

/// The body of an event signal (`siiva{sv}`): the detail, two numbers,
/// a value and the properties, which are always empty.
pub(crate) type EventBody<'a> = (&'a str, i32, i32, Variant<'a>, HashMap<String, Variant<'a>>);

pub(crate) fn event_body<'a>(detail: &'a str, detail1: i32, detail2: i32, value: Variant<'a>) -> EventBody<'a> {
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

    pub(crate) fn version(&self) -> u32 {
        EVENT_OBJECT_VERSION
    }

    pub(crate) fn emit_children_changed_signal(&self, operation: &str, index_in_parent: i32, child: Variant<'_>) {
        self.emit_signal("ChildrenChanged", event_body(operation, index_in_parent, 0, child));
    }

    pub(crate) fn emit_property_change_signal(&self, property_name: &str, value: Variant<'_>) {
        self.emit_signal("PropertyChange", event_body(property_name, 0, 0, value));
    }

    pub(crate) fn emit_state_changed_signal(&self, state_name: &str, detail1: i32, value: Variant<'_>) {
        self.emit_signal("StateChanged", event_body(state_name, detail1, 0, value));
    }

    pub(crate) fn emit_selection_changed_signal(&self) {
        self.emit_signal("SelectionChanged", event_body("", 0, 0, Variant::from(0i32)));
    }

    pub(crate) fn emit_text_caret_moved_signal(&self, position: i32) {
        self.emit_signal("TextCaretMoved", event_body("", position, 0, Variant::from(0i32)));
    }

    pub(crate) fn emit_text_selection_changed_signal(&self) {
        self.emit_signal("TextSelectionChanged", event_body("", 0, 0, Variant::from(0i32)));
    }

    pub(crate) fn emit_text_attributes_changed_signal(&self) {
        self.emit_signal("TextAttributesChanged", event_body("", 0, 0, Variant::from(0i32)));
    }

    pub(crate) fn emit_text_changed_signal(&self, detail: &str, offset: i32, length: i32, text: &str) {
        // D-Bus strings cannot carry a NUL. Preserve its UTF-16 width.
        let text = if text.contains('\0') {
            Cow::Owned(text.replace('\0', "\u{fffd}"))
        } else {
            Cow::Borrowed(text)
        };
        self.emit_signal("TextChanged", event_body(detail, offset, length, Variant::from(text.as_ref())));
    }

    pub(crate) fn emit_bounds_changed_signal(&self) {
        self.emit_signal("BoundsChanged", event_body("", 0, 0, Variant::from(0i32)));
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

    fn get_property(&self, name: &str) -> Option<Variant<'static>> {
        (name == "version").then(|| Variant::from(self.version()))
    }
}
