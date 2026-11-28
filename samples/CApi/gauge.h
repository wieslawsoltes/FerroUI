/* A control written in C through the C ABI of FerroUI (docs/c-abi.md, section 15): Gauge, a bar
 * that shows a level from 0 to 1 and takes the level a click gives it. One registered class on the
 * base Control, one registered property (Level, a number that affects the render), and the
 * callbacks of the class: create (the data of an instance), measure, arrange, render and pointer.
 * Used by gauge.c (a window with gauges) and by abi_test.c (its tests). */

#ifndef FERROUI_SAMPLE_GAUGE_H
#define FERROUI_SAMPLE_GAUGE_H

#include <stdlib.h>

#include "ferroui.h"

/* The data of one gauge: its brushes and the size it was arranged at. */
typedef struct {
    int64_t track;
    int64_t fill;
    double width;
    double height;
} Gauge;

static int64_t gauge_class;
static int64_t gauge_level;

static inline void gauge_release(void *data) {
    free(data);
}

static inline FerroStatus gauge_create(void *class_data, FerroHandle self) {
    (void)class_data;
    Gauge *gauge = calloc(1, sizeof *gauge);
    if (gauge == NULL) return FERRO_ERROR;
    gauge->track = ferro_draw_brush(0xFFE0E0E0);
    gauge->fill = ferro_draw_brush(0xFF2E7D32);
    return ferro_object_set_data(self, gauge, gauge_release);
}

/* A gauge wants 160 by 16, or less where less is available. */
static inline FerroStatus gauge_measure(void *data, FerroHandle self, double available_width,
                                        double available_height, double *width, double *height) {
    (void)data;
    (void)self;
    *width = available_width < 160 ? available_width : 160;
    *height = available_height < 16 ? available_height : 16;
    return 0;
}

/* It takes the size it is given, and keeps it for the render and the pointer. */
static inline FerroStatus gauge_arrange(void *data, FerroHandle self, double final_width,
                                        double final_height, double *width, double *height) {
    (void)self;
    Gauge *gauge = data;
    gauge->width = final_width;
    gauge->height = final_height;
    *width = final_width;
    *height = final_height;
    return 0;
}

/* The track, and the filled part of the level over it. */
static inline FerroStatus gauge_render(void *data, FerroHandle self, int64_t context) {
    Gauge *gauge = data;
    double level = ferro_get_f64_or(self, gauge_level, 0);
    double radius = gauge->height / 2;
    FerroStatus status = ferro_draw_rect(context, 0, 0, gauge->width, gauge->height, radius, radius, gauge->track, 0);
    if (status != FERRO_OK) return status;
    return ferro_draw_rect(context, 0, 0, gauge->width * level, gauge->height, radius, radius, gauge->fill, 0);
}

/* A press sets the level where it is; a status of 1 marks the event as handled. */
static inline FerroStatus gauge_pointer(void *data, FerroHandle self, int32_t kind, double x, double y,
                                        int64_t modifiers) {
    (void)y;
    (void)modifiers;
    Gauge *gauge = data;
    if (kind != FERRO_POINTER_PRESSED || gauge->width <= 0) return 0;
    double level = x / gauge->width;
    level = level < 0 ? 0 : level > 1 ? 1 : level;
    FerroStatus status = ferro_set_f64(self, gauge_level, level);
    return status == FERRO_OK ? 1 : status;
}

/* Registers the class and its property once; the type ID of the class, or a negative status. */
static inline int64_t gauge_register(void) {
    if (gauge_class > 0) return gauge_class;
    int64_t class_id = ferro_class_register("Gauge", -1, FERRO_BASE_CONTROL);
    if (class_id < 0) return class_id;
    gauge_level = ferro_property_register(class_id, "Level", -1, FERRO_SIGNAL_F64, 0.5, NULL,
                                          FERRO_PROPERTY_AFFECTS_RENDER);
    if (gauge_level < 0) return gauge_level;
    ferro_class_on_create(class_id, gauge_create);
    ferro_class_on_measure(class_id, gauge_measure);
    ferro_class_on_arrange(class_id, gauge_arrange);
    ferro_class_on_render(class_id, gauge_render);
    ferro_class_on_pointer(class_id, gauge_pointer);
    gauge_class = class_id;
    return class_id;
}

#endif
