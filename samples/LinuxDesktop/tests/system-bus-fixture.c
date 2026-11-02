// NetworkManager, BlueZ and UPower endpoints on the smoke test's private bus.
// The desktop uses its production system-bus proxies against these endpoints.
#include <gio/gio.h>
#include <stdio.h>
#include <string.h>

#define NETWORK "org.freedesktop.NetworkManager"
#define POWER "org.freedesktop.UPower"
#define BLUEZ "org.bluez"
#define ADAPTER BLUEZ ".Adapter1"
#define RADIO "/org/bluez/hci0"
#define MANAGER "/org/freedesktop/NetworkManager"
#define ACTIVE MANAGER "/ActiveConnection/1"
#define WIRED MANAGER "/ActiveConnection/2"
#define DISPLAY_DEVICE "/org/freedesktop/UPower/devices/DisplayDevice"

static const char xml[] =
    "<node>"
    "<interface name='" NETWORK "'>"
    "<property name='WirelessEnabled' type='b' access='readwrite'/>"
    "<property name='WirelessHardwareEnabled' type='b' access='read'/>"
    "<property name='ActiveConnections' type='ao' access='read'/>"
    "<property name='PrimaryConnection' type='o' access='read'/>"
    "</interface>"
    "<interface name='" NETWORK ".Connection.Active'>"
    "<property name='Id' type='s' access='read'/>"
    "<property name='Type' type='s' access='read'/>"
    "<property name='State' type='u' access='read'/>"
    "</interface>"
    "<interface name='" POWER ".Device'>"
    "<property name='IsPresent' type='b' access='read'/>"
    "<property name='Percentage' type='d' access='read'/>"
    "<property name='State' type='u' access='read'/>"
    "</interface>"
    "<interface name='" ADAPTER "'>"
    "<property name='Powered' type='b' access='readwrite'/>"
    "</interface>"
    "<interface name='org.freedesktop.DBus.ObjectManager'>"
    "<method name='GetManagedObjects'><arg type='a{oa{sa{sv}}}' direction='out'/></method>"
    "</interface>"
    "</node>";

static gboolean wifi = TRUE;
static gboolean powered = TRUE;

static void managed_objects(GDBusConnection *connection, const gchar *sender, const gchar *path,
                            const gchar *interface, const gchar *method, GVariant *args,
                            GDBusMethodInvocation *invocation, gpointer data) {
    GVariantBuilder properties, interfaces, objects;
    g_variant_builder_init(&properties, G_VARIANT_TYPE("a{sv}"));
    g_variant_builder_add(&properties, "{sv}", "Powered", g_variant_new_boolean(powered));
    g_variant_builder_init(&interfaces, G_VARIANT_TYPE("a{sa{sv}}"));
    g_variant_builder_add(&interfaces, "{s@a{sv}}", ADAPTER, g_variant_builder_end(&properties));
    g_variant_builder_init(&objects, G_VARIANT_TYPE("a{oa{sa{sv}}}"));
    g_variant_builder_add(&objects, "{o@a{sa{sv}}}", RADIO, g_variant_builder_end(&interfaces));
    g_dbus_method_invocation_return_value(invocation,
        g_variant_new("(@a{oa{sa{sv}}})", g_variant_builder_end(&objects)));
}

static GVariant *read_property(GDBusConnection *connection, const gchar *sender, const gchar *path,
                               const gchar *interface, const gchar *property, GError **error,
                               gpointer data) {
    g_print("Read %s.%s\n", interface, property);
    if (!strcmp(property, "Powered")) return g_variant_new_boolean(powered);
    if (!strcmp(property, "WirelessEnabled")) return g_variant_new_boolean(wifi);
    if (!strcmp(property, "WirelessHardwareEnabled") || !strcmp(property, "IsPresent"))
        return g_variant_new_boolean(TRUE);
    if (!strcmp(property, "ActiveConnections")) {
        const gchar *ids[] = {WIRED, ACTIVE, NULL};
        return g_variant_new_objv(ids, wifi ? 2 : 1);
    }
    if (!strcmp(property, "PrimaryConnection")) return g_variant_new_object_path(WIRED);
    if (!strcmp(property, "Type")) return g_variant_new_string(!strcmp(path, ACTIVE) ? "802-11-wireless" : "802-3-ethernet");
    if (!strcmp(property, "Id")) {
        g_print("Read connection ID at %s\n", path);
        return g_variant_new_string(!strcmp(path, ACTIVE) ? "Studio Wi-Fi" : "Office Ethernet");
    }
    if (!strcmp(property, "Percentage")) return g_variant_new_double(76.0);
    if (!strcmp(property, "State")) return g_variant_new_uint32(2);
    g_set_error(error, G_DBUS_ERROR, G_DBUS_ERROR_UNKNOWN_PROPERTY, "Unknown property %s", property);
    return NULL;
}

static gboolean write_property(GDBusConnection *connection, const gchar *sender, const gchar *path,
                               const gchar *interface, const gchar *property, GVariant *value,
                               GError **error, gpointer data) {
    if (!strcmp(interface, ADAPTER) && !strcmp(property, "Powered")) {
        powered = g_variant_get_boolean(value);
        g_print("Set Bluetooth Powered=%d\n", powered);
        return TRUE;
    }
    if (strcmp(interface, NETWORK) || strcmp(property, "WirelessEnabled")) {
        g_set_error_literal(error, G_DBUS_ERROR, G_DBUS_ERROR_PROPERTY_READ_ONLY, "Read only");
        return FALSE;
    }
    wifi = g_variant_get_boolean(value);
    g_print("Set WirelessEnabled=%d\n", wifi);
    return TRUE;
}

int main(void) {
    setvbuf(stdout, NULL, _IOLBF, 0);
    GError *error = NULL;
    GDBusConnection *bus = g_bus_get_sync(G_BUS_TYPE_SESSION, NULL, &error);
    if (!bus) g_error("%s", error->message);
    GDBusNodeInfo *node = g_dbus_node_info_new_for_xml(xml, &error);
    if (!node) g_error("%s", error->message);
    const GDBusInterfaceVTable properties = {.get_property = read_property, .set_property = write_property};
    const char *paths[] = {MANAGER, ACTIVE, DISPLAY_DEVICE, RADIO};
    for (size_t i = 0; i < G_N_ELEMENTS(paths); i++) {
        if (!g_dbus_connection_register_object(bus, paths[i], node->interfaces[i], &properties,
                                               NULL, NULL, &error)) g_error("%s", error->message);
    }
    if (!g_dbus_connection_register_object(bus, WIRED, node->interfaces[1], &properties, NULL, NULL, &error))
        g_error("%s", error->message);
    const GDBusInterfaceVTable manager = {.method_call = managed_objects};
    if (!g_dbus_connection_register_object(bus, "/", node->interfaces[4], &manager, NULL, NULL, &error))
        g_error("%s", error->message);
    g_bus_own_name_on_connection(bus, BLUEZ, G_BUS_NAME_OWNER_FLAGS_NONE, NULL, NULL, NULL, NULL);
    g_bus_own_name_on_connection(bus, NETWORK, G_BUS_NAME_OWNER_FLAGS_NONE, NULL, NULL, NULL, NULL);
    g_bus_own_name_on_connection(bus, POWER, G_BUS_NAME_OWNER_FLAGS_NONE, NULL, NULL, NULL, NULL);
    g_main_loop_run(g_main_loop_new(NULL, FALSE));
    return 0;
}
