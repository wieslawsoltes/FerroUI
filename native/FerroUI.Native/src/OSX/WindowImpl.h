//
// Created by Dan Walmsley on 04/05/2022.
//

#ifndef FERRO_NATIVE_OSX_WINDOWIMPL_H
#define FERRO_NATIVE_OSX_WINDOWIMPL_H

#import "WindowBaseImpl.h"
#include "IWindowStateChanged.h"
class WindowImpl : public virtual WindowBaseImpl, public virtual IFrnWindow, public IWindowStateChanged
{
public:
    FORWARD_IUNKNOWN()
BEGIN_INTERFACE_MAP()
        INHERIT_INTERFACE_MAP(WindowBaseImpl)
        INTERFACE_MAP_ENTRY(IFrnWindow, IID_IFrnWindow)
    END_INTERFACE_MAP()
    virtual ~WindowImpl()
    {
    }

    ComPtr<IFrnWindowEvents> WindowEvents;

    WindowImpl(IFrnWindowEvents* events);

    virtual HRESULT Show (bool activate, bool isDialog) override;

    virtual HRESULT SetEnabled (bool enable) override;

    void StartStateTransition () override ;

    void EndStateTransition () override ;

    SystemDecorations Decorations () override ;

    FrnWindowState WindowState () override ;

    void WindowStateChanged () override ;

    bool UndecoratedIsMaximized ();

    bool IsZoomed ();

    void DoZoom();

    virtual HRESULT SetCanResize(bool value) override;

    virtual HRESULT SetCanMinimize(bool value) override;

    virtual HRESULT SetCanMaximize(bool value) override;

    virtual HRESULT SetDecorations(SystemDecorations value) override;

    virtual HRESULT SetTitle (char* utf8title) override;

    virtual HRESULT SetTitleBarColor(FrnColor color) override;

    virtual HRESULT GetWindowState (FrnWindowState*ret) override;

    virtual HRESULT TakeFocusFromChildren () override;

    virtual HRESULT SetExtendClientArea (bool enable) override;

    virtual HRESULT GetExtendTitleBarHeight (double*ret) override;

    virtual HRESULT SetExtendTitleBarHeight (double value) override;

    virtual HRESULT GetWindowZOrder (long* zOrder) override;

    void EnterFullScreenMode ();

    void ExitFullScreenMode ();

    virtual HRESULT SetWindowState (FrnWindowState state) override;

    virtual HRESULT SetWindowState (FrnWindowState state, bool shouldResize);

    virtual bool IsModal() override;

    bool IsOwned();

    virtual void BringToFront () override;

    bool CanBecomeKeyWindow ();

    bool CanZoom() override { return _isEnabled && _canMaximize; }

    bool IsTransitioningWindowState() { return _transitioningWindowState; }

protected:
    virtual NSWindowStyleMask CalculateStyleMask() override;
    virtual void UpdateAppearance() override;

private:
    void ZOrderChildWindows();
    void OnInitialiseNSWindow();
    NSString *_lastTitle;
    bool _isEnabled;
    bool _canResize;
    bool _canMinimize;
    bool _canMaximize;
    bool _fullScreenActive;
    SystemDecorations _decorations;
    FrnWindowState _lastWindowState;
    FrnWindowState _actualWindowState;
    bool _inSetWindowState;
    NSRect _preZoomSize;
    bool _transitioningWindowState;
    bool _isClientAreaExtended;
    bool _isModal;
};

#endif //FERRO_NATIVE_OSX_WINDOWIMPL_H
