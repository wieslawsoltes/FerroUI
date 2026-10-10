//! `org.freedesktop.DBus.Introspectable` (the port of
//! `BuiltInIntrospectionHandler.cs` of the reference's D-Bus library).

use super::interface::{reply, CallResult, DBusError, DBusInterface};
use std::collections::BTreeSet;
use std::rc::Rc;

/// What introspection of a path shows.
pub(crate) struct IntrospectionData {
    pub(crate) interfaces: Vec<Rc<dyn DBusInterface>>,
    pub(crate) child_segments: BTreeSet<String>,
}

pub(crate) fn handle(path: &str, member: &str, data: &IntrospectionData) -> CallResult {
    if member != "Introspect" {
        return Err(DBusError::unknown_method());
    }

    reply((build_xml(path, data),))
}

pub(crate) fn build_xml(path: &str, data: &IntrospectionData) -> String {
    let mut sb = String::new();
    sb.push_str("<!DOCTYPE node PUBLIC \"-//freedesktop//DTD D-BUS Object Introspection 1.0//EN\"\n");
    sb.push_str(" \"http://www.freedesktop.org/standards/dbus/1.0/introspect.dtd\">\n");
    sb.push_str(&format!("<node name=\"{}\">\n", escape_xml(path)));

    // Standard introspectable + properties interfaces are always present on real nodes
    if !data.interfaces.is_empty() {
        sb.push_str("  <interface name=\"org.freedesktop.DBus.Introspectable\">\n");
        sb.push_str("    <method name=\"Introspect\">\n");
        sb.push_str("      <arg name=\"xml_data\" type=\"s\" direction=\"out\"/>\n");
        sb.push_str("    </method>\n");
        sb.push_str("  </interface>\n");
        sb.push_str("  <interface name=\"org.freedesktop.DBus.Properties\">\n");
        sb.push_str("    <method name=\"Get\">\n");
        sb.push_str("      <arg name=\"interface_name\" type=\"s\" direction=\"in\"/>\n");
        sb.push_str("      <arg name=\"property_name\" type=\"s\" direction=\"in\"/>\n");
        sb.push_str("      <arg name=\"value\" type=\"v\" direction=\"out\"/>\n");
        sb.push_str("    </method>\n");
        sb.push_str("    <method name=\"GetAll\">\n");
        sb.push_str("      <arg name=\"interface_name\" type=\"s\" direction=\"in\"/>\n");
        sb.push_str("      <arg name=\"value\" type=\"a{sv}\" direction=\"out\"/>\n");
        sb.push_str("    </method>\n");
        sb.push_str("    <method name=\"Set\">\n");
        sb.push_str("      <arg name=\"interface_name\" type=\"s\" direction=\"in\"/>\n");
        sb.push_str("      <arg name=\"property_name\" type=\"s\" direction=\"in\"/>\n");
        sb.push_str("      <arg name=\"value\" type=\"v\" direction=\"in\"/>\n");
        sb.push_str("    </method>\n");
        sb.push_str("  </interface>\n");

        for interface in &data.interfaces {
            interface.description().write_introspection_xml(&mut sb, "  ");
        }
    }

    for child in &data.child_segments {
        sb.push_str(&format!("  <node name=\"{}\"/>\n", escape_xml(child)));
    }

    sb.push_str("</node>\n");
    sb
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}
