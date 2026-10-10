//! The port of `ApplicationAccessibleHandler.cs`: `org.a11y.atspi.Accessible`
//! of the root object of the application.

use super::application_at_spi_node::ApplicationAtSpiNode;
use super::at_spi_constants::*;
use super::at_spi_server::AtSpiServer;
use super::at_spi_state::AtSpiState;
use super::dbus::descriptions::ACCESSIBLE;
use super::dbus::interface::{args, reply, CallResult, DBusError, DBusInterface, InterfaceDescription};
use super::dbus::types::{AtSpiAttributeSet, AtSpiObjectReference, AtSpiRelationEntry};
use std::rc::{Rc, Weak};
use zbus::zvariant::Value;

pub(crate) struct ApplicationAccessibleHandler {
    server: Weak<AtSpiServer>,
    app_node: Rc<ApplicationAtSpiNode>,
}

impl ApplicationAccessibleHandler {
    pub(crate) fn new(server: Weak<AtSpiServer>, app_node: Rc<ApplicationAtSpiNode>) -> Self {
        Self { server, app_node }
    }

    fn interfaces() -> Vec<String> {
        vec![IFACE_ACCESSIBLE.to_string(), IFACE_APPLICATION.to_string()]
    }

    fn version(&self) -> u32 {
        ACCESSIBLE_VERSION
    }

    fn name(&self) -> String {
        self.app_node.name().to_string()
    }

    fn description(&self) -> String {
        String::new()
    }

    fn child_count(&self) -> i32 {
        i32::try_from(self.app_node.window_children().len()).unwrap_or(i32::MAX)
    }

    fn locale(&self) -> String {
        resolve_locale()
    }

    fn accessible_id(&self) -> String {
        String::new()
    }

    fn help_text(&self) -> String {
        String::new()
    }

    fn get_index_in_parent_async(&self) -> i32 {
        -1
    }

    fn get_relation_set_async(&self) -> Vec<AtSpiRelationEntry> {
        Vec::new()
    }

    fn get_role_async(&self) -> u32 {
        self.app_node.role() as u32
    }

    fn get_role_name_async(&self) -> &'static str {
        "application"
    }

    fn get_localized_role_name_async(&self) -> &'static str {
        "application"
    }

    fn get_state_async(&self) -> Vec<u32> {
        build_state_set(&[AtSpiState::Active])
    }

    fn get_attributes_async(&self) -> AtSpiAttributeSet {
        AtSpiAttributeSet::new()
    }

    fn get_application_async(&self, server: &AtSpiServer) -> AtSpiObjectReference {
        server.get_root_reference()
    }

    fn get_interfaces_async(&self) -> Vec<String> {
        Self::interfaces()
    }

    fn parent(&self) -> AtSpiObjectReference {
        AtSpiObjectReference::new("", NULL_PATH)
    }

    fn get_child_at_index_async(&self, server: &AtSpiServer, index: i32) -> AtSpiObjectReference {
        let children = self.app_node.window_children();
        match usize::try_from(index).ok().and_then(|index| children.get(index)) {
            Some(child) => server.get_reference(Some(child)),
            None => server.get_null_reference(),
        }
    }

    fn get_children_async(&self, server: &AtSpiServer) -> Vec<AtSpiObjectReference> {
        self.app_node.window_children().iter().map(|child| server.get_reference(Some(child))).collect()
    }
}

impl DBusInterface for ApplicationAccessibleHandler {
    fn description(&self) -> &'static InterfaceDescription {
        &ACCESSIBLE
    }

    fn call(&self, member: &str, body: &zbus::message::Body) -> CallResult {
        let server = self.server.upgrade().ok_or_else(|| DBusError::failed("The accessibility server is gone."))?;
        match member {
            "GetChildAtIndex" => reply((self.get_child_at_index_async(&server, args::<i32>(body)?).to_wire(),)),
            "GetChildren" => {
                reply((self.get_children_async(&server).iter().map(AtSpiObjectReference::to_wire).collect::<Vec<_>>(),))
            }
            "GetIndexInParent" => reply((self.get_index_in_parent_async(),)),
            "GetRelationSet" => reply((self.get_relation_set_async(),)),
            "GetRole" => reply((self.get_role_async(),)),
            "GetRoleName" => reply((self.get_role_name_async(),)),
            "GetLocalizedRoleName" => reply((self.get_localized_role_name_async(),)),
            "GetState" => reply((self.get_state_async(),)),
            "GetAttributes" => reply((self.get_attributes_async(),)),
            "GetApplication" => reply((self.get_application_async(&server).to_wire(),)),
            "GetInterfaces" => reply((self.get_interfaces_async(),)),
            _ => Err(DBusError::unknown_method()),
        }
    }

    fn get_property(&self, name: &str) -> Option<Value<'static>> {
        Some(match name {
            "version" => Value::from(self.version()),
            "Name" => Value::from(self.name()),
            "Description" => Value::from(self.description()),
            "AccessibleId" => Value::from(self.accessible_id()),
            "HelpText" => Value::from(self.help_text()),
            "Parent" => self.parent().to_dbus_struct(),
            "ChildCount" => Value::from(self.child_count()),
            "Locale" => Value::from(self.locale()),
            _ => return None,
        })
    }
}
