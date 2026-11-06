#import <Foundation/Foundation.h>
#import <Metal/Metal.h>
#import <CoreVideo/CoreVideo.h>
#import <IOSurface/IOSurfaceRef.h>
#include <stdint.h>
@interface FerroMediaMetalOutput:NSObject
@property(nonatomic) CVPixelBufferRef buffer;
@property(nonatomic,strong) id<MTLTexture> texture;
@property(nonatomic,strong) id<MTLCommandBuffer> commands;
@end
@implementation FerroMediaMetalOutput
- (void)dealloc {if(_buffer)CVPixelBufferRelease(_buffer);}
@end
void ferroui_media_metal_layout(uint32_t width,uint32_t height,uint64_t *stride,uint64_t *bytes){
    *stride=IOSurfaceAlignProperty(kIOSurfaceBytesPerRow,(size_t)width*4);
    *bytes=IOSurfaceAlignProperty(kIOSurfaceAllocSize,(size_t)*stride*height);
}
void *ferroui_media_metal_create(void *device,void *queue,uint32_t width,uint32_t height){
    @autoreleasepool{
        uint64_t stride,bytes;ferroui_media_metal_layout(width,height,&stride,&bytes);
        IOSurfaceRef surface=IOSurfaceCreate((__bridge CFDictionaryRef)@{
            (__bridge NSString*)kIOSurfaceWidth:@(width),(__bridge NSString*)kIOSurfaceHeight:@(height),
            (__bridge NSString*)kIOSurfaceBytesPerElement:@4,(__bridge NSString*)kIOSurfaceBytesPerRow:@(stride),
            (__bridge NSString*)kIOSurfaceAllocSize:@(bytes),(__bridge NSString*)kIOSurfacePixelFormat:@(kCVPixelFormatType_32BGRA)});
        if(!surface)return NULL;
        CVPixelBufferRef buffer=NULL;
        CVReturn status=CVPixelBufferCreateWithIOSurface(kCFAllocatorDefault,surface,
            (__bridge CFDictionaryRef)@{(__bridge NSString*)kCVPixelBufferMetalCompatibilityKey:@YES},&buffer);
        if(status!=kCVReturnSuccess){CFRelease(surface);return NULL;}
        CVBufferSetAttachment(buffer,kCVImageBufferColorPrimariesKey,kCVImageBufferColorPrimaries_ITU_R_709_2,kCVAttachmentMode_ShouldPropagate);
        CVBufferSetAttachment(buffer,kCVImageBufferTransferFunctionKey,kCVImageBufferTransferFunction_sRGB,kCVAttachmentMode_ShouldPropagate);
        MTLTextureDescriptor *description=[MTLTextureDescriptor texture2DDescriptorWithPixelFormat:MTLPixelFormatBGRA8Unorm width:width height:height mipmapped:NO];
        description.usage=MTLTextureUsageRenderTarget|MTLTextureUsageShaderRead;
        description.storageMode=MTLStorageModeShared;
        id<MTLTexture> texture=[(__bridge id<MTLDevice>)device newTextureWithDescriptor:description iosurface:surface plane:0];
        CFRelease(surface);
        if(!texture){CVPixelBufferRelease(buffer);return NULL;}
        FerroMediaMetalOutput *output=[FerroMediaMetalOutput new];output.buffer=buffer;output.texture=texture;
        output.commands=[(__bridge id<MTLCommandQueue>)queue commandBuffer];
        if(!output.commands)return NULL;
        return (__bridge_retained void*)output;
    }
}
void *ferroui_media_metal_buffer(void *p){return CVPixelBufferRetain(((__bridge FerroMediaMetalOutput*)p).buffer);}
void *ferroui_media_metal_texture(void *p){return (__bridge void*)((__bridge FerroMediaMetalOutput*)p).texture;}
void *ferroui_media_metal_commands(void *p){return (__bridge void*)((__bridge FerroMediaMetalOutput*)p).commands;}
void ferroui_media_metal_commit(void *p,void *context,void (*finished)(void*,int)){
    @autoreleasepool{
        id<MTLCommandBuffer> commands=((__bridge FerroMediaMetalOutput*)p).commands;
        [commands addCompletedHandler:^(id<MTLCommandBuffer> completed){
            if(completed.status!=MTLCommandBufferStatusCompleted) NSLog(@"FerroUI media renderer failed: %@",completed.error);
            finished(context,completed.status==MTLCommandBufferStatusCompleted);
        }];
        [commands commit];
    }
}
void ferroui_media_metal_release(void *p){id owner=(__bridge_transfer id)p;(void)owner;}
void *ferroui_media_metal_input(void *device,void *pointer){
    CVPixelBufferRef buffer=pointer;IOSurfaceRef surface=CVPixelBufferGetIOSurface(buffer);
    if(!surface || CVPixelBufferIsPlanar(buffer))return NULL;
    OSType format=CVPixelBufferGetPixelFormatType(buffer);
    if(format!=kCVPixelFormatType_32BGRA && format!=kCVPixelFormatType_32RGBA)return NULL;
    MTLTextureDescriptor *description=[MTLTextureDescriptor texture2DDescriptorWithPixelFormat:format==kCVPixelFormatType_32BGRA?MTLPixelFormatBGRA8Unorm:MTLPixelFormatRGBA8Unorm
        width:CVPixelBufferGetWidth(buffer) height:CVPixelBufferGetHeight(buffer) mipmapped:NO];
    description.usage=MTLTextureUsageShaderRead;description.storageMode=MTLStorageModeShared;
    id<MTLTexture> texture=[(__bridge id<MTLDevice>)device newTextureWithDescriptor:description iosurface:surface plane:0];
    return (__bridge_retained void*)texture;
}
