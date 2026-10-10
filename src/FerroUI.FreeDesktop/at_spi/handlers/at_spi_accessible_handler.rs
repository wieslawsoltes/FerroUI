//! The port of `AtSpiAccessibleHandler.cs`: `org.a11y.atspi.Accessible`
//! of a node.

use super::{item_at, node_of, server_of, to_i32};
use crate::at_spi::at_spi_constants::*;
use crate::at_spi::at_spi_node::AtSpiNode;
use crate::at_spi::at_spi_server::AtSpiServer;
use crate::at_spi::dbus::descriptions::ACCESSIBLE;
use crate::at_spi::dbus::interface::{args, reply, CallResult, DBusError, DBusInterface, InterfaceDescription};
use crate::at_spi::dbus::types::{AtSpiAttributeSet, AtSpiObjectReference, AtSpiRelationEntry};
use std::rc::{Rc, Weak};
use zbus::zvariant::Value;

pub(crate) struct AtSpiAccessibleHandler {
    server: Weak<AtSpiServer>,
    node: Weak<AtSpiNode>,
}

impl AtSpiAccessibleHandler {
    pub(crate) fn new(server: Weak<AtSpiServer>, node: Weak<AtSpiNode>) -> Self {
        Self { server, node }
    }

    pub(crate) fn name(node: &AtSpiNode) -> String {
        AtSpiNode::get_accessible_name(node.peer())
    }

    pub(crate) fn parent(server: &AtSpiServer, node: &Rc<AtSpiNode>) -> AtSpiObjectReference {
        // For window nodes, return the ApplicationAtSpiNode as parent
        if let Some(app_root) = node.as_root().and_then(|root| root.app_root()) {
            return AtSpiObjectReference::new(server.unique_name(), app_root.path());
        }

        server.get_reference(node.parent().as_ref())
    }

    pub(crate) fn get_child_at_index(server: &AtSpiServer, node: &AtSpiNode, index: i32) -> AtSpiObjectReference {
        let children = node.ensure_children();
        match item_at(&children, index) {
            Some(child) => server.get_reference(Some(child)),
            None => server.get_null_reference(),
        }
    }

    pub(crate) fn get_children(server: &AtSpiServer, node: &AtSpiNode) -> Vec<AtSpiObjectReference> {
        node.ensure_children().iter().map(|child| server.get_reference(Some(child))).collect()
    }

    pub(crate) fn get_index_in_parent(node: &Rc<AtSpiNode>) -> i32 {
        // Window nodes are children of the ApplicationAtSpiNode, but their
        // internal Parent field is null (they are attached with parent: null).
        // Mirror the Parent property's special case so that backward path
        // walks (e.g. accerciser's get_index_in_parent) work correctly.
        if let Some(app_root) = node.as_root().and_then(|root| root.app_root()) {
            return app_root.window_children().iter().position(|window| Rc::ptr_eq(window, node)).map_or(-1, to_i32);
        }

        let Some(parent) = node.parent() else { return -1 };
        parent.ensure_children().iter().position(|sibling| Rc::ptr_eq(sibling, node)).map_or(-1, to_i32)
    }

    pub(crate) fn get_relation_set(server: &AtSpiServer, node: &AtSpiNode) -> Vec<AtSpiRelationEntry> {
        let mut relations = Vec::new();

        if let Some(labeled_by) = node.peer().get_labeled_by() {
            if let Some(label_node) = server.try_get_attached_node(Some(&labeled_by)) {
                // Relation type 2 = LABELLED_BY
                relations.push((2u32, vec![server.get_reference(Some(&label_node)).to_wire()]));
            }
        }

        relations
    }

    pub(crate) fn get_role(node: &AtSpiNode) -> u32 {
        AtSpiNode::to_at_spi_role(node.peer().get_automation_control_type(), Some(node.peer())) as u32
    }

    pub(crate) fn get_role_name(node: &AtSpiNode) -> &'static str {
        let role = AtSpiNode::to_at_spi_role(node.peer().get_automation_control_type(), Some(node.peer()));
        AtSpiNode::to_at_spi_role_name(role)
    }

    pub(crate) fn get_attributes(node: &AtSpiNode) -> AtSpiAttributeSet {
        let peer = node.peer();
        let mut attrs = AtSpiAttributeSet::new();
        attrs.insert("toolkit".to_string(), TOOLKIT_NAME.to_string());

        if !peer.get_name().is_empty() {
            attrs.insert("explicit-name".to_string(), "true".to_string());
        }

        if let Some(accelerator_key) = peer.get_accelerator_key().filter(|key| !key.is_empty()) {
            attrs.insert("accelerator-key".to_string(), accelerator_key);
        }

        if let Some(access_key) = peer.get_access_key().filter(|key| !key.is_empty()) {
            attrs.insert("access-key".to_string(), access_key);
        }

        let placeholder_text = peer.get_placeholder_text();
        if !placeholder_text.is_empty() {
            attrs.insert("placeholder-text".to_string(), placeholder_text);
        }

        attrs
    }

    pub(crate) fn get_interfaces(node: &AtSpiNode) -> Vec<String> {
        // The set is ordered: the interfaces sorted by their ordinal names.
        node.get_supported_interfaces().into_iter().map(str::to_string).collect()
    }
}

impl DBusInterface for AtSpiAccessibleHandler {
    fn description(&self) -> &'static InterfaceDescription {
        &ACCESSIBLE
    }

    fn call(&self, member: &str, body: &zbus::message::Body) -> CallResult {
        let (server, node) = (server_of(&self.server)?, node_of(&self.node)?);
        match member {
            "GetChildAtIndex" => reply((Self::get_child_at_index(&server, &node, args::<i32>(body)?).to_wire(),)),
            "GetChildren" => reply((Self::get_children(&server, &node)
                .iter()
                .map(AtSpiObjectReference::to_wire)
                .collect::<Vec<_>>(),)),
            "GetIndexInParent" => reply((Self::get_index_in_parent(&node),)),
            "GetRelationSet" => reply((Self::get_relation_set(&server, &node),)),
            "GetRole" => reply((Self::get_role(&node),)),
            "GetRoleName" | "GetLocalizedRoleName" => reply((Self::get_role_name(&node),)),
            "GetState" => reply((node.compute_states(),)),
            "GetAttributes" => reply((Self::get_attributes(&node),)),
            "GetApplication" => reply((server.get_root_reference().to_wire(),)),
            "GetInterfaces" => reply((Self::get_interfaces(&node),)),
            _ => Err(DBusError::unknown_method()),
        }
    }

    fn get_property(&self, name: &str) -> Option<Value<'static>> {
        let (server, node) = (self.server.upgrade()?, self.node.upgrade()?);
        Some(match name {
            "version" => Value::from(ACCESSIBLE_VERSION),
            "Name" => Value::from(Self::name(&node)),
            "Description" | "HelpText" => Value::from(node.peer().get_help_text()),
            "Parent" => Self::parent(&server, &node).to_dbus_struct(),
            "ChildCount" => Value::from(to_i32(node.ensure_children().len())),
            "Locale" => Value::from(resolve_locale()),
            "AccessibleId" => Value::from(node.peer().get_automation_id().unwrap_or_default()),
            _ => return None,
        })
    }
}
