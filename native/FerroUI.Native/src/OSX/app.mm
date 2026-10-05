#include "common.h"
#include "FrnString.h"
#include "menu.h"
@interface FrnAppDelegate : NSObject<NSApplicationDelegate>
-(FrnAppDelegate* _Nonnull) initWithEvents: (IFrnApplicationEvents* _Nonnull) events;
-(void) releaseEvents;
@end

NSApplicationActivationPolicy FrnDesiredActivationPolicy = NSApplicationActivationPolicyRegular;
static NSMenu* s_dockMenu = nil;

static bool IsOSShutdown()
{
    auto evt = [[NSAppleEventManager sharedAppleEventManager] currentAppleEvent];
    if ([evt eventClass] != kCoreEventClass || [evt eventID] != kAEQuitApplication)
        return false;

    auto reason = [evt paramDescriptorForKeyword:kAEQuitReason];
    if (reason == nil)
        reason = [evt attributeDescriptorForKeyword:kAEQuitReason];

    auto reasonCode = [reason enumCodeValue];
    if (reasonCode == 0)
        reasonCode = [reason typeCodeValue];

    switch (reasonCode)
    {
        case kAELogOut:
        case kAEReallyLogOut:
        case kAEShowRestartDialog:
        case kAERestart:
        case kAEShowShutdownDialog:
        case kAEShutDown:
            return true;
        default:
            return false;
    }
}

@implementation FrnAppDelegate
ComPtr<IFrnApplicationEvents> _events;

- (FrnAppDelegate *)initWithEvents:(IFrnApplicationEvents *)events
{
    _events = events;
    return self;
}

- (void)releaseEvents
{
    _events = nil;
}

- (void)applicationWillFinishLaunching:(NSNotification *)notification
{
    if([[NSApplication sharedApplication] activationPolicy] != FrnDesiredActivationPolicy)
    {
        for (NSRunningApplication * app in [NSRunningApplication runningApplicationsWithBundleIdentifier:@"com.apple.dock"]) {
            [app activateWithOptions:NSApplicationActivateIgnoringOtherApps];
            break;
        }
        
        [[NSUserDefaults standardUserDefaults] setBool:NO forKey:@"NSFullScreenMenuItemEverywhere"];
        
        [[NSApplication sharedApplication] setHelpMenu: [[NSMenu new] initWithTitle:@""]];
    }
}

- (void)applicationDidFinishLaunching:(NSNotification *)notification
{
    [[NSRunningApplication currentApplication] activateWithOptions:NSApplicationActivateIgnoringOtherApps];
}

-(BOOL)applicationShouldHandleReopen:(NSApplication *)sender hasVisibleWindows:(BOOL)flag
{
    _events->OnReopen();
    return YES;
}

- (void)applicationDidHide:(NSNotification *)notification
{
    _events->OnHide();
}

- (void)applicationDidUnhide:(NSNotification *)notification
{
    _events->OnUnhide();
}

- (void) applicationDidBecomeActive:(NSNotification *) notification
{
    _events->OnActivate();
}

- (void) applicationDidResignActive:(NSNotification *) notification
{
    _events->OnDeactivate();
}

- (void)application:(NSApplication *)sender openFiles:(NSArray<NSString *> *)filenames
{
    auto array = CreateFrnStringArray(filenames);
    
    _events->FilesOpened(array);
}

- (void)application:(NSApplication *)application openURLs:(NSArray<NSURL *> *)urls
{
    auto array = CreateFrnStringArray(urls);
    
    _events->UrlsOpened(array);
}

- (NSApplicationTerminateReply)applicationShouldTerminate:(NSApplication *)sender
{
    switch (_events->TryShutdown(IsOSShutdown()))
    {
        case ShutdownReplyCancel:
            return NSTerminateCancel;
            
        // The managed dispatcher loop is exiting: let it handle the termination instead.
        case ShutdownReplyDeferToManagedLoop:
            return NSTerminateCancel;

        case ShutdownReplyTerminateNow:
            return NSTerminateNow;

        // Shouldn't happen
        default:
            return NSTerminateNow;
    }
}

- (void)applicationWillTerminate:(NSNotification *)notification
{
    if (!_events)
        return;
    
    // The process is about to exit() so this is the last point where managed code can still safely be called.
    // Keep the application events object alive for the duration of the call, it's about to be released by the managed side.
    ComPtr<IFrnApplicationEvents> events(_events);
    events->OnTerminating();
}

- (NSMenu *)applicationDockMenu:(NSApplication *)sender
{
    return s_dockMenu;
}

@end

@interface FrnApplication : NSApplication

@end

@implementation FrnApplication
{
    BOOL _isHandlingSendEvent;
}

- (void)sendEvent:(NSEvent *)event
{
    bool oldHandling = _isHandlingSendEvent;
    _isHandlingSendEvent = true;
    @try {
        [super sendEvent: event];
        if ([event type] == NSEventTypeKeyUp && ([event modifierFlags] & NSEventModifierFlagCommand))
        {
            [[self keyWindow] sendEvent:event];
        }
        
    } @finally {
        _isHandlingSendEvent = oldHandling;
    }
}

// This is needed for certain embedded controls DO NOT REMOVE..
- (BOOL) isHandlingSendEvent
{
    return _isHandlingSendEvent;
}

- (void)setHandlingSendEvent:(BOOL)handlingSendEvent
{
    _isHandlingSendEvent = handlingSendEvent;
}
@end

extern void InitializeFrnApp(IFrnApplicationEvents* events, bool disableAppDelegate)
{
    if(!disableAppDelegate)
    {
        NSApplication* app = [FrnApplication sharedApplication];
        id delegate = [[FrnAppDelegate alloc] initWithEvents:events];
        [app setDelegate:delegate];
    }
}

extern void ReleaseFrnAppEvents()
{
    NSApplication* app = [FrnApplication sharedApplication];
    id delegate = [app delegate];
    if ([delegate isMemberOfClass:[FrnAppDelegate class]])
    {
        FrnAppDelegate* frnDelegate = delegate;
        [frnDelegate releaseEvents];
        [app setDelegate:nil];
    }
}

HRESULT FrnApplicationCommands::UnhideApp()
{
    START_COM_CALL;
    [[NSApplication sharedApplication] unhide:[NSApp delegate]];
    return S_OK;
}

HRESULT FrnApplicationCommands::HideApp()
{
    START_COM_CALL;
    [[NSApplication sharedApplication] hide:[NSApp delegate]];
    return S_OK;
}

HRESULT FrnApplicationCommands::ShowAll()
{
    START_COM_CALL;
    [[NSApplication sharedApplication] unhideAllApplications:[NSApp delegate]];
    return S_OK;
}

HRESULT FrnApplicationCommands::HideOthers()
{
    START_COM_CALL;
    [[NSApplication sharedApplication] hideOtherApplications:[NSApp delegate]];
    return S_OK;
}


extern IFrnApplicationCommands* CreateApplicationCommands()
{
    return new FrnApplicationCommands();
}

extern void SetDockMenu(NSMenu* menu)
{
    s_dockMenu = menu;
}
