#ifndef FERROUI_MEDIA_LINUX_H
#define FERROUI_MEDIA_LINUX_H
#include <stdint.h>
typedef struct FerroLinuxSession FerroLinuxSession;
typedef struct FerroLinuxFrame FerroLinuxFrame;
typedef struct {
    int32_t state, seekable, buffering, error_kind;
    uint32_t width, height, fps_n, fps_d;
    uint64_t position_ns, duration_ns, seek_done;
    char error[512];
} FerroLinuxState;
typedef struct {
    uint32_t width, height, format, color, plane_count;
    uint32_t strides[4], offsets[4];
    int32_t fds[4];
    uint64_t modifier, bytes, timestamp_ns, sequence, epoch;
} FerroLinuxFrameInfo;
/* All calls are nonblocking on the UI thread. Pipeline work owns a native thread. */
FerroLinuxSession *ferro_linux_open(const char *source, const char *caps, int profile,
    int remote_fd, uint32_t node, uint32_t width, uint32_t height, uint32_t fps_n, uint32_t fps_d);
void ferro_linux_close(FerroLinuxSession *s);
void ferro_linux_state(FerroLinuxSession *s, FerroLinuxState *state);
void ferro_linux_intent(FerroLinuxSession *s, int intent);
uint64_t ferro_linux_seek(FerroLinuxSession *s, uint64_t ns);
void ferro_linux_audio(FerroLinuxSession *s, double volume, int muted);
FerroLinuxFrame *ferro_linux_take(FerroLinuxSession *s);
void ferro_linux_frame_info(FerroLinuxFrame *f, FerroLinuxFrameInfo *info);
void ferro_linux_frame_release(FerroLinuxFrame *f);
void *ferro_linux_alias(FerroLinuxFrame *f, intptr_t display, uint32_t *texture);
void ferro_linux_alias_release(void *alias, int context_alive);
int ferro_linux_convert(FerroLinuxFrame *f, intptr_t display, uint32_t output);
/* Explicit still-photo transfer only, never called from the preview route. */
int ferro_linux_photo_rgb(FerroLinuxFrame *f, uint8_t *rgb, uint64_t capacity);
#endif
