#import <AVFoundation/AVFoundation.h>
#import <CoreVideo/CoreVideo.h>
#import <ImageIO/ImageIO.h>
#import <TargetConditionals.h>
#include <atomic>
#include <cmath>
#include <cstdint>
#include <cstdio>

// One native photo request across generations. A canceled request keeps this
// slot until AVFoundation's final callback, including when its Rust future dies.
static std::atomic<bool> FerroPortablePhotoBusy{false};
#ifdef FERROUI_CAMERA_LIFETIME_TESTS
static std::atomic<int> FerroCameraTestDrops{0}, FerroCameraTestStops{0};
#endif
@class FerroPortableCamera;
@interface FerroPortablePhoto : NSObject<AVCapturePhotoCaptureDelegate>
@property(nonatomic,weak) FerroPortableCamera *owner;
@property(nonatomic,strong) FerroPortablePhoto *keepAlive;
@property(nonatomic,strong) AVCapturePhotoOutput *output;
@property(nonatomic,strong) NSData *data;
@property(nonatomic,copy) NSString *error;
@property(nonatomic) int status;
@property(nonatomic) uint32_t width,height,orientation;
@property(nonatomic) double timestamp;
@end
@implementation FerroPortablePhoto
- (void)photoOutput:(AVCapturePhotoOutput *)output didFinishProcessingPhoto:(AVCapturePhoto *)photo error:(NSError *)error {
    (void)output;
    @synchronized(self) {
        if (_status != 0) return;
        NSData *data = error ? nil : photo.fileDataRepresentation;
        if (!data.length || data.length > 64 * 1024 * 1024) {
            _status=-1; _error=error.localizedDescription ?: @"Native JPEG is unavailable or exceeds 64 MiB";
            return;
        }
        _data=data;
        _width=(uint32_t)photo.resolvedSettings.photoDimensions.width;
        _height=(uint32_t)photo.resolvedSettings.photoDimensions.height;
        CGImageSourceRef source=CGImageSourceCreateWithData((__bridge CFDataRef)data,nullptr);
        NSDictionary *properties=source?CFBridgingRelease(CGImageSourceCopyPropertiesAtIndex(source,0,nullptr)):nil;
        if(source)CFRelease(source);
        if (properties[(id)kCGImagePropertyPixelWidth] && properties[(id)kCGImagePropertyPixelHeight]) {
            _width=[properties[(id)kCGImagePropertyPixelWidth] unsignedIntValue];
            _height=[properties[(id)kCGImagePropertyPixelHeight] unsignedIntValue];
        }
        _orientation=[properties[(id)kCGImagePropertyOrientation] unsignedIntValue];
        if (!_orientation) _orientation=1;
        _timestamp=CMTimeGetSeconds(photo.timestamp);
    }
}
- (void)photoOutput:(AVCapturePhotoOutput *)output didFinishCaptureForResolvedSettings:(AVCaptureResolvedPhotoSettings *)settings error:(NSError *)error {
    (void)output; (void)settings;
    __attribute__((objc_precise_lifetime)) FerroPortablePhoto *strongSelf=self;
    (void)strongSelf;
    @synchronized(self) {
        if (_status==0) {
            _status=error || !_data ? -1 : 1;
            if (_status<0) _error=error.localizedDescription ?: @"Native photo capture failed";
        }
        _output=nil;
        _keepAlive=nil;
    }
    FerroPortablePhotoBusy.store(false);
}
@end

@interface FerroPortableCamera : NSObject<AVCaptureVideoDataOutputSampleBufferDelegate>
@property(nonatomic,strong) AVCaptureSession *session;
@property(nonatomic,strong) AVCaptureDevice *device;
@property(nonatomic,strong) AVCaptureVideoDataOutput *video;
@property(nonatomic,strong) AVCapturePhotoOutput *photo;
@property(nonatomic,strong) dispatch_queue_t queue;
@property(nonatomic,strong) id disconnectObserver,runtimeObserver;
@property(nonatomic,copy) NSDictionary *settings;
@property(nonatomic,copy) NSString *error;
@property(nonatomic) int status;
@property(nonatomic) BOOL stopped;
@property(nonatomic) CVPixelBufferRef latest;
@property(nonatomic) double timestamp;
- (void)close;
@end
@implementation FerroPortableCamera
- (void)captureOutput:(AVCaptureOutput *)output didOutputSampleBuffer:(CMSampleBufferRef)sample fromConnection:(AVCaptureConnection *)connection {
    (void)output; (void)connection;
    CVPixelBufferRef buffer=CMSampleBufferGetImageBuffer(sample);
    if (!buffer) return;
    @synchronized(self) {
        if (_stopped || _status!=1) return;
        if (_latest) CVPixelBufferRelease(_latest);
        _latest=CVPixelBufferRetain(buffer);
        _timestamp=CMTimeGetSeconds(CMSampleBufferGetPresentationTimeStamp(sample));
    }
}
- (void)close {
    AVCaptureSession *session;
    AVCaptureVideoDataOutput *video;
    id disconnectObserver,runtimeObserver;
    @synchronized(self) {
        if (_stopped) return;
        _stopped=YES;
        if (_latest) {CVPixelBufferRelease(_latest);_latest=nullptr;}
        _status=-6;_error=@"Camera was stopped";
        session=_session;video=_video;
        _session=nil;_video=nil;_photo=nil;_device=nil;
        disconnectObserver=_disconnectObserver;runtimeObserver=_runtimeObserver;
        _disconnectObserver=nil;_runtimeObserver=nil;
    }
    if (disconnectObserver) [[NSNotificationCenter defaultCenter] removeObserver:disconnectObserver];
    if (runtimeObserver) [[NSNotificationCenter defaultCenter] removeObserver:runtimeObserver];
    // Release invokes close while the owner is alive. Keep the callback target
    // until earlier callbacks drain; detach it and release inputs off the UI.
    FerroPortableCamera *lifetime=self;
    dispatch_async(_queue, ^{
        @try {
            [video setSampleBufferDelegate:nil queue:nullptr];
            [session stopRunning];
            if (session) {
                [session beginConfiguration];
                for (AVCaptureInput *input in [session.inputs copy]) [session removeInput:input];
                for (AVCaptureOutput *output in [session.outputs copy]) [session removeOutput:output];
                [session commitConfiguration];
            }
        } @catch (NSException *exception) {(void)exception;}
        @synchronized(lifetime) {} // Retain the callback target through teardown.
    });
}


- (void)dealloc {
    if (_latest) CVPixelBufferRelease(_latest);
    if (_disconnectObserver) [[NSNotificationCenter defaultCenter] removeObserver:_disconnectObserver];
    if (_runtimeObserver) [[NSNotificationCenter defaultCenter] removeObserver:_runtimeObserver];
#ifdef FERROUI_CAMERA_LIFETIME_TESTS
    FerroCameraTestDrops.fetch_add(1);
#endif
}
@end

static int FerroCameraFacing(AVCaptureDevice *device) {
    if (device.position==AVCaptureDevicePositionFront) return 1;
    if (device.position==AVCaptureDevicePositionBack) return 2;
    return 3;
}
static NSArray<AVCaptureDevice *> *FerroCameraDevices() {
    // Discovery alone creates neither device inputs nor sessions. The older
    // enumeration preserves the framework's OS floor and includes USB cameras.
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wdeprecated-declarations"
    return [AVCaptureDevice devicesWithMediaType:AVMediaTypeVideo];
#pragma clang diagnostic pop
}
static NSDictionary *FerroCameraDeviceInfo(AVCaptureDevice *device) {
    NSMutableArray *formats=[NSMutableArray array];
    for (AVCaptureDeviceFormat *format in device.formats) {
        CMVideoDimensions d=CMVideoFormatDescriptionGetDimensions(format.formatDescription);
        for (AVFrameRateRange *range in format.videoSupportedFrameRateRanges) {
            if (d.width>0 && d.height>0 && range.maxFrameRate>=1)
                [formats addObject:@{@"width":@(d.width),@"height":@(d.height),@"fps":@(range.maxFrameRate)}];
        }
    }
    return @{@"id":device.uniqueID,@"label":device.localizedName,@"facing":@(FerroCameraFacing(device)),@"formats":formats};
}
static char *FerroCameraJSON(id value) {
    NSData *data=[NSJSONSerialization dataWithJSONObject:value options:0 error:nil];
    if (!data) return nullptr;
    char *out=(char *)malloc(data.length+1);
    if (!out) return nullptr;
    memcpy(out,data.bytes,data.length);out[data.length]=0;
    return out;
}
static void FerroCameraFail(FerroPortableCamera *owner,int code,NSString *message) {
    @synchronized(owner) {
        if (!owner.stopped) {owner.status=code;owner.error=message;}
    }
}
static void FerroCameraConfigure(FerroPortableCamera *owner,NSString *deviceID,int facing,uint32_t width,uint32_t height,double fps,BOOL exact) {
    dispatch_async(owner.queue, ^{
        @autoreleasepool {
          @try {
            @synchronized(owner) {if (owner.stopped) return;}
            NSArray<AVCaptureDevice *> *devices=FerroCameraDevices();
            AVCaptureDevice *selected=nil;
            for (AVCaptureDevice *device in devices) {
                if (deviceID.length ? [device.uniqueID isEqualToString:deviceID] : (!facing || FerroCameraFacing(device)==facing)) {selected=device;break;}
            }
            if (!selected && !exact && !deviceID.length) selected=devices.firstObject;
            if (!selected) {FerroCameraFail(owner,-2,@"Requested camera is unavailable");return;}
            // A video-only provider must never acquire an input that also
            // requests audio, including legacy muxed camera devices.
            if ([selected hasMediaType:AVMediaTypeAudio] || [selected hasMediaType:AVMediaTypeMuxed]) {
                FerroCameraFail(owner,-4,@"This capture profile accepts video-only devices");return;
            }
            AVCaptureSession *session=[AVCaptureSession new];
            NSError *error=nil;
            AVCaptureDeviceInput *input=[AVCaptureDeviceInput deviceInputWithDevice:selected error:&error];
            if (!input || ![session canAddInput:input]) {FerroCameraFail(owner,-5,error.localizedDescription ?: @"Cannot acquire camera");return;}
            [session beginConfiguration];[session addInput:input];
            AVCaptureDeviceFormat *chosen=nil;
            double best=INFINITY;
            double selectedFPS=0;
            double requestedFPS=fps>0?fps:30.;
            for (AVCaptureDeviceFormat *format in selected.formats) {
                CMVideoDimensions d=CMVideoFormatDescriptionGetDimensions(format.formatDescription);
                if (d.width<=0 || d.height<=0 || d.width>8192 || d.height>8192) continue;
                for (AVFrameRateRange *range in format.videoSupportedFrameRateRanges) {
                    double rate=std::fmin(range.maxFrameRate,std::fmax(range.minFrameRate,requestedFPS));
                    if (!rate) continue;
                    if (exact && width && (d.width!=(int32_t)width || d.height!=(int32_t)height || std::abs(rate-fps)>1e-6)) continue;
                    double score=std::abs((double)d.width-(width?:1280))+std::abs((double)d.height-(height?:720))+100.*std::abs((double)rate-requestedFPS);
                    if (score<best) {best=score;chosen=format;selectedFPS=rate;}
                }
            }
            if (!chosen || ![selected lockForConfiguration:&error]) {
                [session commitConfiguration];FerroCameraFail(owner,-4,error.localizedDescription ?: @"Requested camera format is unsupported");return;
            }
            @try {
                selected.activeFormat=chosen;
                selected.activeVideoMinFrameDuration=CMTimeMakeWithSeconds(1./selectedFPS,1000000000);
                selected.activeVideoMaxFrameDuration=CMTimeMakeWithSeconds(1./selectedFPS,1000000000);
            } @finally {[selected unlockForConfiguration];}
            AVCaptureVideoDataOutput *video=[AVCaptureVideoDataOutput new];
            video.alwaysDiscardsLateVideoFrames=YES;
            // Unlike pixel-buffer attributes, this dictionary accepts only
            // the pixel-format key on older iOS profiles. Acquisition later
            // verifies that the delivered storage has an IOSurface.
            video.videoSettings=@{(id)kCVPixelBufferPixelFormatTypeKey:@(kCVPixelFormatType_32BGRA)};
            if (![session canAddOutput:video]) {[session commitConfiguration];FerroCameraFail(owner,-4,@"Camera cannot deliver BGRA preview");return;}
            [session addOutput:video];[video setSampleBufferDelegate:owner queue:owner.queue];
            AVCaptureConnection *connection=[video connectionWithMediaType:AVMediaTypeVideo];
            if (connection.isVideoMirroringSupported) {connection.automaticallyAdjustsVideoMirroring=NO;connection.videoMirrored=NO;}
            AVCapturePhotoOutput *photo=[AVCapturePhotoOutput new];
            if ([session canAddOutput:photo]) [session addOutput:photo];else photo=nil;
            BOOL jpeg=photo && [photo.availablePhotoCodecTypes containsObject:AVVideoCodecTypeJPEG];
            [session commitConfiguration];
            @synchronized(owner) {
                if (owner.stopped) return;
                owner.session=session;owner.device=selected;owner.video=video;owner.photo=jpeg?photo:nil;
            }
            __weak FerroPortableCamera *weak=owner;
            id disconnectObserver=[[NSNotificationCenter defaultCenter] addObserverForName:AVCaptureDeviceWasDisconnectedNotification object:selected queue:nil usingBlock:^(NSNotification *note){(void)note;FerroPortableCamera *live=weak;if(live)FerroCameraFail(live,-3,@"Camera disconnected");}];
            id runtimeObserver=[[NSNotificationCenter defaultCenter] addObserverForName:AVCaptureSessionRuntimeErrorNotification object:session queue:nil usingBlock:^(NSNotification *note){FerroPortableCamera *live=weak;if(live)FerroCameraFail(live,-5,((NSError *)note.userInfo[AVCaptureSessionErrorKey]).localizedDescription ?: @"Camera runtime failure");}];
            @synchronized(owner) {
                if (owner.stopped) {
                    [[NSNotificationCenter defaultCenter] removeObserver:disconnectObserver];
                    [[NSNotificationCenter defaultCenter] removeObserver:runtimeObserver];
                    return;
                }
                owner.disconnectObserver=disconnectObserver;owner.runtimeObserver=runtimeObserver;
            }
            [session startRunning];
            CMVideoDimensions actual=CMVideoFormatDescriptionGetDimensions(selected.activeFormat.formatDescription);
            double actualFPS=1./CMTimeGetSeconds(selected.activeVideoMinFrameDuration);
            @synchronized(owner) {
                if (owner.stopped || owner.status<0) return;
                owner.settings=@{@"device":FerroCameraDeviceInfo(selected),@"width":@(actual.width),@"height":@(actual.height),@"fps":@(actualFPS),@"native_still":@(jpeg)};
                owner.status=session.isRunning?1:-5;
                if (owner.status<0) owner.error=@"Camera did not start";
            }
          } @catch (NSException *exception) {
              FerroCameraFail(owner,-5,exception.reason ?: @"Native camera configuration failed");
          }
        }
    });
}
extern "C" {
int ferroui_portable_camera_permission() {
    if (@available(macOS 10.14,iOS 7.0,*)) {} else {return 0;}
    switch ([AVCaptureDevice authorizationStatusForMediaType:AVMediaTypeVideo]) {
        case AVAuthorizationStatusAuthorized:return 1;
        case AVAuthorizationStatusDenied:return 2;
        case AVAuthorizationStatusRestricted:return 3;
        default:return 0;
    }
}
char *ferroui_portable_camera_devices() {
    @autoreleasepool {
        @try {NSMutableArray *out=[NSMutableArray array];for (AVCaptureDevice *device in FerroCameraDevices()) [out addObject:FerroCameraDeviceInfo(device)];return FerroCameraJSON(out);}
        @catch (NSException *exception) {(void)exception;return nullptr;}
    }
}
void ferroui_portable_camera_free(void *p) {free(p);}
void *ferroui_portable_camera_start(const char *id,int facing,uint32_t width,uint32_t height,double fps,int exact) {
    @autoreleasepool {
        if (![NSThread isMainThread]) return nullptr;
        FerroPortableCamera *owner=[FerroPortableCamera new];
        owner.queue=dispatch_queue_create("org.ferroui.camera",DISPATCH_QUEUE_SERIAL);
        if (@available(macOS 10.15,iOS 11.0,*)) {} else {
            FerroCameraFail(owner,-4,@"This camera profile requires macOS 10.15 or iOS 11");
            return (__bridge_retained void *)owner;
        }
        NSString *deviceID=[NSString stringWithUTF8String:id] ?: @"";
        if (![[NSBundle mainBundle] objectForInfoDictionaryKey:@"NSCameraUsageDescription"]) {FerroCameraFail(owner,-4,@"Application must declare NSCameraUsageDescription before camera start");}
        else {
            int permission=ferroui_portable_camera_permission();
            if (permission==1) FerroCameraConfigure(owner,deviceID,facing,width,height,fps,exact!=0);
            else if (permission==0) {
                __weak FerroPortableCamera *weak=owner;
                [AVCaptureDevice requestAccessForMediaType:AVMediaTypeVideo completionHandler:^(BOOL granted){
                    FerroPortableCamera *live=weak;
                    if (!live) return;
                    if (granted) FerroCameraConfigure(live,deviceID,facing,width,height,fps,exact!=0);
                    else FerroCameraFail(live,-1,@"Camera permission denied");
                }];
            } else FerroCameraFail(owner,permission==3?-7:-1,@"Camera access denied or restricted");
        }
        return (__bridge_retained void *)owner;
    }
}
void ferroui_portable_camera_stop(void *p) {[(__bridge FerroPortableCamera *)p close];}
void ferroui_portable_camera_release(void *p) {FerroPortableCamera *owner=(__bridge_transfer FerroPortableCamera *)p;[owner close];}
int ferroui_portable_camera_status(void *p,char *error,size_t capacity) {
    FerroPortableCamera *owner=(__bridge FerroPortableCamera *)p;
    @synchronized(owner) {if(capacity)std::snprintf(error,capacity,"%s",owner.error.UTF8String ?: "Camera failure");return owner.status;}
}
char *ferroui_portable_camera_settings(void *p) {FerroPortableCamera *owner=(__bridge FerroPortableCamera *)p;@synchronized(owner){return owner.settings?FerroCameraJSON(owner.settings):nullptr;}}
void *ferroui_portable_camera_frame(void *p,double *timestamp) {
    FerroPortableCamera *owner=(__bridge FerroPortableCamera *)p;
    @synchronized(owner) {CVPixelBufferRef buffer=owner.latest;owner.latest=nullptr;*timestamp=owner.timestamp;return buffer;}
}
void *ferroui_portable_camera_photo(void *p) {
    FerroPortableCamera *owner=(__bridge FerroPortableCamera *)p;
    @synchronized(owner) {
        if (owner.stopped || owner.status!=1 || !owner.photo) return nullptr;
        bool expected=false;if(!FerroPortablePhotoBusy.compare_exchange_strong(expected,true))return nullptr;
        FerroPortablePhoto *request=[FerroPortablePhoto new];request.keepAlive=request;request.output=owner.photo;request.owner=owner;
        dispatch_async(owner.queue, ^{
            @synchronized(owner) {
                if (owner.stopped || owner.status!=1) {
                    @synchronized(request){request.status=-2;request.keepAlive=nil;request.output=nil;}
                    FerroPortablePhotoBusy.store(false);return;
                }
            }
            AVCapturePhotoSettings *settings=[AVCapturePhotoSettings photoSettingsWithFormat:@{AVVideoCodecKey:AVVideoCodecTypeJPEG}];
            @try {[request.output capturePhotoWithSettings:settings delegate:request];}
            @catch (NSException *exception) {
                @synchronized(request){request.status=-1;request.error=exception.reason;request.keepAlive=nil;request.output=nil;}
                FerroPortablePhotoBusy.store(false);
            }
        });
        return (__bridge_retained void *)request;
    }
}
struct FerroPortablePhotoInfo {uint64_t length;uint32_t width,height,orientation;double timestamp;};
int ferroui_portable_photo_state(void *p,FerroPortablePhotoInfo *info) {
    FerroPortablePhoto *request=(__bridge FerroPortablePhoto *)p;
    FerroPortableCamera *owner=request.owner;
    if (!owner) return -2;
    @synchronized(owner){if(owner.stopped || owner.status<0)return -2;}
    @synchronized(request){info->length=request.data.length;info->width=request.width;info->height=request.height;info->orientation=request.orientation;info->timestamp=request.timestamp;return request.status;}
}
int ferroui_portable_photo_copy(void *p,uint8_t *bytes,size_t capacity) {
    FerroPortablePhoto *request=(__bridge FerroPortablePhoto *)p;
    @synchronized(request){if(request.status!=1 || request.data.length>capacity)return 0;memcpy(bytes,request.data.bytes,request.data.length);return 1;}
}
void ferroui_portable_photo_release(void *p) {
    FerroPortablePhoto *request=(__bridge_transfer FerroPortablePhoto *)p;
    @synchronized(request){request.status=-2;request.data=nil;}
}
}

#ifdef FERROUI_CAMERA_LIFETIME_TESTS
// Deliberately retaining fake output. No device/input is created: this checks
// native delegate/session retirement, not physical capture functionality.
@interface FerroCameraTestOutput : NSObject
@property(nonatomic,strong) id retainedTarget;
- (void)setSampleBufferDelegate:(id<AVCaptureVideoDataOutputSampleBufferDelegate>)delegate queue:(dispatch_queue_t)queue;
@end
@implementation FerroCameraTestOutput
- (void)setSampleBufferDelegate:(id<AVCaptureVideoDataOutputSampleBufferDelegate>)delegate queue:(dispatch_queue_t)queue {
    (void)queue;_retainedTarget=delegate;
}
@end
@interface FerroCameraTestSession : NSObject
@property(nonatomic,strong) NSMutableArray<AVCaptureOutput *> *testOutputs;
@end
@implementation FerroCameraTestSession
- (void)stopRunning {FerroCameraTestStops.fetch_add(1);}
- (void)beginConfiguration {}
- (void)commitConfiguration {}
- (NSArray<AVCaptureInput *> *)inputs {return @[];}
- (NSArray<AVCaptureOutput *> *)outputs {return _testOutputs;}
- (void)removeOutput:(AVCaptureOutput *)output {[_testOutputs removeObject:output];}
@end
extern "C" void *ferroui_camera_lifetime_fixture() {
    FerroCameraTestDrops.store(0);FerroCameraTestStops.store(0);
    FerroPortableCamera *owner=[FerroPortableCamera new];
    owner.queue=dispatch_queue_create("org.ferroui.camera.lifetime-test",DISPATCH_QUEUE_SERIAL);
    FerroCameraTestSession *session=[FerroCameraTestSession new];
    FerroCameraTestOutput *output=[FerroCameraTestOutput new];
    [output setSampleBufferDelegate:owner queue:owner.queue];
    session.testOutputs=[NSMutableArray arrayWithObject:(AVCaptureOutput *)output];
    owner.session=(AVCaptureSession *)session;owner.video=(AVCaptureVideoDataOutput *)output;
    return (__bridge_retained void *)owner;
}
extern "C" int ferroui_camera_lifetime_result() {
    return FerroCameraTestDrops.load()==1 && FerroCameraTestStops.load()==1;
}
#endif
