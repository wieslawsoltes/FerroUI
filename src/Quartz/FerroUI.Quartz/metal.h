#pragma once
#include <CoreGraphics/CoreGraphics.h>
#include <stdint.h>

// Status codes of the GPU renderer. Nothing in this interface throws: a
// failure is a returned status or a NULL handle.
enum {
  QM_OK = 0,
  QM_NO_DEVICE = 1,
  QM_UNSUPPORTED_DEVICE = 2,
  QM_SHADER_FAILED = 3,
  QM_OUT_OF_MEMORY = 4,
  QM_INVALID_SIZE = 5,
  QM_DEVICE_MISMATCH = 6,
  QM_GPU_FAILED = 7,
  QM_INVALID_ARGUMENT = 8,
};

int qm_is_surface(void *p);
int qm_status(void);
size_t qm_max_texture_size(void);
void *qm_surface(size_t w, size_t h);
int qm_configure(void *device);
int qm_read_pixels(void *surface, uint8_t *out, size_t length);
int qm_write_pixels(void *surface, const uint8_t *data, size_t length);
void *qm_snapshot(void *surface);
void qm_save(void *surface);
void qm_restore(void *surface);
void qm_set_transform(void *surface, double a, double b, double c, double d,
                      double x, double y, double sx, double sy);
void qm_clear(void *surface, double r, double g, double b, double a);
void qm_clip(void *surface, CGPathRef path, int evenodd);
void qm_fill(void *surface, CGPathRef path, int evenodd, double r, double g,
             double b, double a);
void qm_gradient(void *surface, const double *colors, const double *stops,
                 size_t n, double x1, double y1, double x2, double y2,
                 double radius, int radial);
void qm_begin_layer(void *surface, double opacity, int blend);
void qm_end_layer(void *surface);
void qm_draw_image(void *surface, CGImageRef image, double sx, double sy,
                   double sw, double sh, double dx, double dy, double dw,
                   double dh, double alpha);
void qm_draw_surface(void *surface, void *source, double sx, double sy,
                     double sw, double sh, double dx, double dy, double dw,
                     double dh, double alpha);
void qm_copy_surface(void *surface, void *source);
void qm_glyphs(void *surface, void *font, double size, const uint16_t *glyphs,
               const double *positions, size_t n, double r, double g, double b,
               double a);
void qm_shadow(void *surface, double x, double y, double blur, double r,
               double g, double b, double a);
int qm_present(void *surface, void *texture, void *platform_queue,
               int flipped);
void qm_statistics(uint64_t *out, size_t count);
int qm_convert_frame(void *surface, void *texture, int rec709, int flipped,
                     int opaque, void *context, void (*completed)(void *, int));

void qm_blur(void *surface, double sigma, double ax, double ay, double bx,
             double by);
void *qm_layer_snapshot(void *surface);
void qm_filter(void *surface, int kind, const float *data, size_t length,
               void *secondary);

void qm_gradient_options(void *surface, int spread, double aspect,
                         double angle);

void qm_draw_tile(void *surface, void *source, double x, double y, double w,
                  double h, double alpha, int mode);

void qm_render_options(void *surface, int blend, int aliased,
                       int interpolation);

void *qm_take_layer(void *surface);

void qm_acrylic(void *surface, const double *colors, void *noise, int replace);

int qm_snapshot_external(void *surface, void *source, int flipped, void *wait,
                         uint64_t waitValue, void *signal,
                         uint64_t signalValue);
