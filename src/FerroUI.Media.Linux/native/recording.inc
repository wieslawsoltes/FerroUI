/* A recording-only pipeline consumes admitted linear camera samples.
 * Its raw conversion is independent of the EGL preview and has no audio source. */
#define MOVIE_LIMIT (256u * 1024 * 1024)
#define MOVIE_DURATION (60 * GST_SECOND)

struct FerroLinuxRecording {
    gint refs, worker_done;
    GMutex lock;
    GstCaps *input_caps;
    GstBuffer *pending;
    GstClockTime frame_duration;
    gboolean finish;
    int status;
    FerroLinuxRecordingInfo info;
    GByteArray *encoded;
};

static FerroLinuxRecording *ref_recording(FerroLinuxRecording *r) {
    g_atomic_int_inc(&r->refs);
    return r;
}

static void unref_recording(FerroLinuxRecording *r) {
    if (!r || !g_atomic_int_dec_and_test(&r->refs)) return;
    gst_caps_unref(r->input_caps);
    gst_clear_buffer(&r->pending);
    g_clear_pointer(&r->encoded, g_byte_array_unref);
    g_mutex_clear(&r->lock);
    g_free(r);
}

void ferro_linux_record_cancel(FerroLinuxRecording *r) {
    if (!r) return;
    g_mutex_lock(&r->lock);
    r->status = -2;
    gst_clear_buffer(&r->pending);
    g_clear_pointer(&r->encoded, g_byte_array_unref);
    g_mutex_unlock(&r->lock);
}

void ferro_linux_record_release(FerroLinuxRecording *r) {
    ferro_linux_record_cancel(r);
    unref_recording(r);
}

void ferro_linux_record_finish(FerroLinuxRecording *r) {
    g_mutex_lock(&r->lock);
    r->finish = TRUE;
    g_mutex_unlock(&r->lock);
}

int ferro_linux_record_poll(FerroLinuxRecording *r, FerroLinuxRecordingInfo *out) {
    g_mutex_lock(&r->lock);
    *out = r->info;
    int status = r->status;
    g_mutex_unlock(&r->lock);
    return status;
}

int ferro_linux_record_copy(FerroLinuxRecording *r, uint8_t *bytes, uint64_t length) {
    g_mutex_lock(&r->lock);
    gboolean complete = r->status == 2 && length == r->encoded->len;
    if (complete) memcpy(bytes, r->encoded->data, r->encoded->len);
    g_mutex_unlock(&r->lock);
    return complete;
}

static gboolean recording_plugins_available(void) {
    const char *names[] = {"appsrc", "videoconvert", "openh264enc", "h264parse", "mp4mux"};
    for (guint i = 0; i < G_N_ELEMENTS(names); ++i) {
        GstElementFactory *factory = gst_element_factory_find(names[i]);
        if (!factory) return FALSE;
        gst_object_unref(factory);
    }
    return TRUE;
}

static GstCaps *recording_caps(GstCaps *camera_caps) {
    GstVideoInfoDmaDrm drm;
    GstVideoInfo info;
    gst_video_info_dma_drm_init(&drm);
    if (!gst_video_info_dma_drm_from_caps(&drm, camera_caps) || drm.drm_modifier ||
        !gst_video_info_dma_drm_to_video_info(&drm, &info) ||
        info.width < 16 || info.height < 16 || info.width > 1920 || info.height > 1080 ||
        info.width % 2 || info.height % 2 || info.fps_n <= 0 || info.fps_d <= 0 ||
        info.fps_n > 60ll * info.fps_d) return NULL;
    return gst_video_info_to_caps(&info);
}

static void recording_enqueue(FerroLinuxRecording *r, GstBuffer *buffer) {
    g_mutex_lock(&r->lock);
    if (r->status == 1 && !r->finish && !r->pending) r->pending = gst_buffer_ref(buffer);
    g_mutex_unlock(&r->lock);
}

static gpointer recording_worker(gpointer data) {
    FerroLinuxRecording *r = data;
    GError *parse_error = NULL;
    GstElement *pipeline = gst_parse_launch(
        "appsrc name=source is-live=true format=time max-buffers=1 max-bytes=0 block=false leaky-type=downstream "
        "! videoconvert ! video/x-raw,format=I420 ! openh264enc bitrate=8000000 gop-size=30 "
        "! h264parse ! mp4mux fragment-duration=1000 streamable=true "
        "! appsink name=output sync=false max-buffers=2 enable-last-sample=false", &parse_error);
    GstElement *source = NULL, *sink = NULL;
    GstBus *bus = NULL;
    GByteArray *encoded = NULL;
    const char *error = "Could not configure the GStreamer H.264 recording pipeline";
    GstClockTime first = GST_CLOCK_TIME_NONE, end = 0;
    if (!pipeline || parse_error) goto done;
    source = gst_bin_get_by_name(GST_BIN(pipeline), "source");
    sink = gst_bin_get_by_name(GST_BIN(pipeline), "output");
    if (!source || !sink) goto done;
    gst_app_src_set_caps(GST_APP_SRC(source), r->input_caps);
    bus = gst_element_get_bus(pipeline);
    if (gst_element_set_state(pipeline, GST_STATE_PLAYING) == GST_STATE_CHANGE_FAILURE) goto done;

    g_mutex_lock(&r->lock);
    if (r->status == 0) r->status = 1;
    g_mutex_unlock(&r->lock);
    encoded = g_byte_array_new();
    gint64 deadline = g_get_monotonic_time() + 60 * G_TIME_SPAN_SECOND;
    gboolean draining = FALSE;
    while (TRUE) {
        g_mutex_lock(&r->lock);
        gboolean cancelled = r->status == -2;
        gboolean finish = r->finish;
        GstBuffer *buffer = g_steal_pointer(&r->pending);
        g_mutex_unlock(&r->lock);
        if (cancelled) { gst_clear_buffer(&buffer); break; }

        gint64 now = g_get_monotonic_time();
        if (buffer && !draining && now < deadline) {
            GstClockTime timestamp = GST_BUFFER_PTS(buffer);
            if (!GST_CLOCK_TIME_IS_VALID(first)) first = timestamp;
            if (timestamp >= first && timestamp - first >= MOVIE_DURATION) finish = TRUE;
            if (timestamp >= first && timestamp - first >= end && timestamp - first < MOVIE_DURATION) {
                GstBuffer *input = gst_buffer_copy(buffer);
                GST_BUFFER_PTS(input) = timestamp - first;
                GST_BUFFER_DTS(input) = GST_CLOCK_TIME_NONE;
                GST_BUFFER_DURATION(input) = MIN(r->frame_duration, MOVIE_DURATION - GST_BUFFER_PTS(input));
                end = GST_BUFFER_PTS(input) + GST_BUFFER_DURATION(input);
                if (gst_app_src_push_buffer(GST_APP_SRC(source), input) != GST_FLOW_OK)
                    error = "GStreamer rejected a camera recording sample";
                else
                    error = NULL;
            }
        }
        gst_clear_buffer(&buffer);
        if (end && error) break;
        if (!draining && (finish || now >= deadline)) {
            if (!end || gst_app_src_end_of_stream(GST_APP_SRC(source)) != GST_FLOW_OK) {
                error = "Recording finished before a camera sample was encoded";
                break;
            }
            g_mutex_lock(&r->lock);
            r->finish = TRUE;
            gst_clear_buffer(&r->pending);
            g_mutex_unlock(&r->lock);
            draining = TRUE;
            deadline = now + 5 * G_TIME_SPAN_SECOND;
        }
        if (draining && now >= deadline) { error = "GStreamer recording finalization timed out"; break; }
        GstMessage *message = gst_bus_pop_filtered(bus, GST_MESSAGE_ERROR);
        if (message) {
            gst_message_unref(message);
            error = "GStreamer could not encode the negotiated camera samples";
            break;
        }
        GstSample *sample = gst_app_sink_try_pull_sample(GST_APP_SINK(sink), 5 * GST_MSECOND);
        if (!sample) {
            if (draining && gst_app_sink_is_eos(GST_APP_SINK(sink))) break;
            continue;
        }
        GstMapInfo map;
        GstBuffer *output = gst_sample_get_buffer(sample);
        if (!gst_buffer_map(output, &map, GST_MAP_READ)) {
            gst_sample_unref(sample);
            error = "Could not read encoded recording";
            break;
        }
        gboolean fits = map.size <= MOVIE_LIMIT - encoded->len;
        if (fits) g_byte_array_append(encoded, map.data, map.size);
        gst_buffer_unmap(output, &map);
        gst_sample_unref(sample);
        if (!fits) { error = "Encoded recording exceeds 256 MiB"; break; }
    }

done:
    g_clear_error(&parse_error);
    if (pipeline) gst_element_set_state(pipeline, GST_STATE_NULL);
    gst_clear_object(&bus);
    gst_clear_object(&source);
    gst_clear_object(&sink);
    gst_clear_object(&pipeline);
    g_mutex_lock(&r->lock);
    gst_clear_buffer(&r->pending);
    if (r->status >= 0) {
        if (error || !encoded || !encoded->len) {
            g_strlcpy(r->info.error, error ? error : "No encoded recording", sizeof(r->info.error));
            r->status = -1;
        } else {
            r->encoded = g_steal_pointer(&encoded);
            r->info.length = r->encoded->len;
            r->info.duration_ns = end;
            r->status = 2;
        }
    }
    g_atomic_int_set(&r->worker_done, 1);
    g_mutex_unlock(&r->lock);
    g_clear_pointer(&encoded, g_byte_array_unref);
    unref_recording(r);
    return NULL;
}
