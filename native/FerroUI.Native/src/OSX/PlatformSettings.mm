#include "common.h"
#include "FrnString.h"

@interface CocoaThemeObserver : NSObject
-(id)initWithCallback:(IFrnActionCallback *)callback;
@end

@interface CocoaLocaleObserver : NSObject
-(id)initWithCallback:(IFrnActionCallback *)callback;
-(void)localeDidChange:(NSNotification *)notification;
@end

class PlatformSettings : public ComSingleObject<IFrnPlatformSettings, &IID_IFrnPlatformSettings>
{
    CocoaThemeObserver* observer;
    CocoaLocaleObserver* localeObserver;

public:
    FORWARD_IUNKNOWN()
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
            observer = [[CocoaThemeObserver alloc] initWithCallback: callback];
            [[NSApplication sharedApplication] addObserver:observer forKeyPath:@"effectiveAppearance" options:NSKeyValueObservingOptionNew context:nil];
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
        localeObserver = [[CocoaLocaleObserver alloc] initWithCallback: callback];
        [[NSNotificationCenter defaultCenter] addObserver:localeObserver
                                                 selector:@selector(localeDidChange:)
                                                     name:NSCurrentLocaleDidChangeNotification
                                                   object:nil];
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
        _callback->Run();
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
    _callback->Run();
}
@end

extern IFrnPlatformSettings* CreatePlatformSettings()
{
    return new PlatformSettings();
}
