

#include "common.h"
#include "menu.h"
#include "KeyTransform.h"
#include <CoreFoundation/CoreFoundation.h>
#include <Carbon/Carbon.h> /* For kVK_ constants, and TIS functions. */

@implementation FrnMenu
{
    bool _isReparented;
    NSObject<NSMenuDelegate>* _wtf;
}

- (id) initWithDelegate: (NSObject<NSMenuDelegate>*)del
{
    self = [super init];
    self.delegate = del;
    _wtf = del;
    _isReparented = false;
    return self;
}

- (bool)hasGlobalMenuItem
{
    return _isReparented;
}

- (void)setHasGlobalMenuItem:(bool)value
{
    _isReparented = value;
}

@end

@implementation FrnMenuItem
{
    ComObjectWeakPtr<FrnAppMenuItem> _item;
}

- (id) initWithFrnAppMenuItem: (FrnAppMenuItem*)menuItem
{
    if(self != nil)
    {
        _item = menuItem;
        self = [super initWithTitle:@""
                             action:@selector(didSelectItem:)
                      keyEquivalent:@""];
        
        [self setEnabled:YES];
        
        [self setTarget:self];
    }
    
    return self;
}

- (BOOL)validateMenuItem:(NSMenuItem *)menuItem
{
    if([self submenu] != nil)
    {
        return YES;
    }
    auto item = _item.tryGet();
    if(item == nullptr)
        return NO;
    
    return item->EvaluateItemEnabled();
}

- (void)didSelectItem:(nullable id)sender
{
    auto item = _item.tryGet();
    if(item == nullptr)
        return;
    item->RaiseOnClicked();
}
@end

FrnAppMenuItem::FrnAppMenuItem(bool isSeparator)
{
    _isCheckable = false;

    if(isSeparator)
    {
        _native = [NSMenuItem separatorItem];
    }
    else
    {
        _native = [[FrnMenuItem alloc] initWithFrnAppMenuItem: this];
    }
    
    _callback = nullptr;
}

NSMenuItem* FrnAppMenuItem::GetNative()
{
    return _native;
}

HRESULT FrnAppMenuItem::SetSubMenu (IFrnMenu* menu)
{
    START_COM_CALL;

    @autoreleasepool
    {
        if(menu != nullptr)
        {
            auto nsMenu = dynamic_cast<FrnAppMenu*>(menu)->GetNative();

            [_native setSubmenu: nsMenu];

            // Parity with -[NSMenu setSubmenu:forItem:]: without submenuAction: a click is dispatched to
            // didSelectItem: and dismisses the menu instead of opening the submenu.
            [_native setTarget: nil];
            [_native setAction: @selector(submenuAction:)];
        }
        else
        {
            [_native setSubmenu: nullptr];

            // The item is reused, so put its own action back.
            [_native setTarget: _native];
            [_native setAction: @selector(didSelectItem:)];
        }

        return S_OK;
    }
}

HRESULT FrnAppMenuItem::SetTitle (char* utf8String)
{
    START_COM_CALL;
    
    @autoreleasepool
    {
        if (utf8String != nullptr)
        {
            [_native setTitle:[NSString stringWithUTF8String:(const char*)utf8String]];
        }
        
        return S_OK;
    }
}

HRESULT FrnAppMenuItem::SetToolTip (char* utf8String)
{
    START_COM_CALL;

    @autoreleasepool
    {
        if (utf8String != nullptr)
        {
            [_native setToolTip:[NSString stringWithUTF8String:(const char*)utf8String]];
        }

        return S_OK;
    }
}

HRESULT FrnAppMenuItem::SetGesture (FrnKey key, FrnInputModifiers modifiers)
{
    START_COM_CALL;
    
    @autoreleasepool
    {
        if(key != FrnKeyNone)
        {
            NSEventModifierFlags flags = 0;
            
            if (modifiers & Control)
                flags |= NSEventModifierFlagControl;
            if (modifiers & Shift)
                flags |= NSEventModifierFlagShift;
            if (modifiers & Alt)
                flags |= NSEventModifierFlagOption;
            if (modifiers & Windows)
                flags |= NSEventModifierFlagCommand;
            
            auto menuChar = MenuCharFromVirtualKey(key);
            
            if (menuChar != 0)
            {
                auto keyString = [NSString stringWithCharacters:&menuChar length:1];
                
                [_native setKeyEquivalent: keyString];
                [_native setKeyEquivalentModifierMask:flags];
                
                return S_OK;
            }
        }
        
        // Nothing matched... clear.
        [_native setKeyEquivalent: @""];
        [_native setKeyEquivalentModifierMask: 0];
        
        return S_OK;
    }
}

HRESULT FrnAppMenuItem::SetAction (IFrnPredicateCallback* predicate, IFrnActionCallback* callback)
{
    START_COM_CALL;
    
    @autoreleasepool
    {
        _predicate = predicate;
        _callback = callback;
        return S_OK;
    }
}

HRESULT FrnAppMenuItem::SetIsChecked (bool isChecked)
{
    START_COM_CALL;
    
    @autoreleasepool
    {
        [_native setState:(isChecked && _isCheckable ? NSOnState : NSOffState)];
        return S_OK;
    }
}

HRESULT FrnAppMenuItem::SetIsVisible (bool isVisible)
{
    START_COM_CALL;
    
    @autoreleasepool
    {
        [_native setHidden:!isVisible];
        return S_OK;
    }
}

HRESULT FrnAppMenuItem::SetToggleType(FrnMenuItemToggleType toggleType)
{
    START_COM_CALL;
    
    @autoreleasepool
    {
        switch(toggleType)
        {
            case FrnMenuItemToggleType::None:
                [_native setOnStateImage: [NSImage imageNamed:@"NSMenuCheckmark"]];
                
                _isCheckable = false;
                break;
                
            case FrnMenuItemToggleType::CheckMark:
                [_native setOnStateImage: [NSImage imageNamed:@"NSMenuCheckmark"]];
                
                _isCheckable = true;
                break;
                
            case FrnMenuItemToggleType::Radio:
                [_native setOnStateImage: [NSImage imageNamed:@"NSMenuItemBullet"]];
                
                _isCheckable = true;
                break;
        }
        
        return S_OK;
    }
}

HRESULT FrnAppMenuItem::SetIcon(void *data, size_t length)
{
    START_COM_CALL;
    
    @autoreleasepool
    {
        if(data != nullptr)
        {
            NSData *imageData = [NSData dataWithBytes:data length:length];
            NSImage *image = [[NSImage alloc] initWithData:imageData];
            
            NSSize originalSize = [image size];
             
            NSSize size;
            size.height = floor([[NSFont menuFontOfSize:0] pointSize] * 1.333333);
            
            auto scaleFactor = size.height / originalSize.height;
            size.width = floor(originalSize.width * scaleFactor);
            
            [image setSize: size];
            [_native setImage:image];
        }
        else
        {
            [_native setImage:nullptr];
        }
        return S_OK;
    }
}

bool FrnAppMenuItem::EvaluateItemEnabled()
{
    if(_predicate != nullptr)
    {
        auto result = _predicate->Evaluate ();
        
        return result;
    }
    
    return false;
}

void FrnAppMenuItem::RaiseOnClicked()
{
    if(_callback != nullptr)
    {
        _callback->Run();
    }
}

FrnAppMenu::FrnAppMenu(IFrnMenuEvents* events)
{
    _baseEvents = events;
    _delegate = [[FrnMenuDelegate alloc] initWithParent: this];
    _native = [[FrnMenu alloc] initWithDelegate: _delegate];
}

FrnAppMenu::~FrnAppMenu()
{
    [_delegate parentDestroyed];
}


FrnMenu* FrnAppMenu::GetNative()
{
    return _native;
}

void FrnAppMenu::RaiseNeedsUpdate()
{
    if(_baseEvents != nullptr)
    {
        _baseEvents->NeedsUpdate();
    }
}

void FrnAppMenu::RaiseOpening()
{
    if(_baseEvents != nullptr)
    {
        _baseEvents->Opening();
    }
}

void FrnAppMenu::RaiseClosed()
{
    if(_baseEvents != nullptr)
    {
        _baseEvents->Closed();
    }
}


HRESULT FrnAppMenu::InsertItem(int index, IFrnMenuItem *item)
{
    START_COM_CALL;
    
    @autoreleasepool
    {
        if([_native hasGlobalMenuItem])
        {
            index++;
        }
        
        auto frnMenuItem = dynamic_cast<FrnAppMenuItem*>(item);
        
        if(frnMenuItem != nullptr)
        {
            [_native insertItem: frnMenuItem->GetNative() atIndex:index];
        }
        
        return S_OK;
    }
}

HRESULT FrnAppMenu::RemoveItem (IFrnMenuItem* item)
{
    START_COM_CALL;
    
    @autoreleasepool
    {
        auto frnMenuItem = dynamic_cast<FrnAppMenuItem*>(item);
        
        if(frnMenuItem != nullptr)
        {
            [_native removeItem:frnMenuItem->GetNative()];
        }
        
        return S_OK;
    }
}

HRESULT FrnAppMenu::SetTitle (char* utf8String)
{
    START_COM_CALL;
    
    @autoreleasepool
    {
        if (utf8String != nullptr)
        {
            [_native setTitle:[NSString stringWithUTF8String:(const char*)utf8String]];
        }
        
        return S_OK;
    }
}

HRESULT FrnAppMenu::Clear()
{
    START_COM_CALL;
    
    @autoreleasepool
    {
        [_native removeAllItems];
        return S_OK;
    }
}

@implementation FrnMenuDelegate
{
    FrnAppMenu* _parent;
}
- (id) initWithParent:(FrnAppMenu *)parent
{
    self = [super init];
    _parent = parent;
    return self;
}

- (void) parentDestroyed
{
    _parent = nullptr;
}

- (BOOL)menu:(NSMenu *)menu updateItem:(NSMenuItem *)item atIndex:(NSInteger)index shouldCancel:(BOOL)shouldCancel
{
    if(shouldCancel)
        return NO;
    return YES;
}

- (NSInteger)numberOfItemsInMenu:(NSMenu *)menu
{
    return [menu numberOfItems];
}

- (void)menuNeedsUpdate:(NSMenu *)menu
{
    if(_parent)
        _parent->RaiseNeedsUpdate();
}

- (void)menuWillOpen:(NSMenu *)menu
{
    if(_parent)
        _parent->RaiseOpening();
}

- (void)menuDidClose:(NSMenu *)menu
{
    if(_parent)
        _parent->RaiseClosed();
}

@end

extern IFrnMenu* CreateAppMenu(IFrnMenuEvents* cb)
{
    @autoreleasepool
    {
        return new FrnAppMenu(cb);
    }
}

extern IFrnMenuItem* CreateAppMenuItem()
{
    @autoreleasepool
    {
        return new FrnAppMenuItem(false);
    }
}

extern IFrnMenuItem* CreateAppMenuItemSeparator()
{
    @autoreleasepool
    {
        return new FrnAppMenuItem(true);
    }
}

static ComStaticPtr<FrnAppMenu> s_appMenu;
static NSMenuItem* s_appMenuItem = nullptr;

extern void SetAppMenu(IFrnMenu *menu)
{
    s_appMenu.set(dynamic_cast<FrnAppMenu*>(menu));
    
    if(s_appMenu != nullptr)
    {
        auto currentMenu = [s_appMenuItem menu];
        
        if (currentMenu != nullptr)
        {
            [currentMenu removeItem:s_appMenuItem];
        }
        
        s_appMenuItem = [s_appMenu->GetNative() itemAtIndex:0];
        
        if (currentMenu == nullptr)
        {
            currentMenu = [s_appMenuItem menu];
        }
        
        [[s_appMenuItem menu] removeItem:s_appMenuItem];
        
        [currentMenu insertItem:s_appMenuItem atIndex:0];
        
        if([s_appMenuItem submenu] == nullptr)
        {
            [s_appMenuItem setSubmenu:[NSMenu new]];
        }
    }
    else
    {
        s_appMenuItem = nullptr;
    }
}

extern void SetServicesMenu (IFrnMenu* menu)
{
    auto nativeMenu = dynamic_cast<FrnAppMenu*>(menu);
    [NSApplication sharedApplication].servicesMenu = nativeMenu->GetNative();
}

extern FrnAppMenu* GetAppMenu ()
{
    return s_appMenu.getRaw();
}

extern NSMenuItem* GetAppMenuItem ()
{
    return s_appMenuItem;
}


