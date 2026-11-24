

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

- (id) initWithFrnAppMenuItem: (const ComPtr<FrnAppMenuItem>&)menuItem
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
        _native = [[FrnMenuItem alloc] initWithFrnAppMenuItem: comself(this)];
    }
    
    _callback = nullptr;
}

NSMenuItem* FrnAppMenuItem::GetNative()
{
    return _native;
}

HRESULT FrnAppMenuItem::SetSubMenu (const ComPtr<IFrnMenu>& menu)
{
    @autoreleasepool
    {
        auto frnMenu = menu.dynamicCast<FrnAppMenu>();
        if(frnMenu != nullptr)
        {
            [_native setSubmenu: frnMenu->GetNative()];

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

HRESULT FrnAppMenuItem::SetAction (const ComPtr<IFrnPredicateCallback>& predicate, const ComPtr<IFrnActionCallback>& callback)
{
    @autoreleasepool
    {
        _predicate = predicate;
        _callback = callback;
        return S_OK;
    }
}

HRESULT FrnAppMenuItem::SetIsChecked (bool isChecked)
{
    @autoreleasepool
    {
        [_native setState:(isChecked && _isCheckable ? NSOnState : NSOffState)];
        return S_OK;
    }
}

HRESULT FrnAppMenuItem::SetIsVisible (bool isVisible)
{
    @autoreleasepool
    {
        [_native setHidden:!isVisible];
        return S_OK;
    }
}

HRESULT FrnAppMenuItem::SetToggleType(FrnMenuItemToggleType toggleType)
{
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

FrnAppMenu::FrnAppMenu(const ComPtr<IFrnMenuEvents>& events)
{
    _baseEvents = events;
    _delegate = [[FrnMenuDelegate alloc] initWithParent: comself(this)];
    _native = [[FrnMenu alloc] initWithDelegate: _delegate];
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


HRESULT FrnAppMenu::InsertItem(int index, const ComPtr<IFrnMenuItem>& item)
{
    @autoreleasepool
    {
        if([_native hasGlobalMenuItem])
        {
            index++;
        }
        
        auto frnMenuItem = item.dynamicCast<FrnAppMenuItem>();
        
        if(frnMenuItem != nullptr)
        {
            [_native insertItem: frnMenuItem->GetNative() atIndex:index];
        }
        
        return S_OK;
    }
}

HRESULT FrnAppMenu::RemoveItem (const ComPtr<IFrnMenuItem>& item)
{
    @autoreleasepool
    {
        auto frnMenuItem = item.dynamicCast<FrnAppMenuItem>();
        
        if(frnMenuItem != nullptr)
        {
            [_native removeItem:frnMenuItem->GetNative()];
        }
        
        return S_OK;
    }
}

HRESULT FrnAppMenu::SetTitle (char* utf8String)
{
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
    @autoreleasepool
    {
        [_native removeAllItems];
        return S_OK;
    }
}

@implementation FrnMenuDelegate
{
    ComObjectWeakPtr<FrnAppMenu> _parent;
}
- (id) initWithParent:(const ComPtr<FrnAppMenu>&)parent
{
    self = [super init];
    _parent = parent;
    return self;
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
    if(auto parent = _parent.tryGet())
        parent->RaiseNeedsUpdate();
}

- (void)menuWillOpen:(NSMenu *)menu
{
    if(auto parent = _parent.tryGet())
        parent->RaiseOpening();
}

- (void)menuDidClose:(NSMenu *)menu
{
    if(auto parent = _parent.tryGet())
        parent->RaiseClosed();
}

@end

extern ComPtr<IFrnMenu> CreateAppMenu(const ComPtr<IFrnMenuEvents>& cb)
{
    return comnew<FrnAppMenu>(cb);
}

extern ComPtr<IFrnMenuItem> CreateAppMenuItem()
{
    return comnew<FrnAppMenuItem>(false);
}

extern ComPtr<IFrnMenuItem> CreateAppMenuItemSeparator()
{
    return comnew<FrnAppMenuItem>(true);
}

static ComStaticPtr<FrnAppMenu> s_appMenu;
static NSMenuItem* s_appMenuItem = nullptr;

extern void SetAppMenu(const ComPtr<IFrnMenu>& menu)
{
    s_appMenu.set(menu.dynamicCast<FrnAppMenu>());
    
    if(s_appMenu)
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

extern void SetServicesMenu(const ComPtr<IFrnMenu>& menu)
{
    auto nativeMenu = menu.dynamicCast<FrnAppMenu>();
    [NSApplication sharedApplication].servicesMenu = nativeMenu ? nativeMenu->GetNative() : nil;
}

extern ComPtr<FrnAppMenu> GetAppMenu()
{
    return s_appMenu;
}

extern NSMenuItem* GetAppMenuItem ()
{
    return s_appMenuItem;
}


