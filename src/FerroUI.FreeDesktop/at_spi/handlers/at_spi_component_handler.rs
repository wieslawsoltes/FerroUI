//! The port of `AtSpiComponentHandler.cs`: `org.a11y.atspi.Component`.

use super::at_spi_coordinate_helper::AtSpiCoordinateHelper;
use super::{node_of, server_of};
use crate::at_spi::at_spi_constants::*;
use crate::at_spi::at_spi_coord_type::AtSpiCoordType;
use crate::at_spi::at_spi_node::AtSpiNode;
use crate::at_spi::at_spi_server::AtSpiServer;
use crate::at_spi::dbus::descriptions::COMPONENT;
use crate::at_spi::dbus::interface::{args, reply, CallResult, DBusError, DBusInterface, InterfaceDescription};
use crate::at_spi::dbus::types::{AtSpiObjectReference, AtSpiRect};
use ferroui_base::{PixelPoint, Point};
use ferroui_controls::automation::peers::AutomationControlType;
use ferroui_controls::automation::provider::IRootProvider;
use std::rc::{Rc, Weak};
use zbus::zvariant::Value;

pub(crate) struct AtSpiComponentHandler {
    server: Weak<AtSpiServer>,
    node: Weak<AtSpiNode>,
}

impl AtSpiComponentHandler {
    pub(crate) fn new(server: Weak<AtSpiServer>, node: Weak<AtSpiNode>) -> Self {
        Self { server, node }
    }

    fn version() -> u32 {
        COMPONENT_VERSION
    }

    fn get_position_async(node: &AtSpiNode, coord_type: u32) -> (i32, i32) {
        let extents = Self::get_extents_async(node, coord_type);
        (extents.0, extents.1)
    }

    fn get_mdiz_order_async() -> i16 {
        -1
    }

    fn grab_focus_async(node: &AtSpiNode) -> bool {
        node.peer().set_focus();
        true
    }

    fn get_alpha_async() -> f64 {
        1.0
    }

    fn set_extents_async(node: &AtSpiNode, x: i32, y: i32, _width: i32, _height: i32, coord_type: u32) -> bool {
        // Only support moving (not resizing) for now
        Self::set_position_async(node, x, y, coord_type)
    }

    fn set_size_async(_width: i32, _height: i32) -> bool {
        false
    }

    fn scroll_to_async(_scroll_type: u32) -> bool {
        false
    }

    fn scroll_to_point_async(_coord_type: u32, _x: i32, _y: i32) -> bool {
        false
    }

    fn contains_async(node: &AtSpiNode, x: i32, y: i32, coord_type: u32) -> bool {
        let rect = AtSpiCoordinateHelper::get_screen_extents(node);
        let point = Self::translate_to_screen(node, x, y, coord_type);
        rect.contains_exclusive(Point::new(f64::from(point.0), f64::from(point.1)))
    }

    fn get_accessible_at_point_async(
        server: &AtSpiServer,
        node: &Rc<AtSpiNode>,
        x: i32,
        y: i32,
        coord_type: u32,
    ) -> AtSpiObjectReference {
        if Self::contains_async(node, x, y, coord_type) {
            server.get_reference(Some(node))
        } else {
            server.get_null_reference()
        }
    }

    pub(crate) fn get_extents_async(node: &AtSpiNode, coord_type: u32) -> AtSpiRect {
        let rect = AtSpiCoordinateHelper::get_screen_extents(node);
        let translated = AtSpiCoordinateHelper::translate_rect(node, rect, coord_type);
        (translated.x as i32, translated.y as i32, translated.width as i32, translated.height as i32)
    }

    fn get_size_async(node: &AtSpiNode) -> (i32, i32) {
        let rect = AtSpiCoordinateHelper::get_screen_extents(node);
        (rect.width as i32, rect.height as i32)
    }

    fn get_layer_async(node: &AtSpiNode) -> u32 {
        if node.peer().get_automation_control_type() == AutomationControlType::Window {
            WINDOW_LAYER
        } else {
            WIDGET_LAYER
        }
    }

    fn set_position_async(node: &AtSpiNode, x: i32, y: i32, coord_type: u32) -> bool {
        let Some(platform_impl) =
            node.peer().get_provider::<dyn IRootProvider>().and_then(|provider| provider.platform_impl())
        else {
            return false;
        };
        let Some(window_impl) = platform_impl.as_window_impl() else { return false };

        let screen_pos = Self::translate_to_screen(node, x, y, coord_type);
        window_impl.move_(PixelPoint::new(screen_pos.0, screen_pos.1));
        true
    }

    fn translate_to_screen(node: &AtSpiNode, x: i32, y: i32, coord_type: u32) -> (i32, i32) {
        match AtSpiCoordType::from_u32(coord_type) {
            Some(AtSpiCoordType::Screen) | None => (x, y),
            Some(AtSpiCoordType::Window) => {
                let window_rect = AtSpiCoordinateHelper::get_window_rect(node);
                (x + window_rect.x as i32, y + window_rect.y as i32)
            }
            Some(AtSpiCoordType::Parent) => {
                let parent_rect = AtSpiCoordinateHelper::get_parent_screen_rect(node);
                (x + parent_rect.x as i32, y + parent_rect.y as i32)
            }
        }
    }
}

impl DBusInterface for AtSpiComponentHandler {
    fn description(&self) -> &'static InterfaceDescription {
        &COMPONENT
    }

    fn call(&self, member: &str, body: &zbus::message::Body) -> CallResult {
        let (server, node) = (server_of(&self.server)?, node_of(&self.node)?);
        match member {
            "Contains" => {
                let (x, y, coord_type) = args::<(i32, i32, u32)>(body)?;
                reply((Self::contains_async(&node, x, y, coord_type),))
            }
            "GetAccessibleAtPoint" => {
                let (x, y, coord_type) = args::<(i32, i32, u32)>(body)?;
                reply((Self::get_accessible_at_point_async(&server, &node, x, y, coord_type).to_wire(),))
            }
            "GetExtents" => reply((Self::get_extents_async(&node, args::<u32>(body)?),)),
            "GetPosition" => reply(Self::get_position_async(&node, args::<u32>(body)?)),
            "GetSize" => reply(Self::get_size_async(&node)),
            "GetLayer" => reply((Self::get_layer_async(&node),)),
            "GetMDIZOrder" => reply((Self::get_mdiz_order_async(),)),
            "GrabFocus" => reply((Self::grab_focus_async(&node),)),
            "GetAlpha" => reply((Self::get_alpha_async(),)),
            "SetExtents" => {
                let (x, y, width, height, coord_type) = args::<(i32, i32, i32, i32, u32)>(body)?;
                reply((Self::set_extents_async(&node, x, y, width, height, coord_type),))
            }
            "SetPosition" => {
                let (x, y, coord_type) = args::<(i32, i32, u32)>(body)?;
                reply((Self::set_position_async(&node, x, y, coord_type),))
            }
            "SetSize" => {
                let (width, height) = args::<(i32, i32)>(body)?;
                reply((Self::set_size_async(width, height),))
            }
            "ScrollTo" => reply((Self::scroll_to_async(args::<u32>(body)?),)),
            "ScrollToPoint" => {
                let (coord_type, x, y) = args::<(u32, i32, i32)>(body)?;
                reply((Self::scroll_to_point_async(coord_type, x, y),))
            }
            _ => Err(DBusError::unknown_method()),
        }
    }

    fn get_property(&self, name: &str) -> Option<Value<'static>> {
        (name == "version").then(|| Value::from(Self::version()))
    }
}
