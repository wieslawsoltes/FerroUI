//! The proxies the reference generates from `DBusXml/` and calls
//! (`OrgA11yBusProxy`, `OrgA11yAtspiSocketProxy`,
//! `OrgA11yAtspiRegistryProxy`), with the members that are called and
//! the signals that are listened to. `OrgA11yStatusProxy` is the
//! properties proxy of the D-Bus library (`at_spi_accessibility_watcher.rs`).

use super::types::{AtSpiEventListener, ObjectReferenceWire};

/// `org.a11y.Bus` at `/org/a11y/bus` of the session bus.
#[zbus::proxy(interface = "org.a11y.Bus", gen_blocking = false, assume_defaults = false)]
pub(crate) trait OrgA11yBus {
    fn get_address(&self) -> zbus::Result<String>;
}

/// `org.a11y.atspi.Socket` at the root path of the registry.
#[zbus::proxy(interface = "org.a11y.atspi.Socket", gen_blocking = false, assume_defaults = false)]
pub(crate) trait OrgA11yAtspiSocket {
    fn embed(&self, plug: &ObjectReferenceWire) -> zbus::Result<ObjectReferenceWire>;
}

/// `org.a11y.atspi.Registry` at `/org/a11y/atspi/registry`.
#[zbus::proxy(interface = "org.a11y.atspi.Registry", gen_blocking = false, assume_defaults = false)]
pub(crate) trait OrgA11yAtspiRegistry {
    fn get_registered_events(&self) -> zbus::Result<Vec<AtSpiEventListener>>;

    #[zbus(signal)]
    fn event_listener_registered(&self, bus: String, event: String, properties: Vec<String>) -> zbus::Result<()>;

    #[zbus(signal)]
    fn event_listener_deregistered(&self, bus: String, event: String) -> zbus::Result<()>;
}
