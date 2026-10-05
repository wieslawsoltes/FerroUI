#ifndef common_h
#define common_h
#include "comimpl.h"
#include "ferro-native.h"
#include <stdio.h>
#import <Foundation/Foundation.h>
#import <AppKit/AppKit.h>
#include <pthread.h>
#include "noarc.h"

extern IFrnPlatformThreadingInterface* CreatePlatformThreading();
extern void FreeFrnGCHandle(void* handle);
extern void PostDispatcherCallback(IFrnActionCallback* cb);
extern IFrnTopLevel* CreateFrnTopLevel(IFrnTopLevelEvents* events);
extern IFrnWindow* CreateFrnWindow(IFrnWindowEvents*events);
extern IFrnPopup* CreateFrnPopup(IFrnWindowEvents*events);
extern IFrnStorageProvider* CreateStorageProvider();
extern IFrnScreens* CreateScreens(IFrnScreenEvents* cb);
extern IFrnClipboard* CreateClipboard(NSPasteboard* pb);
extern NSObject<NSDraggingSource>* CreateDraggingSource(NSDragOperation op, IFrnDndResultCallback* cb, void* handle);
extern void* GetFrnDataObjectHandleFromDraggingInfo(NSObject<NSDraggingInfo>* info);
extern NSString* GetFrnCustomDataType();
extern FrnDragDropEffects ConvertDragDropEffects(NSDragOperation nsop);
extern IFrnCursorFactory* CreateCursorFactory();
extern IFrnGlDisplay* GetGlDisplay();
extern IFrnMetalDisplay* GetMetalDisplay();
extern IFrnMenu* CreateAppMenu(IFrnMenuEvents* events);
extern IFrnTrayIcon* CreateTrayIcon();
extern IFrnMenuItem* CreateAppMenuItem();
extern IFrnMenuItem* CreateAppMenuItemSeparator();
extern IFrnApplicationCommands* CreateApplicationCommands();
extern IFrnPlatformBehaviorInhibition* CreatePlatformBehaviorInhibition();
extern IFrnNativeControlHost* CreateNativeControlHost(NSView* parent);
extern IFrnPlatformSettings* CreatePlatformSettings();
extern IFrnPlatformRenderTimer* CreatePlatformRenderTimer();
extern IFrnNativeObjectsMemoryManagement* CreateMemoryManagementHelper();
extern void SetAppMenu(IFrnMenu *menu);
extern void SetServicesMenu (IFrnMenu* menu);
class FrnAppMenu;
extern FrnAppMenu* GetAppMenu ();
extern NSMenuItem* GetAppMenuItem ();
extern void SetDockMenu(NSMenu* menu);

extern void InitializeFrnApp(IFrnApplicationEvents* events, bool disableAppDelegate);
extern void ReleaseFrnAppEvents();
extern NSApplicationActivationPolicy FrnDesiredActivationPolicy;
extern NSPoint ToNSPoint (FrnPoint p);
extern NSRect ToNSRect (FrnRect r);
extern FrnPoint ToFrnPoint (NSPoint p);
extern FrnPoint ConvertPointY (FrnPoint p);
extern NSSize ToNSSize (FrnSize s);
extern FrnSize FromNSSize (NSSize s);
extern IFrnMTLSharedEvent* ImportMTLSharedEvent(void* object);
#ifdef DEBUG
#define NSDebugLog(...) NSLog(__VA_ARGS__)
#else
#define NSDebugLog(...) (void)0
#endif

template<typename T> inline T* objc_cast(id from) {
    if(from == nil)
        return nil;
    if ([from isKindOfClass:[T class]]) {
        return static_cast<T*>(from);
    }
    return nil;
}

template<typename T> class ObjCWrapper {
public:
    T* Value;
    ObjCWrapper(T* value)
    {
        Value = value;
    }
    operator T*() const
    {
        return Value;
    }
    T* operator->() const
    {
        return Value;
    }
    ~ObjCWrapper()
    {
        Value = nil;
    }
};

@interface ActionCallback : NSObject
- (ActionCallback*) initWithCallback: (IFrnActionCallback*) callback;
- (void) action;
@end

@implementation NSScreen (FrnNSScreen)
- (CGDirectDisplayID)av_displayId
{
    return [self.deviceDescription[@"NSScreenNumber"] unsignedIntValue];
}
@end

class FrnInsidePotentialDeadlock
{
public:
    static bool IsInside();
    FrnInsidePotentialDeadlock();
    ~FrnInsidePotentialDeadlock();
};


class FrnApplicationCommands : public ComSingleObject<IFrnApplicationCommands, &IID_IFrnApplicationCommands>
{
public:
    FORWARD_IUNKNOWN()
    
    virtual HRESULT UnhideApp() override;
    virtual HRESULT HideApp() override;
    virtual HRESULT ShowAll() override;
    virtual HRESULT HideOthers() override;
};
#define NSApp [NSApplication sharedApplication]

#define START_COM_ARP_CALL START_ARP_CALL; START_COM_CALL

#endif
