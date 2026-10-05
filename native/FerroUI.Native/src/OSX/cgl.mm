#include "common.h"
#include <dlfcn.h>

static CGLContextObj CreateCglContext(CGLContextObj share)
{
    int attributes[] = {
        kCGLPFAAccelerated,
        kCGLPFAOpenGLProfile, (CGLPixelFormatAttribute)kCGLOGLPVersion_3_2_Core,
        kCGLPFADepthSize, 8,
        kCGLPFAStencilSize, 8,
        kCGLPFAColorSize, 32,
        0
    };
    
    CGLPixelFormatObj pix;
    CGLError errorCode;
    GLint num; // stores the number of possible pixel formats
    errorCode = CGLChoosePixelFormat( (CGLPixelFormatAttribute*)attributes, &pix, &num );
    if(errorCode != 0)
        return nil;
    CGLContextObj ctx = nil;
    errorCode = CGLCreateContext(pix, share, &ctx );
    CGLDestroyPixelFormat( pix );
    if(errorCode != 0)
        return nil;
    return ctx;
};



class FrnGlContext : public virtual ComSingleObject<IFrnGlContext, &IID_IFrnGlContext>
{
    // Debug
    int _usageCount = 0;
public:
    CGLContextObj Context;
    int SampleCount = 0, StencilBits = 0;
    FORWARD_IUNKNOWN()
    
    class SavedGlContext : public virtual ComUnknownObject
    {
        CGLContextObj _savedContext;
        ComPtr<FrnGlContext> _parent;
    public:
        SavedGlContext(CGLContextObj saved, FrnGlContext* parent)
        {
            _savedContext = saved;
            _parent = parent;
            _parent->_usageCount++;
        }
        
        ~SavedGlContext()
        {
            if(_parent->Context == CGLGetCurrentContext())
                CGLSetCurrentContext(_savedContext);
            _parent->_usageCount--;
            CGLUnlockContext(_parent->Context);
        }
    };
    
    FrnGlContext(CGLContextObj context)
    {
        Context = context;
        CGLPixelFormatObj fmt = CGLGetPixelFormat(context);
        CGLDescribePixelFormat(fmt, 0, kCGLPFASamples, &SampleCount);
        CGLDescribePixelFormat(fmt, 0, kCGLPFAStencilSize, &StencilBits);
        
    }
    
    virtual HRESULT LegacyMakeCurrent() override
    {
        START_COM_CALL;
        
        if(CGLSetCurrentContext(Context) != 0)
            return E_FAIL;
        return S_OK;
    }
    
    virtual HRESULT MakeCurrent(IUnknown** ppv) override
    {
        START_COM_CALL;
        
        CGLContextObj saved = CGLGetCurrentContext();
        CGLLockContext(Context);
        if(CGLSetCurrentContext(Context) != 0)
        {
            CGLUnlockContext(Context);
            return E_FAIL;
        }
        *ppv = new SavedGlContext(saved, this);
        
        return S_OK;
    }
    
    virtual int GetSampleCount() override
    {
        return SampleCount;
    }
    
    virtual int GetStencilSize() override
    {
        return StencilBits;
    }
    
    virtual void* GetNativeHandle() override
    {
        return Context;
    }
    
    int texImageIOSurface2D(int target, int internal_format,
                                int width, int height, int format, int type, void* ioSurface, int plane) override
    {
        return CGLTexImageIOSurface2D(Context, target, internal_format, width, height, format, type, (IOSurfaceRef)ioSurface, plane);
    }
    
    bool GetIOKitRegistryId(uint64_t *value) override {
        if (@available(macOS 10.13, *))
        {
            
            GLint rendererId;
            if(CGLGetParameter(Context, kCGLCPCurrentRendererID, &rendererId) != 0)
                return false;
            
            GLint rendererCount = 0;
            CGLRendererInfoObj rendererInfo;
            
            if(CGLQueryRendererInfo(0xFFFFFFFF, &rendererInfo, &rendererCount))
                return false;
            
            @try
            {
                for(auto i = 0; i < rendererCount; i++)
                {
                    GLint thisRendererID;
                    
                    CGLDescribeRenderer(rendererInfo, i, kCGLRPRendererID, &thisRendererID);
                    if(thisRendererID == rendererId)
                    {
                        GLint gpuIDLow  = 0;
                        GLint gpuIDHigh = 0;
                        
                        if(CGLDescribeRenderer(rendererInfo, 0, kCGLRPRegistryIDLow, &gpuIDLow))
                            return false;
                        
                        if(CGLDescribeRenderer(rendererInfo, 0, kCGLRPRegistryIDHigh, &gpuIDHigh))
                            return false;
                        
                        *value = ((uint64_t)gpuIDHigh << 32) | gpuIDLow;
                        return true;
                    }
                }
                return false;
                
            }
            @finally
            {
                CGLDestroyRendererInfo(rendererInfo);
            }
        }
        else
            return false;
    }
    
    
    ~FrnGlContext()
    {
        CGLReleaseContext(Context);
    }
};

class FrnGlDisplay : public virtual ComSingleObject<IFrnGlDisplay, &IID_IFrnGlDisplay>
{
    void* _libgl;
    
public:
    FORWARD_IUNKNOWN()
    
    FrnGlDisplay()
    {
        _libgl = dlopen("/System/Library/Frameworks/OpenGL.framework/Versions/A/Libraries/libGL.dylib", RTLD_LAZY);
    }
    
    virtual void* GetProcAddress(char* proc)  override
    {
        return dlsym(_libgl, proc);
    }
    
    virtual HRESULT CreateContext(IFrnGlContext* share, IFrnGlContext**ppv) override
    {
        START_COM_CALL;
        
        CGLContextObj shareContext = nil;
        if(share != nil)
        {
            FrnGlContext* shareCtx = dynamic_cast<FrnGlContext*>(share);
            if(shareCtx != nil)
                shareContext = shareCtx->Context;
        }
        CGLContextObj ctx = ::CreateCglContext(shareContext);
        if(ctx == nil)
            return E_FAIL;
        *ppv = new FrnGlContext(ctx);
        return S_OK;
    }
    
    virtual HRESULT WrapContext(void* native, IFrnGlContext**ppv) override
    {
        START_COM_CALL;
        
        if(native == nil)
            return E_INVALIDARG;
        *ppv = new FrnGlContext((CGLContextObj) native);
        return S_OK;
    }
    
    virtual void LegacyClearCurrentContext() override
    {
        CGLSetCurrentContext(nil);
    }
};

static ComStaticPtr<FrnGlDisplay> GlDisplay(comnew<FrnGlDisplay>());


extern IFrnGlDisplay* GetGlDisplay()
{
    return GlDisplay;
};

