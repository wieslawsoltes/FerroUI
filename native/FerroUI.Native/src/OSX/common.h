#ifndef common_h
#define common_h
#include "comimpl.h"
#include "ferro-native.h"
#include <stdio.h>
#import <Foundation/Foundation.h>
#import <AppKit/AppKit.h>
#include <pthread.h>
#include "noarc.h"

extern ComPtr<IFrnPlatformThreadingInterface> CreatePlatformThreading();
extern void FreeFrnGCHandle(void* handle);
extern void PostDispatcherCallback(const ComPtr<IFrnActionCallback>& cb);
extern ComPtr<IFrnTopLevel> CreateFrnTopLevel(const ComPtr<IFrnTopLevelEvents>& events);
extern ComPtr<IFrnWindow> CreateFrnWindow(const ComPtr<IFrnWindowEvents>& events);
extern ComPtr<IFrnPopup> CreateFrnPopup(const ComPtr<IFrnWindowEvents>& events);
extern ComPtr<IFrnStorageProvider> CreateStorageProvider();
extern ComPtr<IFrnScreens> CreateScreens(const ComPtr<IFrnScreenEvents>& cb);
extern ComPtr<IFrnClipboard> CreateClipboard(NSPasteboard* pb);
extern NSObject<NSDraggingSource>* CreateDraggingSource(NSDragOperation op, const ComPtr<IFrnDndResultCallback>& cb, void* handle);
extern void* GetFrnDataObjectHandleFromDraggingInfo(NSObject<NSDraggingInfo>* info);
extern NSString* GetFrnCustomDataType();
extern FrnDragDropEffects ConvertDragDropEffects(NSDragOperation nsop);
extern ComPtr<IFrnCursorFactory> CreateCursorFactory();
extern ComPtr<IFrnGlDisplay> GetGlDisplay();
extern ComPtr<IFrnMetalDisplay> GetMetalDisplay();
extern ComPtr<IFrnMenu> CreateAppMenu(const ComPtr<IFrnMenuEvents>& events);
extern ComPtr<IFrnTrayIcon> CreateTrayIcon();
extern ComPtr<IFrnMenuItem> CreateAppMenuItem();
extern ComPtr<IFrnMenuItem> CreateAppMenuItemSeparator();
extern ComPtr<IFrnApplicationCommands> CreateApplicationCommands();
extern ComPtr<IFrnPlatformBehaviorInhibition> CreatePlatformBehaviorInhibition();
extern ComPtr<IFrnNativeControlHost> CreateNativeControlHost(NSView* parent);
extern ComPtr<IFrnPlatformSettings> CreatePlatformSettings();
extern ComPtr<IFrnPlatformRenderTimer> CreatePlatformRenderTimer();
extern ComPtr<IFrnNativeObjectsMemoryManagement> CreateMemoryManagementHelper();
extern void SetAppMenu(const ComPtr<IFrnMenu>& menu);
extern void SetServicesMenu(const ComPtr<IFrnMenu>& menu);
class FrnAppMenu;
extern ComPtr<FrnAppMenu> GetAppMenu();
extern NSMenuItem* GetAppMenuItem ();
extern void SetDockMenu(NSMenu* menu);

extern void InitializeFrnApp(const ComPtr<IFrnApplicationEvents>& events, bool disableAppDelegate);
extern void ReleaseFrnAppEvents();
extern NSApplicationActivationPolicy FrnDesiredActivationPolicy;
extern NSPoint ToNSPoint (FrnPoint p);
extern NSRect ToNSRect (FrnRect r);
extern FrnPoint ToFrnPoint (NSPoint p);
extern FrnPoint ConvertPointY (FrnPoint p);
extern NSSize ToNSSize (FrnSize s);
extern FrnSize FromNSSize (NSSize s);
extern ComPtr<IFrnMTLSharedEvent> ImportMTLSharedEvent(void* object);
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
- (ActionCallback*) initWithCallback: (const ComPtr<IFrnActionCallback>&) callback;
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
    virtual HRESULT ShowAboutPanel(const char* applicationName, const char* version) override;
};
#define NSApp [NSApplication sharedApplication]


#endif
