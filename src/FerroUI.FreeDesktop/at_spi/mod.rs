//! Accessibility over AT-SPI: the automation peers of the application as
//! objects of the accessibility bus (the port of the reference's
//! AT-SPI project). `docs/porting/atspi.md` has the design.
//!
//! The server ([`AtSpiServer`]) is started by a platform backend, which
//! adds and removes the automation peers of its windows; everything else
//! of this module is how the server answers.

mod application_accessible_handler;
mod application_at_spi_node;
mod application_node_application_handler;
mod at_spi_accessibility_watcher;
mod at_spi_cache_handler;
mod at_spi_constants;
mod at_spi_coord_type;
mod at_spi_node;
mod at_spi_node_role_mapping;
mod at_spi_node_state_mapping;
mod at_spi_registry_event_tracker;
mod at_spi_role;
mod at_spi_server;
mod at_spi_state;
mod handlers;
mod root_at_spi_node;

/// What the module has of the reference's D-Bus library and of the code
/// generated from the interface descriptions.
mod dbus {
    pub(crate) mod built_in_introspection_handler;
    pub(crate) mod built_in_properties_handler;
    pub(crate) mod connection;
    pub(crate) mod descriptions;
    pub(crate) mod interface;
    pub(crate) mod proxies;
    pub(crate) mod types;
}

pub use at_spi_accessibility_watcher::AtSpiAccessibilityWatcher;
pub use at_spi_server::AtSpiServer;

#[cfg(test)]
mod tests;
