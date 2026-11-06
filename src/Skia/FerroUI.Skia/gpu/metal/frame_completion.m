#import <Metal/Metal.h>
// Called after Skia submission on the same serial command queue. No wait, poll
// or readback occurs on the UI/render thread. The command buffer retains queue.
void ferro_frame_metal_complete(void *opaqueQueue, void *context, void (*finished)(void *, int)) {
    @autoreleasepool {
        id<MTLCommandQueue> queue = (__bridge id<MTLCommandQueue>)opaqueQueue;
        id<MTLCommandBuffer> buffer = [queue commandBuffer];
        if (!buffer) { finished(context, 0); return; }
        [buffer addCompletedHandler:^(id<MTLCommandBuffer> completed) {
            finished(context, completed.status == MTLCommandBufferStatusCompleted);
        }];
        [buffer commit];
    }
}

#ifdef FERROUI_NATIVE_FRAME_TESTS
// Independent native GPU fixture. No CPU pixel upload: a Metal render pass
// clears IOSurface storage which the Rust test imports and samples directly.
#import <IOSurface/IOSurface.h>
@interface FerroFrameTestOwner : NSObject
@property(nonatomic,strong) id<MTLDevice> device;
@property(nonatomic,strong) id<MTLCommandQueue> queue;
@property(nonatomic,strong) id<MTLTexture> texture;
@property(nonatomic,strong) id<MTLSharedEvent> gate;
@property(nonatomic) IOSurfaceRef surface;
@end
@implementation FerroFrameTestOwner
- (void)dealloc { if (_surface) CFRelease(_surface); }
@end
void *ferro_frame_test_create(double alpha) {
    @autoreleasepool {
        FerroFrameTestOwner *owner = [FerroFrameTestOwner new];
        owner.device = MTLCreateSystemDefaultDevice();
        if (!owner.device) return NULL;
        owner.queue = [owner.device newCommandQueue];
        owner.surface = IOSurfaceCreate((__bridge CFDictionaryRef)@{
            (__bridge NSString *)kIOSurfaceWidth:@8, (__bridge NSString *)kIOSurfaceHeight:@8,
            (__bridge NSString *)kIOSurfaceBytesPerElement:@4,
            (__bridge NSString *)kIOSurfaceBytesPerRow:@32,
            (__bridge NSString *)kIOSurfacePixelFormat:@((uint32_t)'BGRA') });
        if (!owner.surface) return NULL;
        MTLTextureDescriptor *descriptor = [MTLTextureDescriptor texture2DDescriptorWithPixelFormat:MTLPixelFormatBGRA8Unorm width:8 height:8 mipmapped:NO];
        descriptor.usage = MTLTextureUsageRenderTarget | MTLTextureUsageShaderRead;
        owner.texture = [owner.device newTextureWithDescriptor:descriptor iosurface:owner.surface plane:0];
        if (!owner.texture) return NULL;
        id<MTLCommandBuffer> commands = [owner.queue commandBuffer];
        MTLRenderPassDescriptor *pass = [MTLRenderPassDescriptor renderPassDescriptor];
        pass.colorAttachments[0].texture = owner.texture;
        pass.colorAttachments[0].loadAction = MTLLoadActionClear;
        pass.colorAttachments[0].storeAction = MTLStoreActionStore;
        pass.colorAttachments[0].clearColor = MTLClearColorMake(.25*alpha,.5*alpha,.75*alpha,alpha);
        [[commands renderCommandEncoderWithDescriptor:pass] endEncoding];
        [commands commit];
        [commands waitUntilCompleted]; // Fixture setup only, never production.
        return (__bridge_retained void *)owner;
    }
}
void *ferro_frame_test_device(void *p) { return (__bridge void *)((__bridge FerroFrameTestOwner *)p).device; }
void *ferro_frame_test_queue(void *p) { return (__bridge void *)((__bridge FerroFrameTestOwner *)p).queue; }
void *ferro_frame_test_texture(void *p) { return (__bridge void *)((__bridge FerroFrameTestOwner *)p).texture; }
void ferro_frame_test_forget_texture(void *p) { ((__bridge FerroFrameTestOwner *)p).texture = nil; }
uint64_t ferro_frame_test_allocation_size(void *p) { return IOSurfaceGetAllocSize(((__bridge FerroFrameTestOwner *)p).surface); }
uint64_t ferro_frame_test_device_id(void *p) { return ((__bridge FerroFrameTestOwner *)p).device.registryID; }
void ferro_frame_test_block(void *p) {
    FerroFrameTestOwner *owner = (__bridge FerroFrameTestOwner *)p;
    owner.gate = [owner.device newSharedEvent];
    id<MTLCommandBuffer> commands = [owner.queue commandBuffer];
    [commands encodeWaitForEvent:owner.gate value:1];
    [commands commit];
}
void ferro_frame_test_unblock(void *p) { ((__bridge FerroFrameTestOwner *)p).gate.signaledValue = 1; }
void ferro_frame_test_release(void *p) { id owner = (__bridge_transfer id)p; (void)owner; }

#endif
