/* Position and click a real virtual pointer on this fixture's 1280x800
 * output. A persistent device also keeps wl_pointer bound across gestures. */
#include <linux/input-event-codes.h>
#include <time.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>
#include <wayland-client.h>
#include "virtual-pointer.h"

static struct zwlr_virtual_pointer_manager_v1 *manager;

static void global(void *data, struct wl_registry *registry, uint32_t name,
                   const char *interface, uint32_t version) {
    (void)data;
    (void)version;
    if (strcmp(interface, zwlr_virtual_pointer_manager_v1_interface.name) == 0)
        manager = wl_registry_bind(registry, name, &zwlr_virtual_pointer_manager_v1_interface, 1);
}

static void removed(void *data, struct wl_registry *registry, uint32_t name) {
    (void)data; (void)registry; (void)name;
}

int main(void) {
    const struct wl_registry_listener listener = { .global = global, .global_remove = removed };
    struct wl_display *display = wl_display_connect(NULL);
    if (!display) return 1;
    struct wl_registry *registry = wl_display_get_registry(display);
    wl_registry_add_listener(registry, &listener, NULL);
    if (wl_display_roundtrip(display) < 0 || !manager) return 1;
    struct zwlr_virtual_pointer_v1 *pointer =
        zwlr_virtual_pointer_manager_v1_create_virtual_pointer(manager, NULL);
    if (!pointer) return 1;
    if (wl_display_roundtrip(display) < 0) return 1;
    puts("ready");
    fflush(stdout);
    char line[80];
    while (fgets(line, sizeof(line), stdin)) {
        unsigned x, y;
        struct timespec now;
        clock_gettime(CLOCK_MONOTONIC, &now);
        uint32_t time = (uint32_t)((uint64_t)now.tv_sec * 1000 + now.tv_nsec / 1000000);
        if (sscanf(line, "set %u %u", &x, &y) == 2) {
            zwlr_virtual_pointer_v1_motion_absolute(pointer, time, x, y, 1280, 800);
        } else if (strcmp(line, "press button1\n") == 0) {
            zwlr_virtual_pointer_v1_button(pointer, time, BTN_LEFT, WL_POINTER_BUTTON_STATE_PRESSED);
        } else if (strcmp(line, "release button1\n") == 0) {
            zwlr_virtual_pointer_v1_button(pointer, time, BTN_LEFT, WL_POINTER_BUTTON_STATE_RELEASED);
        } else {
            return 2;
        }
        zwlr_virtual_pointer_v1_frame(pointer);
        if (wl_display_roundtrip(display) < 0) return 1;
    }
    return 0;
}
