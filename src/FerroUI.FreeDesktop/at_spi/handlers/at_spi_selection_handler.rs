//! The port of `AtSpiSelectionHandler.cs`: `org.a11y.atspi.Selection`.

use super::{item_at, node_of, server_of, to_i32};
use crate::at_spi::at_spi_constants::SELECTION_VERSION;
use crate::at_spi::at_spi_node::AtSpiNode;
use crate::at_spi::at_spi_server::AtSpiServer;
use crate::at_spi::dbus::descriptions::SELECTION;
use crate::at_spi::dbus::interface::{args, reply, CallResult, DBusError, DBusInterface, InterfaceDescription};
use crate::at_spi::dbus::types::AtSpiObjectReference;
use ferroui_base::Ref;
use ferroui_controls::automation::peers::AutomationPeer;
use ferroui_controls::automation::provider::{ISelectionItemProvider, ISelectionProvider};
use std::rc::{Rc, Weak};
use zbus::zvariant::Value;

pub(crate) struct AtSpiSelectionHandler {
    server: Weak<AtSpiServer>,
    node: Weak<AtSpiNode>,
}

impl AtSpiSelectionHandler {
    pub(crate) fn new(server: Weak<AtSpiServer>, node: Weak<AtSpiNode>) -> Self {
        Self { server, node }
    }

    fn provider(node: &AtSpiNode) -> Option<Rc<dyn ISelectionProvider>> {
        node.peer().get_provider::<dyn ISelectionProvider>()
    }

    fn get_selected_child(server: &AtSpiServer, node: &AtSpiNode, selected_child_index: i32) -> AtSpiObjectReference {
        node.ensure_children();
        let Some(provider) = Self::provider(node) else { return server.get_null_reference() };

        let selection = provider.get_selection();
        let Some(selected_peer) = item_at(&selection, selected_child_index) else {
            return server.get_null_reference();
        };

        match server.try_get_attached_node(Some(selected_peer)) {
            Some(child_node) => server.get_reference(Some(&child_node)),
            None => server.get_null_reference(),
        }
    }

    fn select_child(node: &AtSpiNode, child_index: i32) -> Result<bool, DBusError> {
        let items = Self::collect_selectable_items(node.peer());
        let Some(item) = item_at(&items, child_index) else { return Ok(false) };
        item.add_to_selection().map_err(DBusError::failed)?;
        Ok(true)
    }

    fn deselect_selected_child(node: &AtSpiNode, selected_child_index: i32) -> Result<bool, DBusError> {
        let Some(provider) = Self::provider(node) else { return Ok(false) };

        let selection = provider.get_selection();
        let Some(selected_peer) = item_at(&selection, selected_child_index) else { return Ok(false) };
        let Some(selection_item) = selected_peer.get_provider::<dyn ISelectionItemProvider>() else {
            return Ok(false);
        };

        selection_item.remove_from_selection().map_err(DBusError::failed)?;
        Ok(true)
    }

    fn is_child_selected(node: &AtSpiNode, child_index: i32) -> bool {
        let items = Self::collect_selectable_items(node.peer());
        item_at(&items, child_index).is_some_and(|item| item.is_selected())
    }

    fn select_all(node: &AtSpiNode) -> Result<bool, DBusError> {
        if !Self::provider(node).is_some_and(|provider| provider.can_select_multiple()) {
            return Ok(false);
        }

        for item in Self::collect_selectable_items(node.peer()) {
            item.add_to_selection().map_err(DBusError::failed)?;
        }
        Ok(true)
    }

    fn clear_selection(node: &AtSpiNode) -> Result<bool, DBusError> {
        let Some(provider) = Self::provider(node) else { return Ok(false) };

        for selected_peer in provider.get_selection() {
            if let Some(selection_item) = selected_peer.get_provider::<dyn ISelectionItemProvider>() {
                selection_item.remove_from_selection().map_err(DBusError::failed)?;
            }
        }
        Ok(true)
    }

    fn deselect_child(node: &AtSpiNode, child_index: i32) -> Result<bool, DBusError> {
        let items = Self::collect_selectable_items(node.peer());
        let Some(item) = item_at(&items, child_index) else { return Ok(false) };
        item.remove_from_selection().map_err(DBusError::failed)?;
        Ok(true)
    }

    fn collect_selectable_items(peer: &AutomationPeer) -> Vec<Rc<dyn ISelectionItemProvider>> {
        let mut result = Vec::new();
        Self::collect_selectable_items_core(&peer.get_children(), &mut result);
        result
    }

    fn collect_selectable_items_core(
        children: &[Ref<AutomationPeer>],
        result: &mut Vec<Rc<dyn ISelectionItemProvider>>,
    ) {
        for child in children {
            match child.get_provider::<dyn ISelectionItemProvider>() {
                Some(item) => result.push(item),
                None => Self::collect_selectable_items_core(&child.get_children(), result),
            }
        }
    }
}

impl DBusInterface for AtSpiSelectionHandler {
    fn description(&self) -> &'static InterfaceDescription {
        &SELECTION
    }

    fn call(&self, member: &str, body: &zbus::message::Body) -> CallResult {
        let (server, node) = (server_of(&self.server)?, node_of(&self.node)?);
        match member {
            "GetSelectedChild" => reply((Self::get_selected_child(&server, &node, args::<i32>(body)?).to_wire(),)),
            "SelectChild" => reply((Self::select_child(&node, args::<i32>(body)?)?,)),
            "DeselectSelectedChild" => reply((Self::deselect_selected_child(&node, args::<i32>(body)?)?,)),
            "IsChildSelected" => reply((Self::is_child_selected(&node, args::<i32>(body)?),)),
            "SelectAll" => reply((Self::select_all(&node)?,)),
            "ClearSelection" => reply((Self::clear_selection(&node)?,)),
            "DeselectChild" => reply((Self::deselect_child(&node, args::<i32>(body)?)?,)),
            _ => Err(DBusError::unknown_method()),
        }
    }

    fn get_property(&self, name: &str) -> Option<Value<'static>> {
        let node = self.node.upgrade()?;
        Some(match name {
            "version" => Value::from(SELECTION_VERSION),
            "NSelectedChildren" => {
                Value::from(Self::provider(&node).map_or(0, |provider| to_i32(provider.get_selection().len())))
            }
            _ => return None,
        })
    }
}
