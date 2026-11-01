// NetworkManager and UPower property endpoints on the smoke test's private bus.
// The desktop uses its production system-bus proxies against these endpoints.
#include <gio/gio.h>
#include <stdio.h>
#include <string.h>

#define NETWORK "org.freedesktop.NetworkManager"
#define POWER "org.freedesktop.UPower"
#define MANAGER "/org/freedesktop/NetworkManager"
#define ACTIVE MANAGER "/ActiveConnection/1"
#define DISPLAY_DEVICE "/org/freedesktop/UPower/devices/DisplayDevice"

static const char xml[] =
    "<node>"
    "<interface name='" NETWORK "'>"
    "<property name='WirelessEnabled' type='b' access='readwrite'/>"
    "<property name='WirelessHardwareEnabled' type='b' access='read'/>"
    "<property name='PrimaryConnection' type='o' access='read'/>"
    "</interface>"
    "<interface name='" NETWORK ".Connection.Active'>"
    "<property name='Id' type='s' access='read'/>"
    "</interface>"
    "<interface name='" POWER ".Device'>"
    "<property name='IsPresent' type='b' access='read'/>"
    "<property name='Percentage' type='d' access='read'/>"
    "<property name='State' type='u' access='read'/>"
    "</interface>"
    "</node>";

static gboolean wifi = TRUE;

static GVariant *read_property(GDBusConnection *connection, const gchar *sender, const gchar *path,
                               const gchar *interface, const gchar *property, GError **error,
                               gpointer data) {
    g_print("Read %s.%s\n", interface, property);
    if (!strcmp(property, "WirelessEnabled")) return g_variant_new_boolean(wifi);
    if (!strcmp(property, "WirelessHardwareEnabled") || !strcmp(property, "IsPresent"))
        return g_variant_new_boolean(TRUE);
    if (!strcmp(property, "PrimaryConnection")) return g_variant_new_object_path(wifi ? ACTIVE : "/");
    if (!strcmp(property, "Id")) return g_variant_new_string("Studio Wi-Fi");
    if (!strcmp(property, "Percentage")) return g_variant_new_double(76.0);
    if (!strcmp(property, "State")) return g_variant_new_uint32(2);
    g_set_error(error, G_DBUS_ERROR, G_DBUS_ERROR_UNKNOWN_PROPERTY, "Unknown property %s", property);
    return NULL;
}

static gboolean write_property(GDBusConnection *connection, const gchar *sender, const gchar *path,
                               const gchar *interface, const gchar *property, GVariant *value,
                               GError **error, gpointer data) {
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
    const char *paths[] = {MANAGER, ACTIVE, DISPLAY_DEVICE};
    for (size_t i = 0; i < G_N_ELEMENTS(paths); i++) {
        if (!g_dbus_connection_register_object(bus, paths[i], node->interfaces[i], &properties,
                                               NULL, NULL, &error)) g_error("%s", error->message);
    }
    g_bus_own_name_on_connection(bus, NETWORK, G_BUS_NAME_OWNER_FLAGS_NONE, NULL, NULL, NULL, NULL);
    g_bus_own_name_on_connection(bus, POWER, G_BUS_NAME_OWNER_FLAGS_NONE, NULL, NULL, NULL, NULL);
    g_main_loop_run(g_main_loop_new(NULL, FALSE));
    return 0;
}
