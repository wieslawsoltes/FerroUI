#import <Foundation/Foundation.h>
#import <CoreVideo/CoreVideo.h>
#import <IOSurface/IOSurface.h>
#include <stdint.h>
void ferroui_portable_buffer_info(void *p,uint32_t *width,uint32_t *height,uint64_t *stride,uint64_t *bytes,int32_t *color) {
    CVPixelBufferRef buffer=(CVPixelBufferRef)p;
    *width=(uint32_t)CVPixelBufferGetWidth(buffer);*height=(uint32_t)CVPixelBufferGetHeight(buffer);
    *stride=CVPixelBufferGetBytesPerRow(buffer);
    IOSurfaceRef surface=CVPixelBufferGetIOSurface(buffer);
    *bytes=surface?IOSurfaceGetAllocSize(surface):CVPixelBufferGetDataSize(buffer);
    CFTypeRef primaries=CVBufferGetAttachment(buffer,kCVImageBufferColorPrimariesKey,NULL);
    CFTypeRef transfer=CVBufferGetAttachment(buffer,kCVImageBufferTransferFunctionKey,NULL);
    *color=0; // Unknown is not silently interpreted as sRGB.
    if (primaries && CFEqual(primaries,kCVImageBufferColorPrimaries_ITU_R_709_2) && transfer) {
        if (CFEqual(transfer,kCVImageBufferTransferFunction_ITU_R_709_2)) *color=2;
        else if (CFEqual(transfer,kCVImageBufferTransferFunction_sRGB)) *color=1;
    }
}
void *ferroui_portable_buffer_surface(void *p) {return CVPixelBufferGetIOSurface((CVPixelBufferRef)p);}
void ferroui_portable_buffer_release(void *p) {CVPixelBufferRelease((CVPixelBufferRef)p);}
