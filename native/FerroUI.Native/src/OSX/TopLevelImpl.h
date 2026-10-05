//
//  TopLevelImpl.h
//  FerroUI.Native.OSX
//
//  Created by Benedikt Stebner on 16.05.24.
//

#ifndef TopLevelImpl_h
#define TopLevelImpl_h

#include "rendertarget.h"
#include "INSWindowHolder.h"
#include "FrnTextInputMethod.h"
#include "AutoFitContentView.h"
#include <list>

class TopLevelImpl : public virtual ComObject,
                     public virtual IFrnTopLevel,
                     public INSViewHolder{
    
public:
    FORWARD_IUNKNOWN()
    BEGIN_INTERFACE_MAP()
    INTERFACE_MAP_ENTRY(IFrnTopLevel, IID_IFrnTopLevel)
    END_INTERFACE_MAP()
    
    virtual ~TopLevelImpl();
    
    TopLevelImpl(IFrnTopLevelEvents* events);
                         
    virtual FrnView *GetNSView() override;
                         
    virtual HRESULT SetCursor(IFrnCursor* cursor) override;
                         
    virtual HRESULT GetScaling(double*ret) override;
                         
    virtual HRESULT GetClientSize(FrnSize *ret) override;
                           
    virtual HRESULT GetInputMethod(IFrnTextInputMethod **ppv) override;
                           
    virtual HRESULT ObtainNSViewHandle(void** retOut) override;
                                                  
    virtual HRESULT ObtainNSViewHandleRetained(void** retOut) override;
                           
    virtual HRESULT CreateSoftwareRenderTarget(IFrnSoftwareRenderTarget** ret) override;
                                                  
    virtual HRESULT CreateMetalRenderTarget(IFrnMetalDevice* device, IFrnMetalRenderTarget** ret) override;
                           
    virtual HRESULT CreateGlRenderTarget(IFrnGlContext* context, IFrnGlSurfaceRenderTarget** ret) override;

    virtual HRESULT CreateNativeControlHost(IFrnNativeControlHost **retOut) override;
                         
    virtual HRESULT Invalidate() override;
                         
    virtual HRESULT PointToClient(FrnPoint point, FrnPoint *ret) override;

    virtual HRESULT PointToScreen(FrnPoint point, FrnPoint *ret) override;
     
    virtual HRESULT SetTransparencyMode(FrnWindowTransparencyMode mode) override;

    virtual HRESULT GetCurrentDisplayId (CGDirectDisplayID* ret) override;

    virtual HRESULT BeginDragAndDropOperation(
        FrnDragDropEffects effects,
        FrnPoint point,
        IFrnClipboardDataSource* source,
        IFrnDndResultCallback* callback,
        void* sourceHandle) override;

protected:
    NSCursor *cursor;
    virtual void UpdateAppearance();
                           
public:
    NSObject<IRenderTarget> *currentRenderTarget;
    ComPtr<FrnTextInputMethod> InputMethod;
    ComPtr<IFrnTopLevelEvents> TopLevelEvents;
    FrnView *View;
                         
    void UpdateCursor();
    virtual void SetClientSize(NSSize size);
};

#endif /* TopLevelImpl_h */
