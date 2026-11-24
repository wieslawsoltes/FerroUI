#import <Foundation/Foundation.h>
#import <AppKit/AppKit.h>
#include "WindowProtocol.h"
#include "WindowBaseImpl.h"
#include "FrnAccessibility.h"

@interface FrnWindow : NSWindow <FrnWindowProtocol, NSWindowDelegate, FrnAccessibility>
-(FrnWindow* _Nonnull) initWithParent: (const ComPtr<WindowBaseImpl>&) parent contentRect: (NSRect)contentRect styleMask: (NSWindowStyleMask)styleMask;
-(FrnView* _Nullable) view;
@end

@interface FrnPanel : NSPanel <FrnWindowProtocol, NSWindowDelegate, FrnAccessibility>
-(FrnPanel* _Nonnull) initWithParent: (const ComPtr<WindowBaseImpl>&) parent contentRect: (NSRect)contentRect styleMask: (NSWindowStyleMask)styleMask;
@end
