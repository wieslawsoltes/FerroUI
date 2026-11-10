#import <Metal/Metal.h>
// The texture of a frame layer that several Graphite recorders draw into
// (pipelined_layer.rs). Graphite makes the texture of an ordinary layer on
// the recorder of the frames, where no other recorder reaches it; this one
// is made here and wrapped once by every recorder that draws a band of it.
// The description is the one Graphite gives a render target surface: one
// sample, no mipmaps, private storage, drawn to and sampled.
void *ferro_skia_layer_texture_new(void *opaqueDevice, int width, int height, int rgba) {
    @autoreleasepool {
        id<MTLDevice> device = (__bridge id<MTLDevice>)opaqueDevice;
        if (!device || width <= 0 || height <= 0) { return NULL; }
        MTLPixelFormat format = rgba ? MTLPixelFormatRGBA8Unorm : MTLPixelFormatBGRA8Unorm;
        MTLTextureDescriptor *descriptor =
            [MTLTextureDescriptor texture2DDescriptorWithPixelFormat:format
                                                               width:(NSUInteger)width
                                                              height:(NSUInteger)height
                                                           mipmapped:NO];
        descriptor.usage = MTLTextureUsageRenderTarget | MTLTextureUsageShaderRead;
        descriptor.storageMode = MTLStorageModePrivate;
        id<MTLTexture> texture = [device newTextureWithDescriptor:descriptor];
        if (!texture) { return NULL; }
        return (__bridge_retained void *)texture;
    }
}

// Gives back the reference `ferro_skia_layer_texture_new` returned.
void ferro_skia_layer_texture_release(void *opaqueTexture) {
    if (!opaqueTexture) { return; }
    id<MTLTexture> texture = (__bridge_transfer id<MTLTexture>)opaqueTexture;
    (void)texture;
}
