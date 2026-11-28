/* A window with gauges, a control written in C (gauge.h, docs/c-abi.md, section 15): a click on
 * a gauge sets its level.
 *
 *   make -C samples/CApi run-gauge
 *
 * FERROUI_SMOKE_EXIT_MS=<n> closes the window after n milliseconds; FERROUI_SAMPLE_SCRIPT=1 lays
 * the window out, checks the gauges and prints what it found. */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "gauge.h"

static void check(FerroStatus status, const char *what) {
    if (status < 0) {
        fprintf(stderr, "%s failed (%d): %s\n", what, status, ferro_last_error());
        exit(1);
    }
}

static FerroHandle create(int64_t type_id) {
    FerroHandle object = ferro_create(type_id);
    if (object == 0) {
        fprintf(stderr, "ferro_create failed: %s\n", ferro_last_error());
        exit(1);
    }
    return object;
}

/* The classes of the program, registered by their entries before the application starts
 * (docs/c-abi.md, section 16). */
static int64_t window_type, panel_type;

static int64_t on_start(int64_t token) {
    (void)token;
    check((FerroStatus)gauge_register(), "gauge_register");
    FerroHandle window = create(window_type);
    check(ferro_set_str(window, ferro_property_id("Title", -1), "Gauges", -1), "Title");
    check(ferro_set_f64(window, ferro_property_id("Width", -1), 320), "Width");
    check(ferro_set_f64(window, ferro_property_id("Height", -1), 200), "Height");
    FerroHandle panel = create(panel_type);
    check(ferro_set_f64(panel, ferro_property_id("Spacing", -1), 12), "Spacing");
    check(ferro_set_str(panel, ferro_property_id("Margin", -1), "24", -1), "Margin");
    FerroHandle gauges[3];
    for (int n = 0; n < 3; n++) {
        gauges[n] = create(gauge_class);
        check(ferro_set_f64(gauges[n], gauge_level, 0.25 * (n + 1)), "Level");
        check(ferro_add_child(panel, gauges[n]), "ferro_add_child");
    }
    check(ferro_add_child(window, panel), "ferro_add_child");
    check(ferro_show(window), "ferro_show");
    if (getenv("FERROUI_SAMPLE_SCRIPT") != NULL) {
        check(ferro_layout_measure(window, 320, 200), "measure");
        double width = ferro_layout_desired_width(gauges[0]);
        double level = ferro_get_f64_or(gauges[1], gauge_level, -1);
        int ok = width == 160 && level == 0.5;
        printf("script: a gauge measures %.0f wide, the second shows %.2f: %s\n", width, level, ok ? "ok" : "FAILED");
        if (!ok) exit(1);
    }
    for (int n = 0; n < 3; n++) ferro_release(gauges[n]);
    ferro_release(panel);
    ferro_release(window);
    return 0;
}

int main(void) {
    window_type = ferro_type_window();
    panel_type = ferro_type_stack_panel();
    FerroStatus status = ferro_app_run(NULL, on_start, 0);
    if (status != FERRO_OK) {
        fprintf(stderr, "ferro_app_run failed (%d): %s\n", status, ferro_last_error());
        return 1;
    }
    return 0;
}
