//! The port of `AtSpiImageHandler.cs`: `org.a11y.atspi.Image`.

use super::at_spi_component_handler::AtSpiComponentHandler;
use super::node_of;
use crate::at_spi::at_spi_constants::{resolve_locale, IMAGE_VERSION};
use crate::at_spi::at_spi_node::AtSpiNode;
use crate::at_spi::at_spi_server::AtSpiServer;
use crate::at_spi::dbus::descriptions::IMAGE;
use crate::at_spi::dbus::interface::{args, reply, CallResult, DBusError, DBusInterface, InterfaceDescription};
use std::rc::Weak;
use zbus::zvariant::Value;

pub(crate) struct AtSpiImageHandler {
    node: Weak<AtSpiNode>,
}

impl AtSpiImageHandler {
    pub(crate) fn new(_server: Weak<AtSpiServer>, node: Weak<AtSpiNode>) -> Self {
        Self { node }
    }
}

impl DBusInterface for AtSpiImageHandler {
    fn description(&self) -> &'static InterfaceDescription {
        &IMAGE
    }

    fn call(&self, member: &str, body: &zbus::message::Body) -> CallResult {
        let node = node_of(&self.node)?;
        // The extents of an image are those of its component.
        match member {
            "GetImageExtents" => reply((AtSpiComponentHandler::get_extents(&node, args::<u32>(body)?),)),
            "GetImagePosition" => {
                let extents = AtSpiComponentHandler::get_extents(&node, args::<u32>(body)?);
                reply((extents.0, extents.1))
            }
            "GetImageSize" => {
                let extents = AtSpiComponentHandler::get_extents(&node, 0);
                reply((extents.2, extents.3))
            }
            _ => Err(DBusError::unknown_method()),
        }
    }

    fn get_property(&self, name: &str) -> Option<Value<'static>> {
        let node = self.node.upgrade()?;
        Some(match name {
            "version" => Value::from(IMAGE_VERSION),
            "ImageDescription" => Value::from(node.peer().get_help_text()),
            "ImageLocale" => Value::from(resolve_locale()),
            _ => return None,
        })
    }
}
