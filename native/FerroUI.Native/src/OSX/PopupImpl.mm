//
// Created by Dan Walmsley on 06/05/2022.
//

#include "WindowInterfaces.h"
#include "FrnView.h"
#include "WindowImpl.h"
#include "automation.h"
#include "menu.h"
#include "common.h"
#import "WindowBaseImpl.h"
#import "WindowProtocol.h"
#import <AppKit/AppKit.h>

class PopupImpl : public virtual WindowBaseImpl, public IFrnPopup
{
private:
    BEGIN_INTERFACE_MAP()
    INHERIT_INTERFACE_MAP(WindowBaseImpl)
    INTERFACE_MAP_ENTRY(IFrnPopup, IID_IFrnPopup)
    END_INTERFACE_MAP()
    virtual ~PopupImpl(){}
    ComPtr<IFrnWindowEvents> WindowEvents;
    PopupImpl(IFrnWindowEvents* events) : TopLevelImpl(events), WindowBaseImpl(events)
    {
        WindowEvents = events;
        UpdateWindowLevel();
    }
protected:
    virtual NSWindowLevel GetBaseWindowLevel() override
    {
        return NSPopUpMenuWindowLevel;
    }

    virtual NSWindowStyleMask CalculateStyleMask() override
    {
        return NSWindowStyleMaskBorderless;
    }

public:
    virtual HRESULT Show(bool activate, bool isDialog) override
    {
        auto windowProtocol = GetWindowProtocol();
        
        [windowProtocol setEnabled:true];
        
        return WindowBaseImpl::Show(activate, true);
    }
    
    virtual HRESULT SetHitTestVisible(bool value) override
    {
        START_COM_CALL;

        @autoreleasepool
        {
            [Window setIgnoresMouseEvents:!value];
            return S_OK;
        }
    }

    virtual bool ShouldTakeFocusOnShow() override
    {
        auto parent = Parent.tryGet();
        // Don't steal the focus from another windows if our parent is inactive
        if (parent != nullptr && parent->Window != nullptr && ![parent->Window isKeyWindow])
            return false;

        return WindowBaseImpl::ShouldTakeFocusOnShow();
    }
};


extern IFrnPopup* CreateFrnPopup(IFrnWindowEvents*events)
{
    @autoreleasepool
    {
        IFrnPopup* ptr = dynamic_cast<IFrnPopup*>(new PopupImpl(events));
        return ptr;
    }
}
