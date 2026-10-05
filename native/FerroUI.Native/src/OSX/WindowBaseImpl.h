//
// Created by Dan Walmsley on 04/05/2022.
//

#ifndef FERRO_NATIVE_OSX_WINDOWBASEIMPL_H
#define FERRO_NATIVE_OSX_WINDOWBASEIMPL_H

#include "rendertarget.h"
#include "INSWindowHolder.h"
#include "FrnTextInputMethod.h"
#include "TopLevelImpl.h"
#include <list>

@class FrnMenu;
@protocol FrnWindowProtocol;

class WindowBaseImpl : public virtual TopLevelImpl,
                       public virtual IFrnWindowBase,
                       public INSWindowHolder {

public:
    FORWARD_IUNKNOWN()

    BEGIN_INTERFACE_MAP()
        INHERIT_INTERFACE_MAP(TopLevelImpl)
        INTERFACE_MAP_ENTRY(IFrnWindowBase, IID_IFrnWindowBase)
    END_INTERFACE_MAP()

    virtual ~WindowBaseImpl();

    WindowBaseImpl(IFrnWindowBaseEvents *events, bool usePanel = false);

    virtual HRESULT ObtainNSWindowHandle(void **ret) override;

    virtual HRESULT ObtainNSWindowHandleRetained(void **ret) override;

    virtual NSWindow *GetNSWindow() override;

    virtual HRESULT Show(bool activate, bool isDialog) override;

    virtual bool IsShown ();

    virtual bool ShouldTakeFocusOnShow();

    virtual HRESULT Hide() override;

    virtual HRESULT Activate() override;

    virtual HRESULT SetTopMost(bool value) override;

    virtual HRESULT Close() override;

    virtual HRESULT GetFrameSize(FrnSize *ret) override;

    virtual HRESULT SetMinMaxSize(FrnSize minSize, FrnSize maxSize) override;

    virtual HRESULT Resize(double x, double y, FrnPlatformResizeReason reason) override;

    virtual HRESULT SetMainMenu(IFrnMenu *menu) override;

    virtual HRESULT BeginMoveDrag() override;

    virtual HRESULT BeginResizeDrag(__attribute__((unused)) FrnWindowEdge edge) override;

    virtual HRESULT GetPosition(FrnPoint *ret) override;

    virtual HRESULT SetPosition(FrnPoint point) override;

    virtual HRESULT SetFrameThemeVariant(FrnPlatformThemeVariant variant) override;

    virtual HRESULT SetTransparencyMode(FrnWindowTransparencyMode mode) override;
                           
    virtual bool IsModal();

    id<FrnWindowProtocol> GetWindowProtocol ();
                           
    virtual void BringToFront ();

    virtual bool CanZoom() { return false; }
                           
    virtual HRESULT SetParent(IFrnWindowBase* parent) override;

    void UpdateWindowLevel();

protected:
    virtual NSWindowLevel GetBaseWindowLevel();

    virtual NSWindowStyleMask CalculateStyleMask() = 0;
    virtual void UpdateAppearance() override;
    virtual void SetClientSize(NSSize size) override;

private:
    void CreateNSWindow (bool isDialog);
    void CleanNSWindow ();

    bool hasPosition;
    NSSize lastSize;
    NSSize lastMinSize;
    NSSize lastMaxSize;
    FrnMenu* lastMenu;
    bool _inResize;

protected:
    AutoFitContentView *StandardContainer;
    FrnPoint lastPositionSet;
    bool _shown;
    bool _isTopmost = false;
    std::list<ComObjectWeakPtr<WindowBaseImpl>> _children;

public:
    ComObjectWeakPtr<WindowBaseImpl> Parent = nullptr;
    NSWindow * Window;
    ComPtr<IFrnWindowBaseEvents> BaseEvents;
};

#endif //FERRO_NATIVE_OSX_WINDOWBASEIMPL_H
