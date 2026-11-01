/* System GStreamer only. No CPU video map, conversion, or upload is admitted. */
#define _GNU_SOURCE
#include "media.h"
#include <gst/app/gstappsink.h>
#include <gst/allocators/gstdmabuf.h>
#include <gst/video/video.h>
#include <gst/video/video-info-dma.h>
#include <EGL/egl.h>
#include <EGL/eglext.h>
#include <GLES3/gl3.h>
#include <GLES2/gl2ext.h>
#include <unistd.h>
#include <sys/stat.h>
#include <string.h>
#include <math.h>

#define FRAME_LIMIT (128ull * 1024 * 1024)
#define DRM_NV12 0x3231564e
#define DRM_XRGB 0x34325258
#define DRM_XBGR 0x34324258
static gint native_active = 0;
struct FerroLinuxFrame { GstSample *sample; FerroLinuxFrameInfo info; };
struct FerroLinuxSession {
    gint refs, closed;
    GMutex lock;
    char *source, *caps;
    int profile, remote_fd, intent, applied_intent;
    uint32_t node, requested_w, requested_h, fps_n, fps_d;
    uint64_t epoch, segment_epoch, expected_segment_epoch, pending_seek, seek_ns, sequence;
    guint32 segment_seq;
    double volume;
    int muted;
    FerroLinuxState state;
    GstElement *pipeline, *sink;
    FerroLinuxFrame *latest;
};
static void unref_session(FerroLinuxSession *s) {
    if (!g_atomic_int_dec_and_test(&s->refs)) return;
    if (s->latest) ferro_linux_frame_release(s->latest);
    if (s->remote_fd >= 0) close(s->remote_fd);
    g_free(s->source); g_free(s->caps); g_mutex_clear(&s->lock); g_free(s);
}
static void fail(FerroLinuxSession *s, int kind, const char *message) {
    g_mutex_lock(&s->lock);
    s->state.state = 5; s->state.error_kind = kind;
    g_strlcpy(s->state.error, message, sizeof(s->state.error));
    g_mutex_unlock(&s->lock);
}
void ferro_linux_frame_release(FerroLinuxFrame *f) {
    if (!f) return;
    gst_sample_unref(f->sample); g_free(f);
}
void ferro_linux_frame_info(FerroLinuxFrame *f, FerroLinuxFrameInfo *out) { *out = f->info; }
static FerroLinuxFrame *admit_sample(GstSample *sample) {
    GstCaps *caps = gst_sample_get_caps(sample);
    GstBuffer *buffer = gst_sample_get_buffer(sample);
    GstVideoInfoDmaDrm drm;
    gst_video_info_dma_drm_init(&drm);
    if (!caps || !buffer || !gst_video_info_dma_drm_from_caps(&drm, caps)) return NULL;
    if (GST_VIDEO_INFO_IS_INTERLACED(&drm.vinfo) || drm.vinfo.par_n != drm.vinfo.par_d) return NULL;
    GstVideoMeta *meta = gst_buffer_get_video_meta(buffer);
    if (!meta || meta->width != GST_VIDEO_INFO_WIDTH(&drm.vinfo) || meta->height != GST_VIDEO_INFO_HEIGHT(&drm.vinfo)
        || !meta->width || !meta->height || meta->width > 8192 || meta->height > 8192
        || meta->n_planes < 1 || meta->n_planes > 4) return NULL;
    if (drm.drm_fourcc != DRM_NV12 && drm.drm_fourcc != DRM_XRGB && drm.drm_fourcc != DRM_XBGR) return NULL;
    if (drm.drm_modifier == G_GUINT64_CONSTANT(0x00ffffffffffffff)) return NULL; /* implicit layout is not negotiation */
    GstVideoCropMeta *crop = gst_buffer_get_video_crop_meta(buffer);
    if (crop && (crop->x || crop->y || crop->width != meta->width || crop->height != meta->height)) return NULL;
    GstVideoColorimetry color = drm.vinfo.colorimetry;
    uint32_t color_kind;
    if (color.primaries != GST_VIDEO_COLOR_PRIMARIES_BT709) return NULL;
    if (drm.drm_fourcc == DRM_NV12) {
        if (meta->n_planes != 2 || color.matrix != GST_VIDEO_COLOR_MATRIX_BT709
            || color.transfer != GST_VIDEO_TRANSFER_BT709) return NULL;
        if (color.range != GST_VIDEO_COLOR_RANGE_0_255 && color.range != GST_VIDEO_COLOR_RANGE_16_235) return NULL;
        color_kind = color.range == GST_VIDEO_COLOR_RANGE_0_255 ? 2 : 3;
    } else {
        if (meta->n_planes != 1 || color.range != GST_VIDEO_COLOR_RANGE_0_255) return NULL;
        if (color.transfer == GST_VIDEO_TRANSFER_SRGB) color_kind = 1;
        else if (color.transfer == GST_VIDEO_TRANSFER_BT709) color_kind = 2;
        else return NULL;
    }
    FerroLinuxFrame *f = g_new0(FerroLinuxFrame, 1);
    f->info.width = meta->width; f->info.height = meta->height;
    f->info.format = drm.drm_fourcc; f->info.modifier = drm.drm_modifier;
    f->info.plane_count = meta->n_planes; f->info.color = color_kind;
    struct stat identities[4]; guint unique = 0;
    for (guint p = 0; p < meta->n_planes; p++) {
        guint index, count; gsize skip;
        if (meta->stride[p] <= 0) goto bad;
        /* Linear planes must fit in both the visible GstMemory range and the
         * FD allocation. Checking just the first byte admits truncated rows.
         * Nonlinear layouts remain the exact negotiated modifier's contract. */
        uint64_t extent = 1;
        if (drm.drm_modifier == 0) {
            uint64_t rows = meta->height, row_bytes = (uint64_t)meta->width * 4;
            if (drm.drm_fourcc == DRM_NV12) {
                rows = p == 0 ? meta->height : ((uint64_t)meta->height + 1) / 2;
                row_bytes = p == 0 ? meta->width : ((uint64_t)meta->width + 1) / 2 * 2;
            }
            if ((uint64_t)meta->stride[p] < row_bytes) goto bad;
            extent = (rows - 1) * (uint64_t)meta->stride[p] + row_bytes;
            if (extent > FRAME_LIMIT) goto bad;
        }
        if (!gst_buffer_find_memory(buffer, meta->offset[p], (gsize)extent, &index, &count, &skip) || count != 1) goto bad;
        GstMemory *mem = gst_buffer_peek_memory(buffer, index);
        if (!mem || !gst_is_dmabuf_memory(mem)) goto bad;
        gsize base, maxsize; gsize visible = gst_memory_get_sizes(mem, &base, &maxsize);
        int fd = gst_dmabuf_memory_get_fd(mem);
        if (fd < 0 || maxsize > FRAME_LIMIT || base > maxsize
            || visible > maxsize - base || skip > visible || base + skip > INT32_MAX) goto bad;
        struct stat st;
        if (fstat(fd, &st)) goto bad;
        uint64_t bytes = st.st_size > 0 ? (uint64_t)st.st_size : maxsize;
        if (!bytes || bytes > FRAME_LIMIT || base + skip >= bytes
            || skip > visible || extent > visible - skip || extent > bytes - (base + skip)) goto bad;
        int found = 0;
        for (guint i = 0; i < unique; i++) if (identities[i].st_dev == st.st_dev && identities[i].st_ino == st.st_ino) found = 1;
        if (!found) { identities[unique++] = st; f->info.bytes += bytes; }
        if (f->info.bytes > FRAME_LIMIT) goto bad;
        f->info.fds[p] = fd; f->info.strides[p] = meta->stride[p]; f->info.offsets[p] = base + skip;
    }
    if (!GST_BUFFER_PTS_IS_VALID(buffer)) goto bad;
    f->info.timestamp_ns = GST_BUFFER_PTS(buffer);
    f->sample = gst_sample_ref(sample);
    return f;
bad:
    g_free(f); return NULL;
}
static GstFlowReturn receive(FerroLinuxSession *s, GstSample *sample) {
    if (!sample) return GST_FLOW_EOS;
    FerroLinuxFrame *f = admit_sample(sample);
    if (!f) { gst_sample_unref(sample); fail(s, 1, "Unadmitted DMA-BUF metadata, color, format, modifier or crop"); return GST_FLOW_NOT_NEGOTIATED; }
    GstCaps *caps = gst_sample_get_caps(sample);
    const GstStructure *structure = gst_caps_get_structure(caps, 0);
    int n = 0, d = 1; gst_structure_get_fraction(structure, "framerate", &n, &d);
    gst_sample_unref(sample);
    g_mutex_lock(&s->lock);
    if (s->segment_epoch != s->epoch || g_atomic_int_get(&s->closed)) { g_mutex_unlock(&s->lock); ferro_linux_frame_release(f); return GST_FLOW_OK; }
    f->info.epoch = s->epoch; f->info.sequence = ++s->sequence;
    FerroLinuxFrame *old = s->latest; s->latest = f;
    s->state.width = f->info.width; s->state.height = f->info.height;
    s->state.fps_n = n; s->state.fps_d = d;
    if (s->epoch > 1) s->state.seek_done = s->epoch;
    g_mutex_unlock(&s->lock);
    ferro_linux_frame_release(old);
    return GST_FLOW_OK;
}
static GstFlowReturn new_sample(GstAppSink *sink, gpointer data) { return receive(data, gst_app_sink_pull_sample(sink)); }
static GstFlowReturn new_preroll(GstAppSink *sink, gpointer data) { return receive(data, gst_app_sink_pull_preroll(sink)); }
static GstPadProbeReturn allocation(GstPad *pad, GstPadProbeInfo *info, gpointer data) {
    (void)pad; (void)data;
    GstQuery *query = GST_PAD_PROBE_INFO_QUERY(info);
    if (GST_QUERY_TYPE(query) == GST_QUERY_ALLOCATION && gst_query_is_writable(query))
        gst_query_add_allocation_meta(query, GST_VIDEO_META_API_TYPE, NULL);
    return GST_PAD_PROBE_OK;
}
static GstPadProbeReturn segment(GstPad *pad, GstPadProbeInfo *info, gpointer data) {
    (void)pad;
    GstEvent *event = GST_PAD_PROBE_INFO_EVENT(info);
    if (GST_EVENT_TYPE(event) == GST_EVENT_SEGMENT) {
        FerroLinuxSession *s = data;
        g_mutex_lock(&s->lock);
        if (!s->segment_seq || gst_event_get_seqnum(event) == s->segment_seq) s->segment_epoch = s->expected_segment_epoch;
        g_mutex_unlock(&s->lock);
    } else if (GST_EVENT_TYPE(event) == GST_EVENT_TAG) {
        GstTagList *tags; char *orientation = NULL; gst_event_parse_tag(event, &tags);
        if (gst_tag_list_get_string(tags, GST_TAG_IMAGE_ORIENTATION, &orientation)) {
            if (strcmp(orientation, "rotate-0")) fail(data, 1, "Rotated source metadata is outside this native profile");
            g_free(orientation);
        }
    }
    return GST_PAD_PROBE_OK;
}
static gpointer worker(gpointer data) {
    FerroLinuxSession *s = data;
    GError *error = NULL;
    if (!gst_init_check(NULL, NULL, &error)) { fail(s, 0, error ? error->message : "GStreamer initialization failed"); g_clear_error(&error); goto done; }
    s->sink = gst_element_factory_make("appsink", NULL);
    GstCaps *caps = gst_caps_from_string(s->caps);
    if (!s->sink || !caps) { if (caps) gst_caps_unref(caps); fail(s, 1, "Missing appsink or admitted DMA-BUF caps"); goto done; }
    gst_object_ref_sink(s->sink);
    gst_app_sink_set_caps(GST_APP_SINK(s->sink), caps);
    gst_caps_unref(caps);
    g_object_set(s->sink, "max-buffers", 1u, "drop", TRUE, "sync", TRUE, "enable-last-sample", FALSE, "wait-on-eos", FALSE, NULL);
    GstAppSinkCallbacks callbacks = { 0 }; callbacks.new_sample = new_sample; callbacks.new_preroll = new_preroll;
    gst_app_sink_set_callbacks(GST_APP_SINK(s->sink), &callbacks, s, NULL);
    GstPad *pad = gst_element_get_static_pad(s->sink, "sink");
    gst_pad_add_probe(pad, GST_PAD_PROBE_TYPE_EVENT_DOWNSTREAM, segment, s, NULL);
    gst_pad_add_probe(pad, GST_PAD_PROBE_TYPE_QUERY_DOWNSTREAM, allocation, s, NULL); gst_object_unref(pad);
    if (s->profile == 0) {
        s->pipeline = gst_element_factory_make("playbin3", NULL);
        if (!s->pipeline) { fail(s, 1, "System GStreamer playbin3 is unavailable"); goto done; }
        g_object_set(s->pipeline, "uri", s->source, "video-sink", s->sink, "flags", 3u, NULL); /* native video + audio; no subtitles */
    } else {
        s->pipeline = gst_pipeline_new(NULL);
        GstElement *source = gst_element_factory_make(s->profile == 1 ? "pipewiresrc" : "v4l2src", NULL);
        if (!source) { fail(s, 1, "System camera source plugin unavailable"); goto done; }
        if (s->profile == 1) {
            char *node = g_strdup_printf("%u", s->node);
            g_object_set(source, "fd", s->remote_fd, "path", node, "do-timestamp", TRUE, "always-copy", FALSE, "min-buffers", 2, "max-buffers", 6, NULL); g_free(node);
        } else {
            g_object_set(source, "device", s->source, "io-mode", 4, "do-timestamp", TRUE, NULL); /* explicit DMABUF */
        }
        gst_object_ref(s->sink);
        gst_bin_add_many(GST_BIN(s->pipeline), source, s->sink, NULL);
        if (!gst_element_link(source, s->sink)) { fail(s, 1, "No direct DMA-BUF camera route"); goto done; }
    }
    GstBus *bus = gst_element_get_bus(s->pipeline);
    if (gst_element_set_state(s->pipeline, s->profile ? GST_STATE_PLAYING : GST_STATE_PAUSED) == GST_STATE_CHANGE_FAILURE) fail(s, 0, "Native pipeline preparation failed");
    while (!g_atomic_int_get(&s->closed)) {
        g_mutex_lock(&s->lock);
        int intent = s->intent; uint64_t seek = s->pending_seek, ns = s->seek_ns;
        double volume = s->volume; int muted = s->muted;
        s->pending_seek = 0;
        int failed = s->state.state == 5;
        g_mutex_unlock(&s->lock);
        if (failed) break;
        if (s->profile == 0) {
            g_object_set(s->pipeline, "volume", volume, "mute", (gboolean)muted, NULL);
            if (intent != s->applied_intent) {
                GstState target = intent == 1 ? GST_STATE_PLAYING : GST_STATE_PAUSED;
                if (gst_element_set_state(s->pipeline, target) == GST_STATE_CHANGE_FAILURE) fail(s, 0, "Native playback state change failed");
                s->applied_intent = intent;
            }
            if (seek) {
                GstEvent *event = gst_event_new_seek(1., GST_FORMAT_TIME, GST_SEEK_FLAG_FLUSH | GST_SEEK_FLAG_ACCURATE,
                    GST_SEEK_TYPE_SET, ns, GST_SEEK_TYPE_NONE, GST_CLOCK_TIME_NONE);
                guint32 seq = gst_util_seqnum_next(); gst_event_set_seqnum(event, seq);
                g_mutex_lock(&s->lock); s->segment_seq = seq; s->expected_segment_epoch = seek; g_mutex_unlock(&s->lock);
                if (!gst_element_send_event(s->pipeline, event)) fail(s, 1, "Native source rejected seeking");
            }
        }
        GstMessage *message;
        while ((message = gst_bus_pop(bus))) {
            switch (GST_MESSAGE_TYPE(message)) {
                case GST_MESSAGE_ERROR: {
                    GError *e = NULL; char *debug = NULL; gst_message_parse_error(message, &e, &debug);
                    fail(s, 0, e ? e->message : "Native pipeline failed"); g_clear_error(&e); g_free(debug); break;
                }
                case GST_MESSAGE_EOS: g_mutex_lock(&s->lock); s->state.state = 4; g_mutex_unlock(&s->lock); break;
                case GST_MESSAGE_BUFFERING: {
                    int percent; gst_message_parse_buffering(message, &percent);
                    g_mutex_lock(&s->lock); s->state.buffering = percent < 100; g_mutex_unlock(&s->lock); break;
                }
                default: break;
            }
            gst_message_unref(message);
        }
        GstState current, pending;
        GstStateChangeReturn changed = gst_element_get_state(s->pipeline, &current, &pending, 0);
        gint64 position = -1, duration = -1;
        gst_element_query_position(s->pipeline, GST_FORMAT_TIME, &position);
        gst_element_query_duration(s->pipeline, GST_FORMAT_TIME, &duration);
        GstQuery *query = gst_query_new_seeking(GST_FORMAT_TIME); gboolean seekable = FALSE;
        if (gst_element_query(s->pipeline, query)) gst_query_parse_seeking(query, NULL, &seekable, NULL, NULL);
        gst_query_unref(query);
        g_mutex_lock(&s->lock);
        if (s->state.state != 4 && s->state.state != 5) {
            if (changed == GST_STATE_CHANGE_FAILURE) { s->state.state = 5; g_strlcpy(s->state.error, "Native pipeline state failed", sizeof(s->state.error)); }
            else if (current == GST_STATE_PLAYING) s->state.state = 2;
            else if (current == GST_STATE_PAUSED && changed != GST_STATE_CHANGE_ASYNC) s->state.state = 1;
            else s->state.state = 0;
        }
        if (position >= 0) s->state.position_ns = position;
        s->state.duration_ns = duration >= 0 ? (uint64_t)duration : UINT64_MAX;
        s->state.seekable = seekable;
        g_mutex_unlock(&s->lock);
        g_usleep(10000);
    }
    gst_object_unref(bus);
done:
    g_atomic_int_set(&s->closed, 1);
    if (s->pipeline) { gst_element_set_state(s->pipeline, GST_STATE_NULL); gst_object_unref(s->pipeline); }
    if (s->sink) gst_object_unref(s->sink);
    s->pipeline = s->sink = NULL;
    g_mutex_lock(&s->lock); FerroLinuxFrame *last = s->latest; s->latest = NULL; g_mutex_unlock(&s->lock);
    ferro_linux_frame_release(last);
    g_atomic_int_add(&native_active, -1);
    unref_session(s); return NULL;
}
FerroLinuxSession *ferro_linux_open(const char *source, const char *caps, int profile, int fd,
    uint32_t node, uint32_t w, uint32_t h, uint32_t n, uint32_t d) {
    gint active;
    do { active = g_atomic_int_get(&native_active); if (active >= 4) return NULL; } while (!g_atomic_int_compare_and_exchange(&native_active, active, active + 1));
    FerroLinuxSession *s = g_new0(FerroLinuxSession, 1);
    s->refs = 2; g_mutex_init(&s->lock);
    s->source = g_strdup(source); s->caps = g_strdup(caps); s->profile = profile;
    s->remote_fd = fd >= 0 ? dup(fd) : -1; s->node = node;
    s->requested_w = w; s->requested_h = h; s->fps_n = n; s->fps_d = d;
    s->volume = 1.; s->epoch = s->segment_epoch = s->expected_segment_epoch = 1; s->applied_intent = -1;
    s->state.duration_ns = UINT64_MAX;
    GThread *thread = g_thread_new("ferroui-media", worker, s); g_thread_unref(thread);
    return s;
}
void ferro_linux_close(FerroLinuxSession *s) { g_atomic_int_set(&s->closed, 1); unref_session(s); }
void ferro_linux_state(FerroLinuxSession *s, FerroLinuxState *state) { g_mutex_lock(&s->lock); *state = s->state; g_mutex_unlock(&s->lock); }
void ferro_linux_intent(FerroLinuxSession *s, int intent) {
    g_mutex_lock(&s->lock); s->intent = intent;
    if (s->state.state == 4 && intent == 1) s->state.state = 0;
    g_mutex_unlock(&s->lock);
}
uint64_t ferro_linux_seek(FerroLinuxSession *s, uint64_t ns) {
    g_mutex_lock(&s->lock);
    uint64_t epoch = ++s->epoch; s->pending_seek = epoch; s->seek_ns = ns;
    s->state.state = 0;
    FerroLinuxFrame *old = s->latest; s->latest = NULL;
    g_mutex_unlock(&s->lock); ferro_linux_frame_release(old); return epoch;
}
void ferro_linux_audio(FerroLinuxSession *s, double volume, int muted) { g_mutex_lock(&s->lock); s->volume = volume; s->muted = muted; g_mutex_unlock(&s->lock); }
FerroLinuxFrame *ferro_linux_take(FerroLinuxSession *s) {
    g_mutex_lock(&s->lock); FerroLinuxFrame *f = s->latest; s->latest = NULL;
    if (f && f->info.epoch != s->epoch) { ferro_linux_frame_release(f); f = NULL; }
    g_mutex_unlock(&s->lock); return f;
}

/* EGL alias ownership stays with the frame lease. No native storage is adopted
 * by Skia. DMA-BUF implicit producer fences are honored by the EGL import. */
typedef struct { EGLDisplay display; EGLImageKHR image; GLuint texture; } Alias;
static Alias *make_alias(FerroLinuxFrame *f, EGLDisplay display, GLenum target) {
    PFNEGLCREATEIMAGEKHRPROC create = (PFNEGLCREATEIMAGEKHRPROC)eglGetProcAddress("eglCreateImageKHR");
    PFNGLEGLIMAGETARGETTEXTURE2DOESPROC bind = (PFNGLEGLIMAGETARGETTEXTURE2DOESPROC)eglGetProcAddress("glEGLImageTargetTexture2DOES");
    if (!create || !bind) return NULL;
    EGLint attrs[64]; int i = 0;
#define ADD(k,v) do { attrs[i++] = (k); attrs[i++] = (EGLint)(v); } while(0)
    ADD(EGL_WIDTH, f->info.width); ADD(EGL_HEIGHT, f->info.height);
    ADD(EGL_LINUX_DRM_FOURCC_EXT, f->info.format);
    static const EGLint fds[] = { EGL_DMA_BUF_PLANE0_FD_EXT, EGL_DMA_BUF_PLANE1_FD_EXT, EGL_DMA_BUF_PLANE2_FD_EXT, EGL_DMA_BUF_PLANE3_FD_EXT };
    static const EGLint offsets[] = { EGL_DMA_BUF_PLANE0_OFFSET_EXT, EGL_DMA_BUF_PLANE1_OFFSET_EXT, EGL_DMA_BUF_PLANE2_OFFSET_EXT, EGL_DMA_BUF_PLANE3_OFFSET_EXT };
    static const EGLint pitches[] = { EGL_DMA_BUF_PLANE0_PITCH_EXT, EGL_DMA_BUF_PLANE1_PITCH_EXT, EGL_DMA_BUF_PLANE2_PITCH_EXT, EGL_DMA_BUF_PLANE3_PITCH_EXT };
    static const EGLint lows[] = { EGL_DMA_BUF_PLANE0_MODIFIER_LO_EXT, EGL_DMA_BUF_PLANE1_MODIFIER_LO_EXT, EGL_DMA_BUF_PLANE2_MODIFIER_LO_EXT, EGL_DMA_BUF_PLANE3_MODIFIER_LO_EXT };
    for (uint32_t p = 0; p < f->info.plane_count; p++) {
        ADD(fds[p], f->info.fds[p]); ADD(offsets[p], f->info.offsets[p]); ADD(pitches[p], f->info.strides[p]);
        ADD(lows[p], f->info.modifier & UINT32_MAX); ADD(lows[p] + 1, f->info.modifier >> 32);
    }
    if (f->info.format == DRM_NV12) {
        ADD(EGL_YUV_COLOR_SPACE_HINT_EXT, EGL_ITU_REC709_EXT);
        ADD(EGL_SAMPLE_RANGE_HINT_EXT, f->info.color == 3 ? EGL_YUV_NARROW_RANGE_EXT : EGL_YUV_FULL_RANGE_EXT);
    }
    attrs[i++] = EGL_NONE;
#undef ADD
    Alias *a = g_new0(Alias, 1); a->display = display;
    a->image = create(display, EGL_NO_CONTEXT, EGL_LINUX_DMA_BUF_EXT, NULL, attrs);
    if (a->image == EGL_NO_IMAGE_KHR) { g_free(a); return NULL; }
    GLint active, old;
    glGetIntegerv(GL_ACTIVE_TEXTURE, &active); glActiveTexture(GL_TEXTURE0);
    glGetIntegerv(target == GL_TEXTURE_2D ? GL_TEXTURE_BINDING_2D : GL_TEXTURE_BINDING_EXTERNAL_OES, &old);
    glGenTextures(1, &a->texture); glBindTexture(target, a->texture);
    glTexParameteri(target, GL_TEXTURE_MIN_FILTER, GL_LINEAR); glTexParameteri(target, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
    glTexParameteri(target, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE); glTexParameteri(target, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);
    bind(target, a->image);
    GLenum error = glGetError();
    glBindTexture(target, old); glActiveTexture(active);
    if (!a->texture || error != GL_NO_ERROR) { ferro_linux_alias_release(a, 1); return NULL; }
    return a;
}
void *ferro_linux_alias(FerroLinuxFrame *f, intptr_t display, uint32_t *texture) {
    if (f->info.format == DRM_NV12) return NULL;
    Alias *a = make_alias(f, (EGLDisplay)display, GL_TEXTURE_2D);
    if (a) *texture = a->texture;
    return a;
}
void ferro_linux_alias_release(void *ptr, int alive) {
    Alias *a = ptr;
    if (alive) {
        PFNEGLDESTROYIMAGEKHRPROC destroy = (PFNEGLDESTROYIMAGEKHRPROC)eglGetProcAddress("eglDestroyImageKHR");
        glDeleteTextures(1, &a->texture);
        if (destroy) destroy(a->display, a->image);
    }
    /* Context-loss retirement quarantines accounting. Native EGL resources are
       then reclaimed by display teardown, never by a foreign/current context. */
    g_free(a);
}
static GLuint shader(GLenum kind, const char *source) {
    GLuint id = glCreateShader(kind); glShaderSource(id, 1, &source, NULL); glCompileShader(id);
    GLint ok; glGetShaderiv(id, GL_COMPILE_STATUS, &ok); if (!ok) { glDeleteShader(id); return 0; } return id;
}
int ferro_linux_convert(FerroLinuxFrame *f, intptr_t display, uint32_t output) {
    /* Only the GLES3 external-texture profile is currently admitted. */
    const char *version = (const char *)glGetString(GL_VERSION);
    if (!version || !strstr(version, "OpenGL ES 3")) return 0;
    Alias *a = make_alias(f, (EGLDisplay)display, GL_TEXTURE_EXTERNAL_OES);
    if (!a) return 0;
    const char *vs = "#version 300 es\nprecision highp float;out vec2 uv;void main(){vec2 p=vec2((gl_VertexID<<1)&2,gl_VertexID&2);uv=vec2(p.x,1.0-p.y);gl_Position=vec4(p*2.0-1.0,0,1);}";
    const char *fs = "#version 300 es\n#extension GL_OES_EGL_image_external_essl3 : require\nprecision highp float;uniform samplerExternalOES source;in vec2 uv;out vec4 color;void main(){color=vec4(texture(source,uv).rgb,1);}";
    GLuint v = shader(GL_VERTEX_SHADER, vs), frag = shader(GL_FRAGMENT_SHADER, fs), program = 0;
    if (v && frag) { program = glCreateProgram(); glAttachShader(program, v); glAttachShader(program, frag); glLinkProgram(program); GLint ok; glGetProgramiv(program, GL_LINK_STATUS, &ok); if (!ok) { glDeleteProgram(program); program = 0; } }
    if (v) glDeleteShader(v); if (frag) glDeleteShader(frag);
    if (!program) { ferro_linux_alias_release(a, 1); return 0; }
    GLint old_program, old_vao, draw, read, viewport[4], active, external, texture2d, mask[4];
    glGetIntegerv(GL_CURRENT_PROGRAM, &old_program); glGetIntegerv(GL_VERTEX_ARRAY_BINDING, &old_vao);
    glGetIntegerv(GL_DRAW_FRAMEBUFFER_BINDING, &draw); glGetIntegerv(GL_READ_FRAMEBUFFER_BINDING, &read);
    glGetIntegerv(GL_VIEWPORT, viewport); glGetIntegerv(GL_ACTIVE_TEXTURE, &active); glGetIntegerv(GL_COLOR_WRITEMASK, mask);
    glActiveTexture(GL_TEXTURE0); glGetIntegerv(GL_TEXTURE_BINDING_EXTERNAL_OES, &external); glGetIntegerv(GL_TEXTURE_BINDING_2D, &texture2d);
    GLenum capabilities[] = {GL_BLEND, GL_DEPTH_TEST, GL_STENCIL_TEST, GL_SCISSOR_TEST, GL_CULL_FACE, GL_RASTERIZER_DISCARD, GL_SAMPLE_ALPHA_TO_COVERAGE, GL_SAMPLE_COVERAGE};
    GLboolean enabled[8]; for (int i = 0; i < 8; i++) { enabled[i] = glIsEnabled(capabilities[i]); glDisable(capabilities[i]); }
    GLuint vao, framebuffer; glGenVertexArrays(1, &vao); glBindVertexArray(vao); glGenFramebuffers(1, &framebuffer);
    glBindFramebuffer(GL_FRAMEBUFFER, framebuffer); glFramebufferTexture2D(GL_FRAMEBUFFER, GL_COLOR_ATTACHMENT0, GL_TEXTURE_2D, output, 0);
    int ok = glCheckFramebufferStatus(GL_FRAMEBUFFER) == GL_FRAMEBUFFER_COMPLETE;
    if (ok) {
        glViewport(0, 0, f->info.width, f->info.height); glColorMask(GL_TRUE,GL_TRUE,GL_TRUE,GL_TRUE);
        glUseProgram(program); glBindTexture(GL_TEXTURE_EXTERNAL_OES, a->texture); glUniform1i(glGetUniformLocation(program, "source"), 0);
        glDrawArrays(GL_TRIANGLES, 0, 3); ok = glGetError() == GL_NO_ERROR;
    }
    glBindFramebuffer(GL_DRAW_FRAMEBUFFER, draw); glBindFramebuffer(GL_READ_FRAMEBUFFER, read);
    glBindVertexArray(old_vao); glUseProgram(old_program); glViewport(viewport[0],viewport[1],viewport[2],viewport[3]);
    glColorMask(mask[0],mask[1],mask[2],mask[3]);
    for (int i = 0; i < 8; i++) if (enabled[i]) glEnable(capabilities[i]);
    glBindTexture(GL_TEXTURE_EXTERNAL_OES, external); glBindTexture(GL_TEXTURE_2D, texture2d); glActiveTexture(active);
    glDeleteFramebuffers(1,&framebuffer); glDeleteVertexArrays(1,&vao); glDeleteProgram(program); ferro_linux_alias_release(a,1);
    return ok;
}

#ifdef FERROUI_NATIVE_FRAME_TESTS
#include "frame_tests.c"
#endif

#include <sys/mman.h>
#include <sys/ioctl.h>
#include <linux/dma-buf.h>
static float clamp01(float v) { return fminf(1.f,fmaxf(0.f,v)); }
static uint8_t photo_channel(float v, int rec709) {
    v = clamp01(v);
    if (rec709) { v = powf(v,2.4f); v = v <= .0031308f ? 12.92f*v : 1.055f*powf(v,1.f/2.4f)-.055f; }
    return (uint8_t)lrintf(clamp01(v)*255.f);
}
int ferro_linux_photo_rgb(FerroLinuxFrame *f, uint8_t *rgb, uint64_t capacity) {
    FerroLinuxFrameInfo *v = &f->info;
    if (v->modifier != 0 || v->width > 4096 || v->height > 4096 || capacity != (uint64_t)v->width*v->height*3) return 0;
    uint8_t *planes[4] = {0}; void *maps[4] = {0}; uint64_t lengths[4] = {0};
    int ok = 0;
    for (uint32_t p = 0; p < v->plane_count; p++) {
        struct stat st; if (fstat(v->fds[p],&st) || st.st_size <= 0 || (uint64_t)st.st_size > FRAME_LIMIT) goto cleanup;
        uint64_t rows = p == 0 ? v->height : (v->height+1)/2;
        uint64_t rowbytes = v->format == DRM_NV12 ? (p == 0 ? v->width : ((v->width+1)/2)*2) : (uint64_t)v->width*4;
        uint64_t last = (uint64_t)v->offsets[p] + (rows-1)*v->strides[p] + rowbytes;
        if (last > (uint64_t)st.st_size || v->strides[p] < rowbytes) goto cleanup;
        struct dma_buf_sync sync = { .flags = DMA_BUF_SYNC_START | DMA_BUF_SYNC_READ };
        if (ioctl(v->fds[p], DMA_BUF_IOCTL_SYNC, &sync)) goto cleanup;
        lengths[p] = st.st_size;
        maps[p] = mmap(NULL,lengths[p],PROT_READ,MAP_SHARED,v->fds[p],0);
        if (maps[p] == MAP_FAILED) { maps[p] = NULL; goto cleanup; }
        planes[p] = (uint8_t *)maps[p] + v->offsets[p];
    }
    for (uint32_t y = 0; y < v->height; y++) for (uint32_t x = 0; x < v->width; x++) {
        float r,g,b;
        if (v->format == DRM_NV12) {
            float yy = planes[0][(uint64_t)y*v->strides[0]+x];
            uint64_t uv = (uint64_t)(y/2)*v->strides[1]+(x/2)*2;
            float u = planes[1][uv]-128.f, vv = planes[1][uv+1]-128.f;
            if (v->color == 3) { yy=(yy-16.f)/219.f; u/=224.f; vv/=224.f; } else { yy/=255.f;u/=255.f;vv/=255.f; }
            r=yy+1.5748f*vv;g=yy-.187324f*u-.468124f*vv;b=yy+1.8556f*u;
        } else {
            uint8_t *p = planes[0]+(uint64_t)y*v->strides[0]+x*4;
            r=p[v->format == DRM_XRGB ? 2 : 0]/255.f;g=p[1]/255.f;b=p[v->format == DRM_XRGB ? 0 : 2]/255.f;
        }
        uint8_t *out=rgb+((uint64_t)y*v->width+x)*3;
        out[0]=photo_channel(r,v->color!=1);out[1]=photo_channel(g,v->color!=1);out[2]=photo_channel(b,v->color!=1);
    }
    ok=1;
cleanup:
    for (uint32_t p=0;p<v->plane_count;p++) if (lengths[p]) {
        if (maps[p]) munmap(maps[p],lengths[p]);
        struct dma_buf_sync sync={.flags=DMA_BUF_SYNC_END|DMA_BUF_SYNC_READ};ioctl(v->fds[p],DMA_BUF_IOCTL_SYNC,&sync);
    }
    return ok;
}
