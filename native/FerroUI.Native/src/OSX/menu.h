//
//  menu.h
//  FerroUI.Native.OSX
//

#ifndef menu_h
#define menu_h

#include "common.h"

class FrnAppMenuItem;
class FrnAppMenu;

@interface FrnMenu : NSMenu
- (id) initWithDelegate: (NSObject<NSMenuDelegate>*) del;
- (void) setHasGlobalMenuItem: (bool) value;
- (bool) hasGlobalMenuItem;
@end

@interface FrnMenuItem : NSMenuItem
- (id) initWithFrnAppMenuItem: (const ComPtr<FrnAppMenuItem>&)menuItem;
- (void)didSelectItem:(id)sender;
@end

class FrnAppMenuItem : public ComSingleObject<IFrnMenuItem, &IID_IFrnMenuItem>
{
private:
    NSMenuItem* _native; // here we hold a pointer to an FrnMenuItem
    ComPtr<IFrnActionCallback> _callback;
    ComPtr<IFrnPredicateCallback> _predicate;
    bool _isCheckable;
    
public:
    FORWARD_IUNKNOWN()
    
    FrnAppMenuItem(bool isSeparator);
    
    NSMenuItem* GetNative();
    
    virtual HRESULT SetSubMenu (const ComPtr<IFrnMenu>& menu) override;
    
    virtual HRESULT SetTitle (char* utf8String) override;

    virtual HRESULT SetToolTip (char* utf8String) override;

    virtual HRESULT SetGesture (FrnKey key, FrnInputModifiers modifiers) override;
    
    virtual HRESULT SetAction (const ComPtr<IFrnPredicateCallback>& predicate, const ComPtr<IFrnActionCallback>& callback) override;
    
    virtual HRESULT SetIsChecked (bool isChecked) override;

    virtual HRESULT SetIsVisible (bool isVisible) override;
        
    virtual HRESULT SetToggleType (FrnMenuItemToggleType toggleType) override;
    
    virtual HRESULT SetIcon (void* data, size_t length) override;
    
    bool EvaluateItemEnabled();
    
    void RaiseOnClicked();
};

class FrnAppMenu;

@interface FrnMenuDelegate : NSObject<NSMenuDelegate>
- (id) initWithParent: (const ComPtr<FrnAppMenu>&) parent;
@end


class FrnAppMenu : public ComSingleObject<IFrnMenu, &IID_IFrnMenu>
{
private:
    FrnMenu* _native;
    ComPtr<IFrnMenuEvents> _baseEvents;
    FrnMenuDelegate* _delegate;
    
public:
    FORWARD_IUNKNOWN()
    
    FrnAppMenu(const ComPtr<IFrnMenuEvents>& events);

    FrnMenu* GetNative();
    
    void RaiseNeedsUpdate ();
    void RaiseOpening();
    void RaiseClosed();
    
    virtual HRESULT InsertItem (int index, const ComPtr<IFrnMenuItem>& item) override;
    
    virtual HRESULT RemoveItem (const ComPtr<IFrnMenuItem>& item) override;
    
    virtual HRESULT SetTitle (char* utf8String) override;
    
    virtual HRESULT Clear () override;
};



#endif

