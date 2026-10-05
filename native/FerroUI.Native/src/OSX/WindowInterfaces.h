//
// Created by Dan Walmsley on 06/05/2022.
//

#import <Foundation/Foundation.h>
#import <AppKit/AppKit.h>
#include "WindowProtocol.h"
#include "WindowBaseImpl.h"
#include "FrnAccessibility.h"

@interface FrnWindow : NSWindow <FrnWindowProtocol, NSWindowDelegate, FrnAccessibility>
-(FrnWindow* _Nonnull) initWithParent: (WindowBaseImpl* _Nonnull) parent contentRect: (NSRect)contentRect styleMask: (NSWindowStyleMask)styleMask;
-(FrnView* _Nullable) view;
@end

@interface FrnPanel : NSPanel <FrnWindowProtocol, NSWindowDelegate, FrnAccessibility>
-(FrnPanel* _Nonnull) initWithParent: (WindowBaseImpl* _Nonnull) parent contentRect: (NSRect)contentRect styleMask: (NSWindowStyleMask)styleMask;
@end
