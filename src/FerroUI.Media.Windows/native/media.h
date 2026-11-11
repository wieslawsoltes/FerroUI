#pragma once
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
typedef struct FerroWindowsSession FerroWindowsSession;
typedef struct FerroWindowsFrame FerroWindowsFrame;
typedef struct {
    int32_t state, seekable, buffering, error_kind;
    uint32_t width,height,fps_n,fps_d;
    uint64_t position_ns,duration_ns,seek_done;
    char error[512];
} FerroWindowsState;
typedef struct {uint32_t width,height,color,rotation;uint64_t bytes,timestamp_ns,sequence,epoch;uint32_t mirrored,x,y,visible_width,visible_height;} FerroWindowsFrameInfo;
/* The returned device is AddRef-owned, queried from the current ANGLE display. */
typedef struct { intptr_t functions[13]; } FerroWindowsEgl;
void *ferro_windows_device(intptr_t display,const FerroWindowsEgl *egl);
void ferro_windows_device_release(void *device);
FerroWindowsSession *ferro_windows_open(void *device,const uint16_t *source,int camera,int resource_file,uint32_t width,uint32_t height,uint32_t fps_n,uint32_t fps_d);
void ferro_windows_close(FerroWindowsSession *s);
void ferro_windows_state(FerroWindowsSession *s,FerroWindowsState *out);
void ferro_windows_intent(FerroWindowsSession *s,int intent);
uint64_t ferro_windows_seek(FerroWindowsSession *s,uint64_t ns);
void ferro_windows_audio(FerroWindowsSession *s,double volume,int muted);
FerroWindowsFrame *ferro_windows_take(FerroWindowsSession *s);
void ferro_windows_frame_info(FerroWindowsFrame *f,FerroWindowsFrameInfo *out);
void ferro_windows_frame_release(FerroWindowsFrame *f);
void *ferro_windows_frame_texture(FerroWindowsFrame *f,uint32_t *slice);
void *ferro_windows_alias(FerroWindowsFrame *f,intptr_t display,const FerroWindowsEgl *egl,uint32_t *texture);
void ferro_windows_alias_release(void *alias,int alive);
int ferro_windows_photo_rgb(FerroWindowsFrame *f,uint8_t *rgb,uint64_t length);
/* Enumerates activation metadata only; does not activate/open capture devices. */
char *ferro_windows_devices(void);
void ferro_windows_string_release(char *text);
#ifdef __cplusplus
}
#endif
