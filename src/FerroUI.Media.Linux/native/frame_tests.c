/* Metadata/ownership tests only. memfd-backed descriptors deliberately do not
 * claim to prove a DMA-BUF import or GPU completion on the receiving host. */
#include <sys/mman.h>
#include <fcntl.h>
#include <stdio.h>
#define REQUIRE(v) do { if (!(v)) { fprintf(stderr,"LINUX_FRAME_CONTRACT_FAILED line=%d\n",__LINE__); return __LINE__; } } while (0)
static GstSample *test_sample(gsize allocation, gsize visible, gsize offset, gint stride, gboolean dma) {
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
    gst_buffer_add_video_meta_full(buffer, GST_VIDEO_FRAME_FLAG_NONE, GST_VIDEO_FORMAT_BGRx, 4, 4, 1, offsets, strides);
    GST_BUFFER_PTS(buffer) = 1234;
    GstCaps *caps = gst_caps_from_string("video/x-raw(memory:DMABuf),format=DMA_DRM,drm-format=XR24,width=4,height=4,framerate=30/1,pixel-aspect-ratio=1/1,interlace-mode=progressive,colorimetry=sRGB");
    GstSample *sample = gst_sample_new(buffer, caps, NULL, NULL);
    gst_buffer_unref(buffer); gst_caps_unref(caps); return sample;
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
#ifdef FERROUI_NATIVE_TEST_MAIN
int main(void) { return ferro_linux_test_frame_contracts(); }
#endif
