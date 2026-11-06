#define _GNU_SOURCE
#include <stdint.h>
#include <unistd.h>
#include <string.h>
#include <gio/gio.h>
#include <gio/gunixfdlist.h>
static gint portal_active = 0;
#include <pipewire/pipewire.h>
typedef struct {
    gint refs, cancelled;
    GMutex lock;
    int result, fd;
    uint32_t node;
    guint response;
    gboolean responded;
    char error[512];
    GCancellable *cancel;
} Portal;
static void portal_unref(Portal *p) {
    if (!g_atomic_int_dec_and_test(&p->refs)) return;
    if (p->fd >= 0) close(p->fd);
    g_object_unref(p->cancel); g_mutex_clear(&p->lock); g_free(p);
}
static void response(GDBusConnection *connection, const gchar *sender, const gchar *path,
    const gchar *interface, const gchar *signal, GVariant *parameters, gpointer data) {
    (void)connection;(void)sender;(void)path;(void)interface;(void)signal;
    Portal *p = data; GVariant *results;
    g_variant_get(parameters, "(u@a{sv})", &p->response, &results); g_variant_unref(results); p->responded = TRUE;
}
typedef struct { uint32_t node; int seq, done, failed; } CameraNodes;
static void registry_global(void *data, uint32_t id, uint32_t permissions, const char *type, uint32_t version, const struct spa_dict *props) {
    (void)permissions; (void)version;
    CameraNodes *nodes = data;
    if (nodes->node != PW_ID_ANY || strcmp(type, PW_TYPE_INTERFACE_Node) || !props) return;
    const char *media_class = spa_dict_lookup(props, PW_KEY_MEDIA_CLASS);
    if (media_class && !strcmp(media_class, "Video/Source")) nodes->node = id;
}
static void core_done(void *data, uint32_t id, int seq) {
    CameraNodes *n = data; if (id == PW_ID_CORE && seq == n->seq) n->done = 1;
}
static void core_error(void *data, uint32_t id, int seq, int result, const char *message) {
    (void)id;(void)seq;(void)result;(void)message; ((CameraNodes *)data)->failed = 1;
}
static const struct pw_registry_events registry_events = { PW_VERSION_REGISTRY_EVENTS, .global = registry_global };
static const struct pw_core_events core_events = { PW_VERSION_CORE_EVENTS, .done = core_done, .error = core_error };
static uint32_t authorized_node(int fd, Portal *p) {
    /* This connection sees only portal-authorized nodes. Never enumerate the
       unrestricted PipeWire server after portal denial. Each remote is consumed
       once; GStreamer receives a separately opened fresh remote connection. */
    static gsize initialized = 0;
    if (g_once_init_enter(&initialized)) { pw_init(NULL, NULL); g_once_init_leave(&initialized, 1); }
    struct pw_main_loop *loop = pw_main_loop_new(NULL);
    if (!loop) { close(fd); return PW_ID_ANY; }
    struct pw_context *context = pw_context_new(pw_main_loop_get_loop(loop), NULL, 0);
    struct pw_core *core = context ? pw_context_connect_fd(context, fd, NULL, 0) : NULL;
    if (!core) { if (!context) close(fd); if (context) pw_context_destroy(context); pw_main_loop_destroy(loop); return PW_ID_ANY; }
    CameraNodes nodes = { .node = PW_ID_ANY };
    struct spa_hook core_hook, registry_hook;
    pw_core_add_listener(core, &core_hook, &core_events, &nodes);
    struct pw_registry *registry = pw_core_get_registry(core, PW_VERSION_REGISTRY, 0);
    pw_registry_add_listener(registry, &registry_hook, &registry_events, &nodes);
    nodes.seq = pw_core_sync(core, PW_ID_CORE, 0);
    gint64 deadline = g_get_monotonic_time() + 5000000;
    while (!nodes.done && !nodes.failed && !g_atomic_int_get(&p->cancelled) && g_get_monotonic_time() < deadline)
        pw_loop_iterate(pw_main_loop_get_loop(loop), 50);
    uint32_t node = nodes.done && !nodes.failed ? nodes.node : PW_ID_ANY;
    spa_hook_remove(&registry_hook); spa_hook_remove(&core_hook);
    pw_proxy_destroy((struct pw_proxy *)registry); pw_core_disconnect(core);
    pw_context_destroy(context); pw_main_loop_destroy(loop);
    return node;
}
static int open_remote(GDBusConnection *bus, Portal *p, GError **error) {
    GVariantBuilder options; g_variant_builder_init(&options, G_VARIANT_TYPE_VARDICT);
    GUnixFDList *fds = NULL;
    GVariant *reply = g_dbus_connection_call_with_unix_fd_list_sync(bus, "org.freedesktop.portal.Desktop", "/org/freedesktop/portal/desktop",
        "org.freedesktop.portal.Camera", "OpenPipeWireRemote", g_variant_new("(a{sv})", &options), G_VARIANT_TYPE("(h)"),
        G_DBUS_CALL_FLAGS_NONE, 30000, NULL, &fds, p->cancel, error);
    if (!reply) return -1;
    gint handle; g_variant_get(reply, "(h)", &handle);
    int fd = g_unix_fd_list_get(fds, handle, error);
    g_variant_unref(reply); g_object_unref(fds); return fd;
}
static gpointer portal_worker(gpointer data) {
    Portal *p = data; GError *error = NULL; guint subscription = 0;
    char *path = NULL; GDBusConnection *bus = NULL;
    GMainContext *context = g_main_context_new(); g_main_context_push_thread_default(context);
    bus = g_bus_get_sync(G_BUS_TYPE_SESSION, p->cancel, &error);
    if (!bus) goto finish;
    char *sender = g_strdup(g_dbus_connection_get_unique_name(bus) + 1);
    for (char *c = sender; *c; c++) if (*c == '.') *c = '_';
    char *token = g_strdup_printf("ferroui_%u_%u", (unsigned)getpid(), g_random_int());
    path = g_strdup_printf("/org/freedesktop/portal/desktop/request/%s/%s", sender, token); g_free(sender);
    subscription = g_dbus_connection_signal_subscribe(bus, "org.freedesktop.portal.Desktop", "org.freedesktop.portal.Request", "Response", path, NULL, G_DBUS_SIGNAL_FLAGS_NONE, response, p, NULL);
    GVariantBuilder options; g_variant_builder_init(&options, G_VARIANT_TYPE_VARDICT);
    g_variant_builder_add(&options, "{sv}", "handle_token", g_variant_new_string(token)); g_free(token);
    GVariant *reply = g_dbus_connection_call_sync(bus, "org.freedesktop.portal.Desktop", "/org/freedesktop/portal/desktop", "org.freedesktop.portal.Camera", "AccessCamera",
        g_variant_new("(a{sv})", &options), G_VARIANT_TYPE("(o)"), G_DBUS_CALL_FLAGS_NONE, 30000, p->cancel, &error);
    if (!reply) goto finish;
    const char *actual; g_variant_get(reply, "(&o)", &actual);
    if (strcmp(actual, path)) { g_set_error_literal(&error, G_IO_ERROR, G_IO_ERROR_FAILED, "Portal returned an unexpected request handle"); g_variant_unref(reply); goto finish; }
    g_variant_unref(reply);
    gint64 deadline = g_get_monotonic_time() + 240000000;
    while (!p->responded && !g_atomic_int_get(&p->cancelled) && g_get_monotonic_time() < deadline) {
        while (g_main_context_iteration(context, FALSE)) {}
        g_usleep(10000);
    }
    if (!p->responded || g_atomic_int_get(&p->cancelled)) {
        /* Close this exact request even if the caller abandoned its future. */
        GVariant *closed = g_dbus_connection_call_sync(bus, "org.freedesktop.portal.Desktop", path, "org.freedesktop.portal.Request", "Close", NULL, NULL, G_DBUS_CALL_FLAGS_NONE, 2000, NULL, NULL);
        if (closed) g_variant_unref(closed);
        g_set_error_literal(&error, G_IO_ERROR, G_IO_ERROR_CANCELLED, "Camera consent cancelled or timed out"); goto finish;
    }
    if (p->response != 0) {
        g_mutex_lock(&p->lock); p->result = p->response == 1 ? 4 : 2; g_strlcpy(p->error, p->response == 1 ? "Camera consent cancelled by the portal" : "Camera access denied by the portal", sizeof(p->error)); g_mutex_unlock(&p->lock); goto finish;
    }
    int enumeration_fd = open_remote(bus, p, &error);
    if (enumeration_fd < 0) goto finish;
    uint32_t node = authorized_node(enumeration_fd, p);
    if (node == PW_ID_ANY) { g_set_error_literal(&error, G_IO_ERROR, G_IO_ERROR_NOT_FOUND, "No portal-authorized camera node"); goto finish; }
    int fd = open_remote(bus, p, &error);
    if (fd < 0) goto finish;
    g_mutex_lock(&p->lock); p->fd = fd; p->node = node; p->result = 1; g_mutex_unlock(&p->lock);
finish:
    if (bus && path && g_atomic_int_get(&p->cancelled) && !p->responded) {
        GVariant *closed = g_dbus_connection_call_sync(bus, "org.freedesktop.portal.Desktop", path, "org.freedesktop.portal.Request", "Close", NULL, NULL, G_DBUS_CALL_FLAGS_NONE, 2000, NULL, NULL);
        if (closed) g_variant_unref(closed);
    }
    if (error) { g_mutex_lock(&p->lock); p->result = g_error_matches(error, G_IO_ERROR, G_IO_ERROR_CANCELLED) ? 4 : 3;
        char *remote = g_dbus_error_get_remote_error(error);
        if (remote && (g_str_has_suffix(remote, ".NotAllowed") || g_str_has_suffix(remote, ".AccessDenied"))) p->result = 2;
        g_free(remote); g_strlcpy(p->error, error->message, sizeof(p->error)); g_mutex_unlock(&p->lock); g_error_free(error); }
    if (bus && subscription) g_dbus_connection_signal_unsubscribe(bus, subscription);
    if (bus) g_object_unref(bus); g_free(path);
    g_main_context_pop_thread_default(context); g_main_context_unref(context); g_atomic_int_set(&portal_active, 0); portal_unref(p); return NULL;
}
void *ferro_linux_portal_begin(void) {
    if (!g_atomic_int_compare_and_exchange(&portal_active, 0, 1)) return NULL;
    Portal *p = g_new0(Portal, 1); p->refs = 2; p->fd = -1; p->cancel = g_cancellable_new(); g_mutex_init(&p->lock);
    GThread *thread = g_thread_new("ferroui-camera-consent", portal_worker, p); g_thread_unref(thread); return p;
}
int ferro_linux_portal_poll(void *ptr, int *fd, uint32_t *node, char *error, uint32_t capacity) {
    Portal *p = ptr; g_mutex_lock(&p->lock); int result = p->result;
    if (result == 1 && p->fd >= 0) { *fd = p->fd; p->fd = -1; *node = p->node; }
    g_strlcpy(error, p->error, capacity); g_mutex_unlock(&p->lock); return result;
}
void ferro_linux_portal_release(void *ptr) { Portal *p = ptr; g_atomic_int_set(&p->cancelled, 1); g_cancellable_cancel(p->cancel); portal_unref(p); }
int ferro_linux_portal_present(void) {
    GError *error = NULL; GDBusConnection *bus = g_bus_get_sync(G_BUS_TYPE_SESSION, NULL, &error);
    if (!bus) { g_clear_error(&error); return 0; }
    GVariant *reply = g_dbus_connection_call_sync(bus, "org.freedesktop.portal.Desktop", "/org/freedesktop/portal/desktop", "org.freedesktop.DBus.Properties", "Get",
        g_variant_new("(ss)", "org.freedesktop.portal.Camera", "IsCameraPresent"), G_VARIANT_TYPE("(v)"), G_DBUS_CALL_FLAGS_NONE, 3000, NULL, &error);
    gboolean present = FALSE;
    if (reply) { GVariant *value; g_variant_get(reply, "(v)", &value); if (g_variant_is_of_type(value, G_VARIANT_TYPE_BOOLEAN)) present = g_variant_get_boolean(value); g_variant_unref(value); g_variant_unref(reply); }
    g_clear_error(&error); g_object_unref(bus); return present;
}
