//! `org.freedesktop.DBus.Properties` of a path with registered objects
//! (the port of `BuiltInPropertiesHandler.cs` of the reference's D-Bus
//! library).

use super::connection::ObjectTable;
use super::interface::{
    reply, CallResult, DBusError, ERROR_INVALID_ARGS, ERROR_UNKNOWN_INTERFACE, ERROR_UNKNOWN_PROPERTY,
};
use std::collections::HashMap;
use std::rc::Rc;
use zbus::zvariant::{OwnedValue, Value};

use super::interface::DBusInterface;

fn find(table: &ObjectTable, path: &str, interface: &str) -> Result<Rc<dyn DBusInterface>, DBusError> {
    table
        .interfaces_of(path)
        .into_iter()
        .find(|handler| handler.description().name == interface)
        .ok_or_else(|| DBusError::new(ERROR_UNKNOWN_INTERFACE, "Unknown interface"))
}

pub(crate) fn handle(table: &ObjectTable, path: &str, member: &str, body: &zbus::message::Body) -> CallResult {
    match member {
        "Get" => {
            let Ok((interface, property)) = body.deserialize::<(String, String)>() else {
                return Err(DBusError::new(ERROR_INVALID_ARGS, "Invalid Get arguments."));
            };
            let handler = find(table, path, &interface)?;
            match handler.get_property(&property) {
                Some(value) => reply((value,)),
                None => Err(DBusError::new(ERROR_UNKNOWN_PROPERTY, "Unknown property")),
            }
        }
        "GetAll" => {
            let Ok(interface) = body.deserialize::<String>() else {
                return Err(DBusError::new(ERROR_INVALID_ARGS, "Invalid GetAll arguments."));
            };
            let handler = find(table, path, &interface)?;
            let all: HashMap<String, Value<'static>> =
                handler.get_all_properties().into_iter().map(|(name, value)| (name.to_string(), value)).collect();
            reply((all,))
        }
        "Set" => {
            let Ok((interface, property, value)) = body.deserialize::<(String, String, OwnedValue)>() else {
                return Err(DBusError::new(ERROR_INVALID_ARGS, "Invalid Set arguments."));
            };
            let handler = find(table, path, &interface)?;
            if !handler.set_property(&property, &value) {
                return Err(DBusError::new(ERROR_UNKNOWN_PROPERTY, "Unknown property"));
            }
            reply(())
        }
        _ => Err(DBusError::unknown_method()),
    }
}
