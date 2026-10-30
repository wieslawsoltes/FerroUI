#include "common.h"
#include "FrnString.h"

@interface CocoaThemeObserver : NSObject
-(id)initWithCallback:(IFrnActionCallback *)callback;
@end

@interface CocoaLocaleObserver : NSObject
-(id)initWithCallback:(IFrnActionCallback *)callback;
-(void)localeDidChange:(NSNotification *)notification;
@end

@interface CocoaAccessibilityObserver : NSObject
-(id)initWithCallback:(IFrnActionCallback *)callback;
-(void)accessibilityOptionsDidChange:(NSNotification *)notification;
-(void)invalidate;
@end

// Keep IFrnPlatformSettings' original vtable intact. New runtimes discover the
// optional interface with QueryInterface and can still load an older native lib.
class PlatformSettings : public ComObject,
                         public virtual IFrnPlatformSettings,
                         public virtual IFrnAccessibilitySettings
{
    CocoaThemeObserver* observer = nil;
    CocoaLocaleObserver* localeObserver = nil;
    CocoaAccessibilityObserver* accessibilityObserver = nil;

    BEGIN_INTERFACE_MAP()
    INTERFACE_MAP_ENTRY(IFrnPlatformSettings, IID_IFrnPlatformSettings)
    INTERFACE_MAP_ENTRY(IFrnAccessibilitySettings, IID_IFrnAccessibilitySettings)
    END_INTERFACE_MAP()

public:
    FORWARD_IUNKNOWN()

    virtual ~PlatformSettings()
    {
        RegisterColorsChange(nullptr);
        RegisterLanguageChange(nullptr);
        RegisterAccessibilityChange(nullptr);
    }

    virtual FrnPlatformThemeVariant GetPlatformTheme() override
    {
        @autoreleasepool
        {
            if (@available(macOS 10.14, *))
            {
                if (NSApplication.sharedApplication.effectiveAppearance.name == NSAppearanceNameAqua
                    || NSApplication.sharedApplication.effectiveAppearance.name == NSAppearanceNameVibrantLight) {
                    return FrnPlatformThemeVariant::Light;
                } else if (NSApplication.sharedApplication.effectiveAppearance.name == NSAppearanceNameDarkAqua
                    || NSApplication.sharedApplication.effectiveAppearance.name == NSAppearanceNameVibrantDark) {
                    return FrnPlatformThemeVariant::Dark;
                } else if (NSApplication.sharedApplication.effectiveAppearance.name == NSAppearanceNameAccessibilityHighContrastAqua
                    || NSApplication.sharedApplication.effectiveAppearance.name == NSAppearanceNameAccessibilityHighContrastVibrantLight) {
                    return FrnPlatformThemeVariant::HighContrastLight;
                } else if (NSApplication.sharedApplication.effectiveAppearance.name == NSAppearanceNameAccessibilityHighContrastDarkAqua
                    || NSApplication.sharedApplication.effectiveAppearance.name == NSAppearanceNameAccessibilityHighContrastVibrantDark) {
                    return FrnPlatformThemeVariant::HighContrastDark;
                }
            }
            return FrnPlatformThemeVariant::Light;
        }
    }
    
    virtual unsigned int GetAccentColor() override
    {
        @autoreleasepool
        {
            if (@available(macOS 11.0, *))
            {
                __block NSColor* color;
                [[NSApp effectiveAppearance] performAsCurrentDrawingAppearance:^{
                    color = [[NSColor controlAccentColor] colorUsingColorSpace:NSColorSpace.sRGBColorSpace];
                }];
                return to_argb(color);
            }
            else if (@available(macOS 10.14, *))
            {
                auto previousAppearance = NSAppearance.currentAppearance;
                NSAppearance.currentAppearance = [NSApp effectiveAppearance];
                auto color = [[NSColor controlAccentColor] colorUsingColorSpace:NSColorSpace.sRGBColorSpace];
                NSAppearance.currentAppearance = previousAppearance;
                return to_argb(color);
            }
            else
            {
                return 0;
            }
        }
    }
    
    virtual void RegisterColorsChange(IFrnActionCallback *callback) override
    {
        if (@available(macOS 10.14, *))
        {
            if (observer != nil)
            {
                [[NSApplication sharedApplication] removeObserver:observer forKeyPath:@"effectiveAppearance"];
                observer = nil;
            }
            if (callback != nullptr)
            {
                observer = [[CocoaThemeObserver alloc] initWithCallback: callback];
                [[NSApplication sharedApplication] addObserver:observer forKeyPath:@"effectiveAppearance" options:NSKeyValueObservingOptionNew context:nil];
            }
        }
    }

    virtual HRESULT GetPreferredLanguage(IFrnString** ret) override
    {
        @autoreleasepool
        {
            if (ret == nullptr)
                return E_POINTER;

            auto language = [[NSLocale preferredLanguages] firstObject];
            *ret = language == nil ? nullptr : CreateFrnString(language);
            return S_OK;
        }
    }

    virtual void RegisterLanguageChange(IFrnActionCallback *callback) override
    {
        if (localeObserver != nil)
        {
            [[NSNotificationCenter defaultCenter] removeObserver:localeObserver];
            localeObserver = nil;
        }
        if (callback != nullptr)
        {
            localeObserver = [[CocoaLocaleObserver alloc] initWithCallback: callback];
            [[NSNotificationCenter defaultCenter] addObserver:localeObserver
                                                     selector:@selector(localeDidChange:)
                                                         name:NSCurrentLocaleDidChangeNotification
                                                       object:nil];
        }
    }

    virtual bool GetReduceMotion() override
    {
        return NSWorkspace.sharedWorkspace.accessibilityDisplayShouldReduceMotion;
    }

    virtual bool GetReduceTransparency() override
    {
        return NSWorkspace.sharedWorkspace.accessibilityDisplayShouldReduceTransparency;
    }

    virtual void RegisterAccessibilityChange(IFrnActionCallback *callback) override
    {
        auto center = NSWorkspace.sharedWorkspace.notificationCenter;
        if (accessibilityObserver != nil)
        {
            [accessibilityObserver invalidate];
            [center removeObserver:accessibilityObserver];
            accessibilityObserver = nil;
        }
        if (callback != nullptr)
        {
            accessibilityObserver = [[CocoaAccessibilityObserver alloc] initWithCallback:callback];
            [center addObserver:accessibilityObserver
                       selector:@selector(accessibilityOptionsDidChange:)
                           name:NSWorkspaceAccessibilityDisplayOptionsDidChangeNotification
                         object:nil];
        }
    }
    
private:
    unsigned int to_argb(NSColor* color)
    {
        const CGFloat* components = CGColorGetComponents(color.CGColor);
        unsigned int alpha = static_cast<unsigned int>(CGColorGetAlpha(color.CGColor) * 0xFF);
        unsigned int red = static_cast<unsigned int>(components[0] * 0xFF);
        unsigned int green = static_cast<unsigned int>(components[1] * 0xFF);
        unsigned int blue = static_cast<unsigned int>(components[2] * 0xFF);
        return (alpha << 24) + (red << 16) + (green << 8) + blue;
    }
};

@implementation CocoaThemeObserver
{
    ComPtr<IFrnActionCallback> _callback;
}
- (id) initWithCallback:(IFrnActionCallback *)callback{
    self = [super init];
    if (self) {
        _callback = callback;
    }
    return self;
}

/*- (void)didChangeValueForKey:(NSString *)key {
    if([key isEqualToString:@"effectiveAppearance"]) {
        _callback->Run();
    }
    else {
        [super didChangeValueForKey:key];
    }
}*/

- (void)observeValueForKeyPath:(NSString *)keyPath
                      ofObject:(id)object
                        change:(NSDictionary *)change
                       context:(void *)context {
    if([keyPath isEqualToString:@"effectiveAppearance"]) {
        auto callback = _callback;
        callback->Run();
    } else {
        [super observeValueForKeyPath:keyPath
                             ofObject:object
                               change:change
                              context:context];
    }
}
@end

@implementation CocoaLocaleObserver
{
    ComPtr<IFrnActionCallback> _callback;
}
- (id) initWithCallback:(IFrnActionCallback *)callback {
    self = [super init];
    if (self) {
        _callback = callback;
    }
    return self;
}

- (void)localeDidChange:(NSNotification *)notification {
    auto callback = _callback;
    callback->Run();
}
@end

@implementation CocoaAccessibilityObserver
{
    ComPtr<IFrnActionCallback> _callback;
}

- (id)initWithCallback:(IFrnActionCallback *)callback {
    self = [super init];
    if (self) {
        _callback = callback;
    }
    return self;
}

- (void)invalidate {
    _callback = nullptr;
}

- (void)invokeCallback {
    // A subscriber can dispose the settings during the call. Keep the COM
    // callback alive even when unregistration clears the observer's field.
    auto callback = _callback;
    if (callback != nullptr) {
        callback->Run();
    }
}

- (void)accessibilityOptionsDidChange:(NSNotification *)notification {
    // Rust platform settings and their subscribers belong to the UI thread.
    // Capture the observer, not a Rust Rc, when a notification arrives elsewhere.
    if ([NSThread isMainThread]) {
        [self invokeCallback];
    } else {
        dispatch_async(dispatch_get_main_queue(), ^{
            [self invokeCallback];
        });
    }
}
@end

extern IFrnPlatformSettings* CreatePlatformSettings()
{
    return new PlatformSettings();
}
