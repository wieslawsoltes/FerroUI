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

impl AtSpiImageHandler {
    fn version() -> u32 {
        IMAGE_VERSION
    }

    fn image_description(node: &AtSpiNode) -> String {
        node.peer().get_help_text()
    }

    fn image_locale() -> String {
        resolve_locale()
    }

    // The extents of an image are those of its component.
    fn get_image_extents_async(node: &AtSpiNode, coord_type: u32) -> (i32, i32, i32, i32) {
        AtSpiComponentHandler::get_extents_async(node, coord_type)
    }

    fn get_image_position_async(node: &AtSpiNode, coord_type: u32) -> (i32, i32) {
        let extents = Self::get_image_extents_async(node, coord_type);
        (extents.0, extents.1)
    }

    fn get_image_size_async(node: &AtSpiNode) -> (i32, i32) {
        let extents = Self::get_image_extents_async(node, 0);
        (extents.2, extents.3)
    }
}

impl DBusInterface for AtSpiImageHandler {
    fn description(&self) -> &'static InterfaceDescription {
        &IMAGE
    }

    fn call(&self, member: &str, body: &zbus::message::Body) -> CallResult {
        let node = node_of(&self.node)?;
        match member {
            "GetImageExtents" => reply((Self::get_image_extents_async(&node, args::<u32>(body)?),)),
            "GetImagePosition" => reply(Self::get_image_position_async(&node, args::<u32>(body)?)),
            "GetImageSize" => reply(Self::get_image_size_async(&node)),
            _ => Err(DBusError::unknown_method()),
        }
    }

    fn get_property(&self, name: &str) -> Option<Value<'static>> {
        let node = self.node.upgrade()?;
        Some(match name {
            "version" => Value::from(Self::version()),
            "ImageDescription" => Value::from(Self::image_description(&node)),
            "ImageLocale" => Value::from(Self::image_locale()),
            _ => return None,
        })
    }
}
