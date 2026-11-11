/* Metadata/ownership tests only. memfd-backed descriptors deliberately do not
 * claim to prove a DMA-BUF import or GPU completion on the receiving host. */
#include <sys/mman.h>
#include <fcntl.h>
#include <stdio.h>
#define REQUIRE(v) do { if (!(v)) { fprintf(stderr,"LINUX_FRAME_CONTRACT_FAILED line=%d\n",__LINE__); return __LINE__; } } while (0)
static GstSample *test_sample_with_size(gsize allocation, gsize visible, gsize offset, gint stride,
                                        gboolean dma, guint width, guint height, GstClockTime pts) {
    GstBuffer *buffer = gst_buffer_new();
    if (dma) {
        int fd = memfd_create("ferroui-frame-contract", MFD_CLOEXEC);
        if (fd < 0 || ftruncate(fd, allocation)) return NULL;
        GstAllocator *allocator = gst_dmabuf_allocator_new();
        GstMemory *memory = gst_dmabuf_allocator_alloc(allocator, fd, visible);
        gst_buffer_append_memory(buffer, memory); gst_object_unref(allocator);
    } else gst_buffer_append_memory(buffer, gst_allocator_alloc(NULL, visible, NULL));
    gsize offsets[GST_VIDEO_MAX_PLANES] = {offset};
    gint strides[GST_VIDEO_MAX_PLANES] = {stride};
    gst_buffer_add_video_meta_full(buffer, GST_VIDEO_FRAME_FLAG_NONE, GST_VIDEO_FORMAT_BGRx, width, height, 1, offsets, strides);
    GST_BUFFER_PTS(buffer) = pts;
    GstCaps *caps = gst_caps_from_string("video/x-raw(memory:DMABuf),format=DMA_DRM,drm-format=XR24,width=4,height=4,framerate=30/1,pixel-aspect-ratio=1/1,interlace-mode=progressive,colorimetry=sRGB");
    gst_caps_set_simple(caps, "width", G_TYPE_INT, width, "height", G_TYPE_INT, height, NULL);
    GstSample *sample = gst_sample_new(buffer, caps, NULL, NULL);
    gst_buffer_unref(buffer); gst_caps_unref(caps); return sample;
}
static GstSample *test_sample(gsize allocation, gsize visible, gsize offset, gint stride, gboolean dma) {
    return test_sample_with_size(allocation, visible, offset, stride, dma, 4, 4, 1234);
}
int ferro_linux_test_frame_contracts(void) {
    gst_init(NULL,NULL);
    GstSample *sample = test_sample(64,64,0,16,TRUE); REQUIRE(sample);
    FerroLinuxFrame *frame = admit_sample(sample); REQUIRE(frame);
    int fd = frame->info.fds[0]; gst_sample_unref(sample);
    REQUIRE(fcntl(fd,F_GETFD)>=0); REQUIRE(frame->info.bytes==64 && frame->info.timestamp_ns==1234);
    ferro_linux_frame_release(frame); REQUIRE(fcntl(fd,F_GETFD)==-1);
    struct {gsize allocation,visible,offset;gint stride;gboolean dma;} invalid[] = {
        {8,64,0,16,TRUE}, /* FD allocation shorter than the complete plane. */
        {64,8,0,16,TRUE}, /* GstMemory visible range shorter than the plane. */
        {64,64,60,16,TRUE}, /* Last row exceeds the allocation. */
        {64,64,0,4,TRUE}, /* Rows overlap / fewer than four pixels. */
        {64,64,0,16,FALSE}, /* System memory must not enter preview. */
    };
    for (guint i=0;i<G_N_ELEMENTS(invalid);i++) {
        sample=test_sample(invalid[i].allocation,invalid[i].visible,invalid[i].offset,invalid[i].stride,invalid[i].dma);
        REQUIRE(sample); frame=admit_sample(sample); gst_sample_unref(sample);
        if(frame){ferro_linux_frame_release(frame);fprintf(stderr,"accepted invalid descriptor %u\n",i);return 1;}
    }
    puts("LINUX_FRAME_CONTRACT_PASS retained_fd=true invalid_extents=4 system_memory=rejected metadata_only=true");
    return 0;
}
static gboolean decode_recording(GstBuffer *movie) {
    GError *error = NULL;
    GstElement *pipeline = gst_parse_launch(
        "appsrc name=source ! qtdemux name=demux demux.video_0 ! h264parse ! openh264dec ! "
        "appsink name=output sync=false", &error);
    gboolean valid = FALSE;
    if (pipeline && !error) {
        GstElement *source = gst_bin_get_by_name(GST_BIN(pipeline), "source");
        GstElement *sink = gst_bin_get_by_name(GST_BIN(pipeline), "output");
        GstElement *demux = gst_bin_get_by_name(GST_BIN(pipeline), "demux");
        gst_element_set_state(pipeline, GST_STATE_PLAYING);
        gst_app_src_push_buffer(GST_APP_SRC(source), gst_buffer_ref(movie));
        gst_app_src_end_of_stream(GST_APP_SRC(source));
        guint frames = 0;
        GstSample *sample;
        while ((sample = gst_app_sink_try_pull_sample(GST_APP_SINK(sink), GST_SECOND))) {
            GstVideoInfo info;
            gboolean size = gst_video_info_from_caps(&info, gst_sample_get_caps(sample)) &&
                info.width == 320 && info.height == 240;
            gst_sample_unref(sample);
            if (!size) break;
            ++frames;
        }
        GstPad *audio = gst_element_get_static_pad(demux, "audio_0");
        valid = frames >= 2 && gst_app_sink_is_eos(GST_APP_SINK(sink)) && !audio;
        gst_clear_object(&audio);
        gst_element_set_state(pipeline, GST_STATE_NULL);
        gst_object_unref(demux);
        gst_object_unref(sink);
        gst_object_unref(source);
    }
    g_clear_error(&error);
    gst_clear_object(&pipeline);
    return valid;
}

int ferro_linux_test_recording(void) {
    gst_init(NULL, NULL);
    if (!recording_plugins_available()) return -1;
    const gsize bytes = 320 * 240 * 4;
    FerroLinuxSession *session = g_new0(FerroLinuxSession, 1);
    g_mutex_init(&session->lock);
    session->refs = 1;
    session->remote_fd = -1;
    session->epoch = session->segment_epoch = 1;
    session->state.state = 2;
    session->recording_available = TRUE;
    FerroLinuxRecording *recording = NULL;
    GstBuffer *movie = NULL;
    int result = 1;
    if (receive(session, test_sample_with_size(bytes, bytes, 0, 1280, TRUE, 320, 240, 0)) != GST_FLOW_OK)
        goto done;
    recording = ferro_linux_record(session);
    if (!recording) goto done;
    FerroLinuxRecordingInfo info;
    gint64 deadline = g_get_monotonic_time() + 5 * G_TIME_SPAN_SECOND;
    int status;
    while (!(status = ferro_linux_record_poll(recording, &info)) && g_get_monotonic_time() < deadline)
        g_usleep(5000);
    if (status != 1) goto done;
    for (int i = 1; i <= 10; ++i) {
        if (receive(session, test_sample_with_size(bytes, bytes, 0, 1280, TRUE, 320, 240,
            i * (GST_SECOND / 30))) != GST_FLOW_OK) goto done;
        g_usleep(34000);
    }
    ferro_linux_record_finish(recording);
    while ((status = ferro_linux_record_poll(recording, &info)) == 1 && g_get_monotonic_time() < deadline)
        g_usleep(5000);
    if (status != 2 || !info.length || !info.duration_ns) goto done;

    movie = gst_buffer_new_allocate(NULL, info.length, NULL);
    GstMapInfo map;
    if (!gst_buffer_map(movie, &map, GST_MAP_WRITE)) goto done;
    gboolean copied = ferro_linux_record_copy(recording, map.data, map.size);
    gst_buffer_unmap(movie, &map);
    if (!copied || !decode_recording(movie)) goto done;

    ferro_linux_close(session);
    session = NULL;
    if (ferro_linux_record_poll(recording, &info) != -2 || !decode_recording(movie)) goto done;
    result = 0;
done:
    if (result && recording) {
        ferro_linux_record_poll(recording, &info);
        fprintf(stderr, "Linux recording fixture failed: %s\n", info.error);
    }
    gst_clear_buffer(&movie);
    if (recording) ferro_linux_record_release(recording);
    if (session) ferro_linux_close(session);
    return result;
}

#ifdef FERROUI_NATIVE_TEST_MAIN
int main(void) {
    int frames = ferro_linux_test_frame_contracts();
    if (frames) return frames;
    int recording = ferro_linux_test_recording();
    if (recording < 0) {
        puts("LINUX_NATIVE_RECORDING_SKIPPED missing GStreamer recording plugins");
        return 0;
    }
    if (recording) return recording;
    puts("LINUX_NATIVE_RECORDING_PASS");
    return 0;
}
#endif
