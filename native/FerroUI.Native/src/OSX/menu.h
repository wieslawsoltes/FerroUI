//
//  menu.h
//  FerroUI.Native.OSX
//
//  Created by Dan Walmsley on 01/08/2019.
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
- (id) initWithFrnAppMenuItem: (FrnAppMenuItem*)menuItem;
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
    
    virtual HRESULT SetSubMenu (IFrnMenu* menu) override;
    
    virtual HRESULT SetTitle (char* utf8String) override;

    virtual HRESULT SetToolTip (char* utf8String) override;

    virtual HRESULT SetGesture (FrnKey key, FrnInputModifiers modifiers) override;
    
    virtual HRESULT SetAction (IFrnPredicateCallback* predicate, IFrnActionCallback* callback) override;
    
    virtual HRESULT SetIsChecked (bool isChecked) override;

    virtual HRESULT SetIsVisible (bool isVisible) override;
        
    virtual HRESULT SetToggleType (FrnMenuItemToggleType toggleType) override;
    
    virtual HRESULT SetIcon (void* data, size_t length) override;
    
    bool EvaluateItemEnabled();
    
    void RaiseOnClicked();
};

class FrnAppMenu;

@interface FrnMenuDelegate : NSObject<NSMenuDelegate>
- (id) initWithParent: (FrnAppMenu*) parent;
- (void) parentDestroyed;
@end


class FrnAppMenu : public ComSingleObject<IFrnMenu, &IID_IFrnMenu>
{
private:
    FrnMenu* _native;
    ComPtr<IFrnMenuEvents> _baseEvents;
    FrnMenuDelegate* _delegate;
    
public:
    FORWARD_IUNKNOWN()
    
    FrnAppMenu(IFrnMenuEvents* events);

    FrnMenu* GetNative();
    
    void RaiseNeedsUpdate ();
    void RaiseOpening();
    void RaiseClosed();
    
    virtual HRESULT InsertItem (int index, IFrnMenuItem* item) override;
    
    virtual HRESULT RemoveItem (IFrnMenuItem* item) override;
    
    virtual HRESULT SetTitle (char* utf8String) override;
    
    virtual HRESULT Clear () override;
    virtual ~FrnAppMenu() override;
};



#endif

