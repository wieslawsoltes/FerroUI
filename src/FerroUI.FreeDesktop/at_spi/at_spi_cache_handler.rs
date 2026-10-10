//! The port of `AtSpiCacheHandler.cs`: `org.a11y.atspi.Cache`, which
//! answers with no items, so that clients ask the objects themselves.

use super::at_spi_constants::CACHE_VERSION;
use super::dbus::descriptions::CACHE;
use super::dbus::interface::{reply, CallResult, DBusError, DBusInterface, InterfaceDescription};
use super::dbus::types::AtSpiAccessibleCacheItem;
use zbus::zvariant::Value;

pub(crate) struct AtSpiCacheHandler;

impl DBusInterface for AtSpiCacheHandler {
    fn description(&self) -> &'static InterfaceDescription {
        &CACHE
    }

    fn call(&self, member: &str, _body: &zbus::message::Body) -> CallResult {
        match member {
            "GetItems" => reply((Vec::<AtSpiAccessibleCacheItem>::new(),)),
            _ => Err(DBusError::unknown_method()),
        }
    }

    fn get_property(&self, name: &str) -> Option<Value<'static>> {
        (name == "version").then(|| Value::from(CACHE_VERSION))
    }
}
