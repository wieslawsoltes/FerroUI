//! The port of `ApplicationNodeApplicationHandler.cs`:
//! `org.a11y.atspi.Application`, of the root object and of every window.

use super::at_spi_constants::*;
use super::dbus::descriptions::APPLICATION;
use super::dbus::interface::{args, reply, CallResult, DBusError, DBusInterface, InterfaceDescription};
use std::cell::Cell;
use zbus::Variant;

pub(crate) struct ApplicationNodeApplicationHandler {
    toolkit_name: &'static str,
    version: String,
    toolkit_version: String,
    atspi_version: &'static str,
    interface_version: u32,
    /// The identifier the registry gives the application.
    id: Cell<i32>,
}

impl ApplicationNodeApplicationHandler {
    pub(crate) fn new() -> Self {
        let version = resolve_toolkit_version();
        Self {
            toolkit_name: TOOLKIT_NAME,
            version: version.clone(),
            toolkit_version: version,
            atspi_version: "2.1",
            interface_version: APPLICATION_VERSION,
            id: Cell::new(0),
        }
    }

}

impl ApplicationNodeApplicationHandler {
    fn get_locale_async(&self, _lctype: u32) -> String {
        resolve_locale()
    }

    fn get_application_bus_address_async(&self) -> String {
        String::new()
    }
}

impl DBusInterface for ApplicationNodeApplicationHandler {
    fn description(&self) -> &'static InterfaceDescription {
        &APPLICATION
    }

    fn call(&self, member: &str, body: &zbus::message::Body) -> CallResult {
        match member {
            "GetLocale" => reply((self.get_locale_async(args::<u32>(body)?),)),
            "GetApplicationBusAddress" => reply((self.get_application_bus_address_async(),)),
            _ => Err(DBusError::unknown_method()),
        }
    }

    fn get_property(&self, name: &str) -> Option<Variant<'static>> {
        Some(match name {
            "ToolkitName" => Variant::from(self.toolkit_name),
            "Version" => Variant::from(self.version.clone()),
            "ToolkitVersion" => Variant::from(self.toolkit_version.clone()),
            "AtspiVersion" => Variant::from(self.atspi_version),
            "InterfaceVersion" => Variant::from(self.interface_version),
            "Id" => Variant::from(self.id.get()),
            _ => return None,
        })
    }

    fn set_property(&self, name: &str, value: &Variant<'_>) -> bool {
        match (name, value) {
            ("Id", Variant::I32(id)) => {
                self.id.set(*id);
                true
            }
            _ => false,
        }
    }
}
