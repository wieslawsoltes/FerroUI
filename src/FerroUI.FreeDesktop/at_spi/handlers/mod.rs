//! The handlers of the interfaces of a node (`Handlers/` of the reference).

pub(crate) mod at_spi_accessible_handler;
pub(crate) mod at_spi_action_handler;
pub(crate) mod at_spi_collection_handler;
pub(crate) mod at_spi_component_handler;
pub(crate) mod at_spi_coordinate_helper;
pub(crate) mod at_spi_editable_text_handler;
pub(crate) mod at_spi_event_object_handler;
pub(crate) mod at_spi_event_window_handler;
pub(crate) mod at_spi_image_handler;
pub(crate) mod at_spi_selection_handler;
pub(crate) mod at_spi_text_handler;
pub(crate) mod at_spi_value_handler;

#[cfg(test)]
mod at_spi_accessible_handler_tests;

use super::at_spi_node::AtSpiNode;
use super::at_spi_server::AtSpiServer;
use super::dbus::interface::{DBusError, ERROR_UNKNOWN_OBJECT};
use std::rc::{Rc, Weak};

/// The node of a handler. A handler holds its node weakly (the node owns
/// its handlers); a call that arrives for a node that is gone is
/// answered as a call for an object that does not exist.
pub(crate) fn node_of(node: &Weak<AtSpiNode>) -> Result<Rc<AtSpiNode>, DBusError> {
    let node = node.upgrade().ok_or_else(|| DBusError::new(ERROR_UNKNOWN_OBJECT, "The object is gone."))?;
    // The peer of a control that is gone is not an element any more (DEVIATIONS.md,
    // Automation peers): the same answer as for a node that is gone.
    if !node.peer().is_element_available() {
        return Err(DBusError::new(ERROR_UNKNOWN_OBJECT, "The element is gone."));
    }
    Ok(node)
}

/// The server of a handler: see [`node_of`].
pub(crate) fn server_of(server: &Weak<AtSpiServer>) -> Result<Rc<AtSpiServer>, DBusError> {
    server.upgrade().ok_or_else(|| DBusError::new(ERROR_UNKNOWN_OBJECT, "The accessibility server is gone."))
}

/// A count or an index as the `i` of a reply.
pub(crate) fn to_i32(value: usize) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

/// The item at an index a call gave; `None` for an index out of range.
pub(crate) fn item_at<T>(items: &[T], index: i32) -> Option<&T> {
    usize::try_from(index).ok().and_then(|index| items.get(index))
}
