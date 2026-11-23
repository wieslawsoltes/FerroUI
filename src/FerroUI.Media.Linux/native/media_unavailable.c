/* The native Linux media bridge of a build without its system libraries (GStreamer 1.24 with its
 * app, video and allocators libraries, EGL, GLES 2): every session fails to open, so the media of
 * the application reports a native failure instead of the build failing. build.rs compiles it and
 * prints a warning when pkg-config does not find one of the libraries. */
#include "media.h"
#include <stddef.h>
#include <string.h>

FerroLinuxSession *ferro_linux_open(const char *source, const char *caps, int profile,
    int remote_fd, uint32_t node, uint32_t width, uint32_t height, uint32_t fps_n, uint32_t fps_d) {
    (void)source; (void)caps; (void)profile; (void)remote_fd; (void)node;
    (void)width; (void)height; (void)fps_n; (void)fps_d;
    return NULL;
}
void ferro_linux_close(FerroLinuxSession *s) { (void)s; }
void ferro_linux_state(FerroLinuxSession *s, FerroLinuxState *state) {
    (void)s;
    if (state != NULL) {
        memset(state, 0, sizeof *state);
        state->state = 5;
        strncpy(state->error, "Native Linux media was built without its system libraries", sizeof state->error - 1);
    }
}
void ferro_linux_intent(FerroLinuxSession *s, int intent) { (void)s; (void)intent; }
uint64_t ferro_linux_seek(FerroLinuxSession *s, uint64_t ns) { (void)s; (void)ns; return 0; }
void ferro_linux_audio(FerroLinuxSession *s, double volume, int muted) { (void)s; (void)volume; (void)muted; }
FerroLinuxFrame *ferro_linux_take(FerroLinuxSession *s) { (void)s; return NULL; }
void ferro_linux_frame_info(FerroLinuxFrame *f, FerroLinuxFrameInfo *info) {
    (void)f;
    if (info != NULL) memset(info, 0, sizeof *info);
}
void ferro_linux_frame_release(FerroLinuxFrame *f) { (void)f; }
void *ferro_linux_alias(FerroLinuxFrame *f, intptr_t display, uint32_t *texture) {
    (void)f; (void)display; (void)texture;
    return NULL;
}
void ferro_linux_alias_release(void *alias, int context_alive) { (void)alias; (void)context_alive; }
int ferro_linux_convert(FerroLinuxFrame *f, intptr_t display, uint32_t output) {
    (void)f; (void)display; (void)output;
    return 0;
}
int ferro_linux_photo_rgb(FerroLinuxFrame *f, uint8_t *rgb, uint64_t capacity) {
    (void)f; (void)rgb; (void)capacity;
    return 0;
}
int ferro_linux_can_record(FerroLinuxSession *s) { (void)s; return 0; }
FerroLinuxRecording *ferro_linux_record(FerroLinuxSession *s) { (void)s; return NULL; }
void ferro_linux_record_finish(FerroLinuxRecording *r) { (void)r; }
void ferro_linux_record_cancel(FerroLinuxRecording *r) { (void)r; }
void ferro_linux_record_release(FerroLinuxRecording *r) { (void)r; }
int ferro_linux_record_poll(FerroLinuxRecording *r, FerroLinuxRecordingInfo *out) {
    (void)r;
    memset(out, 0, sizeof *out);
    return -2;
}
int ferro_linux_record_copy(FerroLinuxRecording *r, uint8_t *bytes, uint64_t length) {
    (void)r; (void)bytes; (void)length;
    return 0;
}
