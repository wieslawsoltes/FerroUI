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

    fn parent(&self) -> AtSpiObjectReference {
        AtSpiObjectReference::new("", NULL_PATH)
    }

    fn get_child_at_index(&self, server: &AtSpiServer, index: i32) -> AtSpiObjectReference {
        let children = self.app_node.window_children();
        match usize::try_from(index).ok().and_then(|index| children.get(index)) {
            Some(child) => server.get_reference(Some(child)),
            None => server.get_null_reference(),
        }
    }

    fn get_children(&self, server: &AtSpiServer) -> Vec<AtSpiObjectReference> {
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
            "GetChildAtIndex" => reply((self.get_child_at_index(&server, args::<i32>(body)?).to_wire(),)),
            "GetChildren" => {
                reply((self.get_children(&server).iter().map(AtSpiObjectReference::to_wire).collect::<Vec<_>>(),))
            }
            "GetIndexInParent" => reply((-1i32,)),
            "GetRelationSet" => reply((Vec::<AtSpiRelationEntry>::new(),)),
            "GetRole" => reply((self.app_node.role() as u32,)),
            "GetRoleName" | "GetLocalizedRoleName" => reply(("application",)),
            "GetState" => reply((build_state_set(&[AtSpiState::Active]),)),
            "GetAttributes" => reply((AtSpiAttributeSet::new(),)),
            "GetApplication" => reply((server.get_root_reference().to_wire(),)),
            "GetInterfaces" => reply((Self::interfaces(),)),
            _ => Err(DBusError::unknown_method()),
        }
    }

    fn get_property(&self, name: &str) -> Option<Value<'static>> {
        Some(match name {
            "version" => Value::from(ACCESSIBLE_VERSION),
            "Name" => Value::from(self.app_node.name().to_string()),
            "Description" | "AccessibleId" | "HelpText" => Value::from(String::new()),
            "Parent" => self.parent().to_dbus_struct(),
            "ChildCount" => Value::from(i32::try_from(self.app_node.window_children().len()).unwrap_or(i32::MAX)),
            "Locale" => Value::from(resolve_locale()),
            _ => return None,
        })
    }
}
