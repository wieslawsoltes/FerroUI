#import <AppKit/AppKit.h>
#import <Metal/Metal.h>
#import <QuartzCore/QuartzCore.h>
#include "common.h"
#include "rendertarget.h"
#import "crapium.h"


class API_AVAILABLE(macos(12.0)) FrnMTLSharedEvent : public ComSingleObject<IFrnMTLSharedEvent, &IID_IFrnMTLSharedEvent>
{
    id<MTLSharedEvent> _event;
public:
    
    FrnMTLSharedEvent(id<MTLSharedEvent> ev) : _event(ev)
    {
        
    }
    
    FORWARD_IUNKNOWN()
    
    id<MTLSharedEvent> GetEvent()
    {
        return _event;
    }
    
    void *GetNativeHandle() override {
        return (__bridge void*)_event;
    }
    
    bool Wait(uint64_t value, uint64_t timeoutMS) override {
        return MtlSharedEventWaitUntilSignaledValueHack(_event, value, timeoutMS);
    }
    
    void SetSignaledValue(uint64_t value) override {
        _event.signaledValue = value;
    }
    
    uint64_t GetSignaledValue() override {
        return _event.signaledValue;
    }
};


class FrnMetalTexture : public ComSingleObject<IFrnMetalTexture, &IID_IFrnMetalTexture>
{
    id<MTLTexture> _texture;
public:
    FORWARD_IUNKNOWN()
    FrnMetalTexture(id<MTLTexture> texture) : _texture(texture)
    {
        
    }
    void *GetNativeHandle() override
    {
        return (__bridge void*)_texture;
    }
    
    int GetWidth() override
    {
        return (int)_texture.width;
    }
    
    int GetHeight() override
    {
        return (int)_texture.height;
    }
    
    int GetSampleCount() override
    {
        return (int)_texture.sampleCount;
    }
    
};

class FrnMetalDevice : public ComSingleObject<IFrnMetalDevice, &IID_IFrnMetalDevice>
{
public:
    id<MTLDevice> device;
    id<MTLCommandQueue> queue;
    FORWARD_IUNKNOWN()

    void *GetDevice() override {
        return (__bridge void*) device;
    }

    void *GetQueue() override {
        return (__bridge void*) queue;
    }
    
    HRESULT ImportIOSurface(void *handle, FrnPixelFormat pixelFormat, ComPtr<IFrnMetalTexture>& ppv) override {
        auto surf = (IOSurfaceRef)handle;
        auto width = IOSurfaceGetWidth(surf);
        auto height = IOSurfaceGetHeight(surf);

        auto desc = [MTLTextureDescriptor new];
        if(pixelFormat == kFrnRgba8888)
            desc.pixelFormat = MTLPixelFormatRGBA8Unorm;
        else if(pixelFormat == kFrnBgra8888)
            desc.pixelFormat = MTLPixelFormatBGRA8Unorm;
        else
            return E_INVALIDARG;
        desc.textureType = MTLTextureType2D;
        desc.width = width;
        desc.height = height;
        desc.depth = 1;
        desc.mipmapLevelCount = 1;
        desc.sampleCount = 1;
        desc.usage = MTLTextureUsageShaderRead | MTLTextureUsageRenderTarget;

        auto texture = [device newTextureWithDescriptor:desc iosurface:surf plane:0];
        if(texture == nullptr)
            return E_FAIL;
        ppv = comnew<FrnMetalTexture>(texture);
        return S_OK;
    }
    
    HRESULT ImportSharedEvent(void *mtlSharedEventInstance, ComPtr<IFrnMTLSharedEvent>& ppv) override {
        if (@available(macOS 12.0, *)) {
            auto external = (__bridge id<MTLSharedEvent>)mtlSharedEventInstance;
            auto handle = external.newSharedEventHandle;
            auto imported = [device newSharedEventWithHandle: handle];
            ppv = comnew<FrnMTLSharedEvent>(imported);
            return S_OK;
        } 
        else
        {
            return E_NOTIMPL;
        }
    }
    
    
    HRESULT SignalOrWait(const ComPtr<IFrnMTLSharedEvent>& ev, uint64_t value, bool wait)
    {
        if (@available(macOS 12.0, *))
        {
            auto e = ev.dynamicCast<FrnMTLSharedEvent>();
            if(e == nullptr)
                return E_FAIL;
            auto buf = [queue commandBuffer];
            if(wait)
                [buf encodeWaitForEvent:e->GetEvent() value:value];
            else
                [buf encodeSignalEvent:e->GetEvent() value:value];
            [buf commit];
            return S_OK;
        }
        else
            return E_FAIL;
    }
    
    HRESULT SubmitWait(const ComPtr<IFrnMTLSharedEvent>& ev, uint64_t value) override {
        return SignalOrWait(ev, value, true);
    }
    
    HRESULT SubmitSignal(const ComPtr<IFrnMTLSharedEvent>& ev, uint64_t value) override {
        return SignalOrWait(ev, value, false);
    }
    
    bool GetIOKitRegistryId(uint64_t *value) override { 
        if (@available(macOS 10.13, *)) {
            *value = [device registryID];
            return true;
        } else {
            return false;
        }
    }
    
    FrnMetalDevice(id <MTLDevice> device, id <MTLCommandQueue> queue) : device(device), queue(queue) {
    }

};


class FrnMetalRenderSession : public ComSingleObject<IFrnMetalRenderingSession, &IID_IFrnMetalRenderingSession>
{
    id<CAMetalDrawable> _drawable;
    id<MTLCommandQueue> _queue;
    id<MTLTexture> _texture;
    CAMetalLayer* _layer;
    FrnPixelSize _size;
    double _scaling;
    bool _presentWithTransaction;
public:
    FORWARD_IUNKNOWN()

    FrnMetalRenderSession(const ComPtr<FrnMetalDevice>& device, CAMetalLayer* layer, id <CAMetalDrawable> drawable, const FrnPixelSize &size, double scaling, bool presentWithTransaction)
            : _drawable(drawable), _size(size), _scaling(scaling), _queue(device->queue),
            _texture([drawable texture]), _presentWithTransaction(presentWithTransaction) {
        _layer = layer;
    }

    HRESULT GetPixelSize(FrnPixelSize *ret) override {
        *ret = _size;
        return 0;
    }

    double GetScaling() override {
        return _scaling;
    }

    void *GetTexture() override {
        return (__bridge void*) _texture;
    }

    ~FrnMetalRenderSession()
    {
        START_ARP_CALL;
        auto buffer = [_queue commandBuffer];
        if(_presentWithTransaction)
        {
            [buffer commit];
            [buffer waitUntilScheduled];
            [_drawable present];
            // Restore the default asynchronous presentation for the off-thread render loop.
            _layer.presentsWithTransaction = NO;
        }
        else
        {
            [buffer presentDrawable: _drawable];
            [buffer commit];
        }
    }
};

class FrnMetalRenderTarget : public ComSingleObject<IFrnMetalRenderTarget, &IID_IFrnMetalRenderTarget>
{
    CAMetalLayer* _layer;
    double _scaling = 1;
    FrnPixelSize _size = {1,1};
    ComPtr<FrnMetalDevice> _device;
public:
    double PendingScaling = 1;
    FrnPixelSize PendingSize = {1,1};
    // The frames begun on the main thread: read and written by that thread alone.
    uint64_t MainThreadFrames = 0;
    FORWARD_IUNKNOWN()
    FrnMetalRenderTarget(CAMetalLayer* layer, const ComPtr<FrnMetalDevice>& device)
    {
        _layer = layer;
        _device = device;
    }

    HRESULT BeginDrawing(ComPtr<IFrnMetalRenderingSession>& ret) override {
        bool onMainThread = [NSThread isMainThread];
        if(onMainThread)
        {
            // Flush all existing rendering
            auto buffer = [_device->queue commandBuffer];
            [buffer commit];
            [buffer waitUntilCompleted];
            _size = PendingSize;
            _scaling= PendingScaling;
            CGSize layerSize = {(CGFloat)_size.Width, (CGFloat)_size.Height};

            [CATransaction begin];
            [CATransaction setDisableActions:YES];
            [_layer setDrawableSize: layerSize];
            _layer.presentsWithTransaction = YES;
            [CATransaction commit];
        }
        auto drawable = [_layer nextDrawable];
        if(drawable == nil)
        {
            if(onMainThread)
                _layer.presentsWithTransaction = NO;
            ret = nullptr;
            return E_FAIL;
        }
        if(onMainThread)
            MainThreadFrames++;
        ret = comnew<FrnMetalRenderSession>(_device, _layer, drawable, _size, _scaling, onMainThread);
        return 0;
    }
};

@implementation MetalRenderTarget
{
    ComPtr<FrnMetalDevice> _device;
    CAMetalLayer* _layer;
    ComPtr<FrnMetalRenderTarget> _target;
}
- (MetalRenderTarget *)initWithDevice:(const ComPtr<IFrnMetalDevice>&)device {
    _device = device.dynamicCast<FrnMetalDevice>();
    if(!_device)
        return nil;
    _layer = [CAMetalLayer new];
    // Skia's untagged SDR render targets contain sRGB values. Leaving the
    // layer untagged makes WindowServer interpret them in the monitor's
    // device profile, so native presentation differs from offscreen pixels.
    CGColorSpaceRef colorSpace = CGColorSpaceCreateWithName(kCGColorSpaceSRGB);
    _layer.colorspace = colorSpace;
    CGColorSpaceRelease(colorSpace);
    _layer.opaque = false;
    _layer.device = _device->device;
    _target = comnew<FrnMetalRenderTarget>(_layer, _device);
    return self;
}


-(ComPtr<IFrnMetalRenderTarget>) createRenderTarget
{
    return _target;
}

- (void)resize:(FrnPixelSize)size withScale:(float)scale {
    CGSize layerSize = {(CGFloat)size.Width, (CGFloat)size.Height};
    _target->PendingScaling = scale;
    _target->PendingSize = size;
}

- (void)setNeedsDisplay {
    [_layer setNeedsDisplay];
}

- (uint64_t)mainThreadFrames {
    return _target->MainThreadFrames;
}

- (CALayer *)layer {
    return _layer;
}
@end


class FrnMetalDisplay : public ComSingleObject<IFrnMetalDisplay, &IID_IFrnMetalDisplay>
{
public:
    FORWARD_IUNKNOWN()
    HRESULT CreateDevice(ComPtr<IFrnMetalDevice>& ret) override {
        auto device = MTLCreateSystemDefaultDevice();
        if(device == nil) {
            ret = nullptr;
            return E_FAIL;
        }
        auto queue = [device newCommandQueue];
        ret = comnew<FrnMetalDevice>(device, queue);
        return S_OK;
    }
};

static ComStaticPtr<FrnMetalDisplay> _display(comnew<FrnMetalDisplay>());

extern ComPtr<IFrnMetalDisplay> GetMetalDisplay()
{
    return _display.get();
}


extern ComPtr<IFrnMTLSharedEvent> ImportMTLSharedEvent(void* object)
{
    if (@available(macOS 12.0, *)) {
    if(object == nullptr)
        return nullptr;
    auto evId = (__bridge id<MTLSharedEvent>)object;
    
    if(evId == nil)
        return nullptr;
    
    
    return comnew<FrnMTLSharedEvent>(evId);
    } 
    else
    {
        return nullptr;
    }
}
