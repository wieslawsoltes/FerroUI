//! What a method handler of an interface is: the shape of the handlers
//! the reference generates from `DBusXml/` (`IDBusInterfaceCallDispatcher`
//! of its D-Bus library, with the property accessors and the
//! introspection writer of a generated handler).
//!
//! A handler of the reference is an interface (`IOrgA11yAtspiAccessible`)
//! implemented by a class, and generated code that reads the arguments of
//! a call, calls the member and writes the reply. Here the class
//! implements [`DBusInterface`]: `call` is that generated code, written by
//! hand from the same descriptions, and the members it calls are the
//! methods of the class.

use zbus::export::serde::ser::Serialize;
use zbus::export::serde::Deserialize;
use zbus::zvariant::{DynamicType, Type, Value};

pub(crate) const ERROR_UNKNOWN_METHOD: &str = "org.freedesktop.DBus.Error.UnknownMethod";
pub(crate) const ERROR_UNKNOWN_INTERFACE: &str = "org.freedesktop.DBus.Error.UnknownInterface";
pub(crate) const ERROR_UNKNOWN_PROPERTY: &str = "org.freedesktop.DBus.Error.UnknownProperty";
pub(crate) const ERROR_UNKNOWN_OBJECT: &str = "org.freedesktop.DBus.Error.UnknownObject";
pub(crate) const ERROR_INVALID_ARGS: &str = "org.freedesktop.DBus.Error.InvalidArgs";
pub(crate) const ERROR_FAILED: &str = "org.freedesktop.DBus.Error.Failed";

/// An error reply (`DBusException`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DBusError {
    pub(crate) name: String,
    pub(crate) message: String,
}

impl DBusError {
    pub(crate) fn new(name: &str, message: impl Into<String>) -> Self {
        Self { name: name.to_string(), message: message.into() }
    }

    pub(crate) fn unknown_method() -> Self {
        Self::new(ERROR_UNKNOWN_METHOD, "Unknown method")
    }

    /// The error of a member that failed: the reference answers an
    /// exception that is not a D-Bus one with `Failed` and its message.
    pub(crate) fn failed(error: impl std::fmt::Display) -> Self {
        Self::new(ERROR_FAILED, error.to_string())
    }
}

/// The body of a reply, of whatever type it was made from.
pub(crate) trait ReplyBody {
    fn build(&self, builder: zbus::message::Builder<'_>) -> zbus::Result<zbus::Message>;
}

impl<B: Serialize + DynamicType> ReplyBody for B {
    fn build(&self, builder: zbus::message::Builder<'_>) -> zbus::Result<zbus::Message> {
        builder.build(self)
    }
}

pub(crate) type Reply = Box<dyn ReplyBody>;
pub(crate) type CallResult = Result<Reply, DBusError>;

/// A reply with the out arguments `body` (a tuple of them; `()` for none).
pub(crate) fn reply<B: Serialize + DynamicType + 'static>(body: B) -> CallResult {
    Ok(Box::new(body))
}

/// The in arguments of a call, as the generated handlers read them: a
/// body that does not have them is `InvalidArgs`.
pub(crate) fn args<'a, T>(body: &'a zbus::message::Body) -> Result<T, DBusError>
where
    T: Deserialize<'a> + Type,
{
    body.deserialize::<T>().map_err(|e| DBusError::new(ERROR_INVALID_ARGS, e.to_string()))
}

/// A method of a description: its name, the signatures of its in
/// arguments and of its out arguments, each list separated by spaces.
pub(crate) type MethodDescription = (&'static str, &'static str, &'static str);

/// An interface as `DBusXml/` describes it: what introspection answers.
pub(crate) struct InterfaceDescription {
    pub(crate) name: &'static str,
    pub(crate) methods: &'static [MethodDescription],
    /// Name, signature, and whether it can be written.
    pub(crate) properties: &'static [(&'static str, &'static str, bool)],
    /// Name and the signatures of the arguments.
    pub(crate) signals: &'static [(&'static str, &'static str)],
}

impl InterfaceDescription {
    /// The description as introspection data (the `WriteIntrospectionXml`
    /// of a generated handler).
    pub(crate) fn write_introspection_xml(&self, out: &mut String, indent: &str) {
        out.push_str(&format!("{indent}<interface name=\"{}\">\n", self.name));
        for (name, ins, outs) in self.methods {
            out.push_str(&format!("{indent}  <method name=\"{name}\">\n"));
            for signature in ins.split_whitespace() {
                out.push_str(&format!("{indent}    <arg type=\"{signature}\" direction=\"in\"/>\n"));
            }
            for signature in outs.split_whitespace() {
                out.push_str(&format!("{indent}    <arg type=\"{signature}\" direction=\"out\"/>\n"));
            }
            out.push_str(&format!("{indent}  </method>\n"));
        }
        for (name, signature, writable) in self.properties {
            let access = if *writable { "readwrite" } else { "read" };
            out.push_str(&format!(
                "{indent}  <property name=\"{name}\" type=\"{signature}\" access=\"{access}\"/>\n"
            ));
        }
        for (name, arguments) in self.signals {
            out.push_str(&format!("{indent}  <signal name=\"{name}\">\n"));
            for signature in arguments.split_whitespace() {
                out.push_str(&format!("{indent}    <arg type=\"{signature}\"/>\n"));
            }
            out.push_str(&format!("{indent}  </signal>\n"));
        }
        out.push_str(&format!("{indent}</interface>\n"));
    }
}

/// A handler of one interface at one object path.
pub(crate) trait DBusInterface {
    fn description(&self) -> &'static InterfaceDescription;

    /// Answers a method call of the interface.
    fn call(&self, member: &str, body: &zbus::message::Body) -> CallResult {
        let _ = (member, body);
        Err(DBusError::unknown_method())
    }

    /// The value of a property (`TryGetProperty`); `None` for a name the
    /// interface does not have.
    fn get_property(&self, name: &str) -> Option<Value<'static>> {
        let _ = name;
        None
    }

    /// Every property (`GetAllProperties`), in the order of the description.
    fn get_all_properties(&self) -> Vec<(&'static str, Value<'static>)> {
        self.description()
            .properties
            .iter()
            .filter_map(|(name, _, _)| self.get_property(name).map(|value| (*name, value)))
            .collect()
    }

    /// Sets a property (`TrySetProperty`); false for a property that does
    /// not exist, cannot be written or was given a value of another type.
    fn set_property(&self, name: &str, value: &Value<'_>) -> bool {
        let _ = (name, value);
        false
    }
}
