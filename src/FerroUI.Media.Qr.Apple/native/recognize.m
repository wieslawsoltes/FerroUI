#import <Foundation/Foundation.h>
#import <Vision/Vision.h>
#import <CoreImage/CoreImage.h>
#import <CoreVideo/CoreVideo.h>
#import <ImageIO/ImageIO.h>
#include <TargetConditionals.h>
#include <stdlib.h>
#include <string.h>

// Synchronous narrow boundary. The bounded Rust analysis worker owns/retains
// the pixel buffer throughout. Vision's internal storage is opaque; the
// framework neither maps nor repacks preview pixels here.
int ferroui_qr_vision_recognize(void *pointer,void **output,size_t *length) {
    @autoreleasepool {
        *output=NULL;*length=0;
        VNDetectBarcodesRequest *request=[VNDetectBarcodesRequest new];
#if TARGET_OS_SIMULATOR
        // Vision's simulator profile has no Neural Engine. Analysis may use
        // CPU compute; this never enables a CPU preview presentation route.
        request.usesCPUOnly=YES;
        // On the tested simulator, revisions 3/4 either fail or report no
        // symbols for independently known input. Revision 1 provides the
        // supported CPU implementation. Device builds retain the OS default.
        request.revision=VNDetectBarcodesRequestRevision1;
#endif
        request.symbologies=@[VNBarcodeSymbologyQR];
        VNImageRequestHandler *handler=[[VNImageRequestHandler alloc] initWithCVPixelBuffer:(CVPixelBufferRef)pointer
            orientation:kCGImagePropertyOrientationUp options:@{}];
        NSError *error=nil;
        if(![handler performRequests:@[request] error:&error]) {NSLog(@"FerroUI native QR recognition failed: %@",error);return 1;}
        if(request.results.count>64) return 2;
        NSMutableArray *results=[NSMutableArray new];
        for(VNBarcodeObservation *observation in request.results) {
            CGPoint corners[]={observation.topLeft,observation.topRight,observation.bottomRight,observation.bottomLeft};
            NSMutableArray *points=[NSMutableArray new];
            for(int i=0;i<4;i++) [points addObject:@[@(corners[i].x),@(1-corners[i].y)]];
            NSMutableArray *codewords=[NSMutableArray new];
            if([observation.barcodeDescriptor isKindOfClass:[CIQRCodeDescriptor class]]) {
                NSData *data=((CIQRCodeDescriptor *)observation.barcodeDescriptor).errorCorrectedPayload;
                const uint8_t *bytes=data.bytes;
                for(NSUInteger i=0;i<data.length;i++) [codewords addObject:@(bytes[i])];
            }
            [results addObject:@{@"text":observation.payloadStringValue ?: (id)[NSNull null],
                @"corners":points,@"codewords":codewords}];
        }
        NSData *encoded=[NSJSONSerialization dataWithJSONObject:results options:0 error:&error];
        if(!encoded) return 1;
        if(encoded.length>1024*1024) return 2;
        void *copy=malloc(encoded.length);
        if(!copy && encoded.length) return 1;
        memcpy(copy,encoded.bytes,encoded.length);*output=copy;*length=encoded.length;return 0;
    }
}
void ferroui_qr_vision_free(void *pointer) {free(pointer);}

// CPU-created analysis-only fixture; never used by a preview route.
void *ferroui_qr_vision_fixture(const uint8_t *luma,size_t width,size_t height) {
    CVPixelBufferRef buffer=NULL;
    if(CVPixelBufferCreate(kCFAllocatorDefault,width,height,kCVPixelFormatType_32BGRA,NULL,&buffer)!=kCVReturnSuccess) return NULL;
    CVPixelBufferLockBaseAddress(buffer,0);
    uint8_t *target=CVPixelBufferGetBaseAddress(buffer);size_t stride=CVPixelBufferGetBytesPerRow(buffer);
    for(size_t y=0;y<height;y++) for(size_t x=0;x<width;x++) {
        uint8_t v=luma[y*width+x];uint8_t *p=target+y*stride+x*4;p[0]=v;p[1]=v;p[2]=v;p[3]=255;
    }
    CVPixelBufferUnlockBaseAddress(buffer,0);return buffer;
}
