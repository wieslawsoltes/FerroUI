/* Hello, World! in C through the C ABI of FerroUI (docs/c-abi.md): a window with a text and a
 * button whose clicks change its text, the C version of samples/HelloWorld.Builders.
 *
 *   make -C samples/CApi run
 *
 * FERROUI_SMOKE_EXIT_MS=<n> closes the window after n milliseconds. */

#include <stdio.h>
#include <stdlib.h>

#include "ferroui.h"

static int64_t content_id;
static int64_t clicks;

/* Ends the program with the message of the last failure. */
static void check(FerroStatus status, const char *what) {
    if (status != FERRO_OK) {
        fprintf(stderr, "%s failed (%d): %s\n", what, status, ferro_last_error());
        exit(1);
    }
}

static FerroHandle create(const char *type) {
    FerroHandle object = ferro_create(ferro_type_id(type, -1));
    if (object == 0) {
        fprintf(stderr, "ferro_create(%s) failed: %s\n", type, ferro_last_error());
        exit(1);
    }
    return object;
}

static int64_t property(const char *name) {
    return ferro_property_id(name, -1);
}

/* The handler of Click: the token is unused, the sender is the button. */
static int64_t on_click(int64_t token, FerroHandle sender, int64_t arg) {
    (void)token;
    (void)arg;
    char text[64];
    clicks += 1;
    snprintf(text, sizeof text, clicks == 1 ? "Clicked %lld time" : "Clicked %lld times", (long long)clicks);
    check(ferro_set_str(sender, content_id, text, -1), "ferro_set_str");
    return 0;
}

static int64_t on_start(int64_t token) {
    (void)token;
    content_id = property("Content");

    FerroHandle window = create("Window");
    check(ferro_set_str(window, property("Title"), "Hello, World!", -1), "Title");
    check(ferro_set_f64(window, property("Width"), 400), "Width");
    check(ferro_set_f64(window, property("Height"), 240), "Height");

    FerroHandle panel = create("StackPanel");
    check(ferro_set_str(panel, property("HorizontalAlignment"), "Center", -1), "HorizontalAlignment");
    check(ferro_set_str(panel, property("VerticalAlignment"), "Center", -1), "VerticalAlignment");
    check(ferro_set_f64(panel, property("Spacing"), 12), "Spacing");

    FerroHandle text = create("TextBlock");
    check(ferro_set_str(text, property("Text"), "Hello, World!", -1), "Text");
    check(ferro_set_f64(text, property("FontSize"), 28), "FontSize");

    FerroHandle button = create("Button");
    check(ferro_set_str(button, property("HorizontalAlignment"), "Center", -1), "HorizontalAlignment");
    check(ferro_set_str(button, content_id, "Click me", -1), "Content");
    if (ferro_subscribe(button, ferro_event_id("Click", -1), on_click, 0) < 0) {
        check(ferro_last_error_code(), "ferro_subscribe");
    }

    check(ferro_add_child(panel, text), "ferro_add_child");
    check(ferro_add_child(panel, button), "ferro_add_child");
    check(ferro_add_child(window, panel), "ferro_add_child");
    check(ferro_show(window), "ferro_show");

    /* The window holds its tree, and the platform holds the open window. */
    ferro_release(text);
    ferro_release(button);
    ferro_release(panel);
    ferro_release(window);
    return 0;
}

int main(void) {
    if (ferro_abi_version() != FERRO_ABI_VERSION) {
        fprintf(stderr, "the library implements ABI %d, this program needs %d\n", ferro_abi_version(), FERRO_ABI_VERSION);
        return 1;
    }
    FerroStatus status = ferro_app_run(NULL, on_start, 0);
    if (status != FERRO_OK) {
        fprintf(stderr, "ferro_app_run failed (%d): %s\n", status, ferro_last_error());
        return 1;
    }
    printf("hello_world: the window closed after %lld clicks\n", (long long)clicks);
    return 0;
}
