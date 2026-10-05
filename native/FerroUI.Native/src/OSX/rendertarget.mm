#include "common.h"
#include "rendertarget.h"
#import <IOSurface/IOSurfaceObjC.h>
#import <QuartzCore/QuartzCore.h>

#include <OpenGL/glext.h>
#include <OpenGL/gl3.h>
#include <queue>

@implementation IOSurfaceHolder : NSObject
{
    @public IOSurfaceRef surface;
    @public FrnPixelSize size;
    @public float scale;
    ComPtr<IFrnGlContext> _context;
    GLuint _framebuffer, _texture, _renderbuffer;
}

- (IOSurfaceHolder*) initWithSize: (FrnPixelSize) size
                        withScale: (float)scale
                withOpenGlContext: (IFrnGlContext*) context
{
    long bytesPerRow = IOSurfaceAlignProperty(kIOSurfaceBytesPerRow, size.Width * 4);
    long allocSize = IOSurfaceAlignProperty(kIOSurfaceAllocSize, size.Height * bytesPerRow);
    NSDictionary* options = @{
                              (id)kIOSurfaceWidth: @(size.Width),
                              (id)kIOSurfaceHeight:  @(size.Height),
                              (id)kIOSurfacePixelFormat: @((uint)'BGRA'),
                              (id)kIOSurfaceBytesPerElement: @(4),
                              (id)kIOSurfaceBytesPerRow: @(bytesPerRow),
                              (id)kIOSurfaceAllocSize: @(allocSize),
                              
                              //(id)kIOSurfaceCacheMode: @(kIOMapWriteCombineCache),
                              (id)kIOSurfaceElementWidth: @(1),
                              (id)kIOSurfaceElementHeight: @(1)
                              };
    
    surface = IOSurfaceCreate((CFDictionaryRef)options);
    self->scale = scale;
    self->size = size;
    self->_context = context;
    return self;
}

-(HRESULT) prepareForGlRender
{
    if(_context == nil)
        return E_FAIL;
    if(CGLGetCurrentContext() != _context->GetNativeHandle())
        return E_FAIL;
    if(_framebuffer == 0)
        glGenFramebuffersEXT(1, &_framebuffer);
    
    
    glBindFramebufferEXT(GL_FRAMEBUFFER_EXT, _framebuffer);
    if(_texture == 0)
    {
        glGenTextures(1, &_texture);
    
        glBindTexture(GL_TEXTURE_RECTANGLE_EXT, _texture);
        CGLError res = CGLTexImageIOSurface2D((CGLContextObj)_context->GetNativeHandle(),
                               GL_TEXTURE_RECTANGLE_EXT, GL_RGBA8,
                               size.Width, size.Height, GL_BGRA, GL_UNSIGNED_INT_8_8_8_8_REV, surface, 0);
        glBindTexture(GL_TEXTURE_RECTANGLE_EXT, 0);
        
        if(res != 0)
        {
            glDeleteTextures(1, &_texture);
            _texture = 0;
            glBindFramebufferEXT(GL_FRAMEBUFFER_EXT, 0);
            return E_FAIL;
        }
        glFramebufferTexture2DEXT(GL_FRAMEBUFFER_EXT, GL_COLOR_ATTACHMENT0_EXT, GL_TEXTURE_RECTANGLE_EXT, _texture, 0);
    }

    if(_renderbuffer == 0)
    {
        glGenRenderbuffers(1, &_renderbuffer);
        glBindRenderbuffer(GL_RENDERBUFFER, _renderbuffer);
        glRenderbufferStorage(GL_RENDERBUFFER, GL_DEPTH_COMPONENT, size.Width, size.Height);
        glFramebufferRenderbuffer(GL_FRAMEBUFFER, GL_DEPTH_ATTACHMENT, GL_RENDERBUFFER, _renderbuffer);
    }

    return S_OK;
}

-(void) finishDraw
{
    glBindFramebufferEXT(GL_FRAMEBUFFER_EXT, 0);
    if(_framebuffer != 0) {
        glDeleteFramebuffers(1, &_framebuffer);
        _framebuffer = 0;
    }
    if(_texture != 0) {
        glDeleteTextures(1, &_texture);
        _texture = 0;
    }
    if(_renderbuffer != 0) {
        glDeleteRenderbuffers(1, &_renderbuffer);
        _renderbuffer = 0;
    }
    glFlush();
}

-(void) dealloc
{
    if(surface != nullptr)
    {
        CFRelease(surface);
        surface = nil;
    }
}
@end


static IFrnGlSurfaceRenderTarget* CreateGlRenderTarget(IOSurfaceRenderTarget* target);
static IFrnSoftwareRenderTarget* CreateSoftwareRenderTarget(IOSurfaceRenderTarget* target);

static bool SizeEquals(FrnPixelSize& left, FrnPixelSize& right)
{
    return left.Width == right.Width && right.Height == left.Height;
}

class ConsumeSurfacesCallback : public ComSingleObject<IFrnActionCallback, &IID_IFrnActionCallback>
{
    IOSurfaceRenderTarget* _target;
public:
    FORWARD_IUNKNOWN()
    ConsumeSurfacesCallback(IOSurfaceRenderTarget* target)
    {
        _target = target;
    }

    void Run() override {
        [_target consumeSurfaces];
    }
};

@implementation IOSurfaceRenderTarget
{
    CALayer* _layer;
    @public NSObject* lock;
    ComPtr<IFrnGlContext> _glContext;
    bool _consumeSurfacesScheduled;
    std::queue<ObjCWrapper<IOSurfaceHolder>> _surfaces;
    IOSurfaceHolder* _activeSurface;
    FrnPixelSize _size;
    float _scale;
}

- (IOSurfaceRenderTarget*) initWithOpenGlContext: (IFrnGlContext*) context;
{
    self = [super init];
    _glContext = context;
    lock = [NSObject new];
    _layer = [CALayer new];
    [self resize:{1,1} withScale: 1];
    
    return self;
}

- (CALayer *)layer {
    return _layer;
}

- (void)resize:(FrnPixelSize)size withScale: (float) scale{

    if(size.Height <= 0)
        size.Height = 1;
    if(size.Width <= 0)
        size.Width = 1;

    @synchronized (lock) {
        _size = size;
        _scale = scale;
    }
}

- (void)setSurfaceInUiThreadContext: (IOSurfaceHolder*) surface
{
    [CATransaction begin];
    if(surface == _activeSurface)
        [_layer setContents: nil];
    _activeSurface = surface;
    [_layer setContentsScale: _activeSurface->scale];
    [_layer setContents: (__bridge IOSurface*) _activeSurface->surface];
    [CATransaction commit];
}

- (void)consumeSurfaces {
    @synchronized (lock) {
        _consumeSurfacesScheduled = false;

        while(_surfaces.size() > 1)
            _surfaces.pop();

        if(_surfaces.size() == 0)
            return;

        auto targetSurface = _surfaces.front();
        _surfaces.pop();

        [self setSurfaceInUiThreadContext: targetSurface];
    }
    // This can trigger event processing on the main thread
    // which might need to lock the renderer
    // which can cause a deadlock. So flush call is outside of the lock
    [CATransaction flush];

}

- (IOSurfaceHolder*) getNextSurfaceInSafeContext
{
    IOSurfaceHolder* targetSurface = nil;
    if([NSThread isMainThread])
    {
        // Drain the surface queue and try to find a surface usable for rendering
        while(_surfaces.size() > 0)
        {
            auto front = _surfaces.front();
            _surfaces.pop();
            if(targetSurface == nil && SizeEquals(front.Value->size, _size))
            {
                targetSurface = front;
            }
        }
        if(targetSurface == nil && _activeSurface != nil && SizeEquals(_activeSurface->size, _size))
            targetSurface = _activeSurface;
    }
    else
    {
        // Try to reuse an outdated surface that is still not picked up by the UI thread
        while(_surfaces.size() > 1)
        {
            auto front = _surfaces.front();
            _surfaces.pop();

            // Simply discard the surface on size mismatch
            if(SizeEquals(front.Value->size, _size))
                targetSurface = front;
        }
    }

    if(targetSurface == nil)
        targetSurface = [[IOSurfaceHolder alloc] initWithSize: _size withScale: _scale withOpenGlContext: _glContext];
    return targetSurface;
}

- (void) presentSurfaceInSafeContext: (IOSurfaceHolder*) surface
{
    if([NSThread isMainThread])
        [self setSurfaceInUiThreadContext: surface];
    else
    {
        _surfaces.push(surface);
        if(_consumeSurfacesScheduled)
            return;
        _consumeSurfacesScheduled = true;
        __block auto strongSelf = self;
        ComPtr<ConsumeSurfacesCallback> cb(new ConsumeSurfacesCallback(self), true);
        PostDispatcherCallback(cb);
    }
}

- (void) presentSurface: (IOSurfaceHolder*) surface
{
    @synchronized(lock)
    {
        [self presentSurfaceInSafeContext: surface];
    }
}

- (HRESULT)setSwFrame:(FrnFramebuffer *)fb {
    @synchronized (lock) {
        if(fb->PixelFormat == FrnPixelFormat::kFrnRgb565)
            return E_INVALIDARG;
        auto surface = [self getNextSurfaceInSafeContext];
        IOSurfaceRef surf = surface->surface;
        if(IOSurfaceLock(surf, 0, nil))
            return E_FAIL;
        size_t w = MIN(fb->Width, IOSurfaceGetWidth(surf));
        size_t h = MIN(fb->Height, IOSurfaceGetHeight(surf));
        size_t wbytes = w*4;
        size_t sstride = IOSurfaceGetBytesPerRow(surf);
        size_t fstride = fb->Stride;
        char*pSurface = (char*)IOSurfaceGetBaseAddress(surf);
        char*pFb = (char*)fb->Data;
        for(size_t y = 0; y < h; y++)
        {
            memcpy(pSurface + y*sstride, pFb + y*fstride, wbytes);
        }
        IOSurfaceUnlock(surf, 0, nil);
        [self presentSurfaceInSafeContext: surface];
        return S_OK;
    }
}

-(IFrnGlSurfaceRenderTarget*) createSurfaceRenderTarget
{
    return CreateGlRenderTarget(self);
}

-(IFrnSoftwareRenderTarget*) createSoftwareRenderTarget
{
    return CreateSoftwareRenderTarget(self);
}

@end

class FrnGlRenderingSession : public ComSingleObject<IFrnGlSurfaceRenderingSession, &IID_IFrnGlSurfaceRenderingSession>
{
    ComPtr<IUnknown> _releaseContext;
    IOSurfaceRenderTarget* _target;
    IOSurfaceHolder* _surface;
public:
    FORWARD_IUNKNOWN()
    FrnGlRenderingSession(IOSurfaceRenderTarget* target, IOSurfaceHolder* surface, ComPtr<IUnknown> releaseContext)
    {
        _target = target;
        // This happens in a synchronized block set up by FrnRenderTarget, so we take the current surface for this
        // particular render session
        _surface = surface;
        _releaseContext = releaseContext;
    }
    
    virtual HRESULT GetPixelSize(FrnPixelSize* ret)  override
    {
        START_COM_CALL;
        
        if(!_surface)
            return E_FAIL;
        *ret = _surface->size;
        return S_OK;
    }
    
    virtual HRESULT GetScaling(double* ret)  override
    {
        START_COM_CALL;
        
        if(!_surface)
            return E_FAIL;
        *ret = _surface->scale;
        return S_OK;
    }
    
    virtual ~FrnGlRenderingSession()
    {
        START_ARP_CALL;
        [_surface finishDraw];
        [_target presentSurface: _surface];
        _releaseContext = nil;
    }
};

class FrnGlRenderTarget : public ComSingleObject<IFrnGlSurfaceRenderTarget, &IID_IFrnGlSurfaceRenderTarget>
{
    IOSurfaceRenderTarget* _target;
public:
    FORWARD_IUNKNOWN()
    FrnGlRenderTarget(IOSurfaceRenderTarget* target)
    {
        _target = target;
    }
    
    virtual HRESULT BeginDrawing(IFrnGlSurfaceRenderingSession** ret)  override
    {
        START_COM_ARP_CALL;
        ComPtr<IUnknown> releaseContext;
        @synchronized (_target->lock) {
            auto surface = [_target getNextSurfaceInSafeContext];
            _target->_glContext->MakeCurrent(releaseContext.getPPV());
            HRESULT res = [surface prepareForGlRender];
            if(res)
                return res;
            *ret = new FrnGlRenderingSession(_target, surface, releaseContext);
            return S_OK;
        }
    }
};


static IFrnGlSurfaceRenderTarget* CreateGlRenderTarget(IOSurfaceRenderTarget* target)
{
    return new FrnGlRenderTarget(target);
};


class FrnSoftwareRenderTarget : public ComSingleObject<IFrnSoftwareRenderTarget, &IID_IFrnSoftwareRenderTarget>
{
    IOSurfaceRenderTarget* _target;
public:
    FORWARD_IUNKNOWN()

    FrnSoftwareRenderTarget(IOSurfaceRenderTarget *target) {
        _target = target;
    }

    HRESULT SetFrame(FrnFramebuffer *fb) override {
        START_COM_ARP_CALL;
        [_target setSwFrame: fb];
        return 0;
    }
};

static IFrnSoftwareRenderTarget* CreateSoftwareRenderTarget(IOSurfaceRenderTarget* target)
{
    return new FrnSoftwareRenderTarget(target);
};
