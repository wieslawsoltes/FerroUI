#pragma once

#import <AppKit/AppKit.h>
#include "comimpl.h"

@class FrnMenu;
struct IFrnAutomationPeer;

@protocol FrnWindowProtocol
-(void) pollModalSession: (NSModalSession _Nonnull) session;
-(bool) shouldTryToHandleEvents;
-(void) setEnabled: (bool) enable;
-(void) showAppMenuOnly;
-(void) showWindowMenuWithAppMenu;
-(void) applyMenu:(FrnMenu* _Nullable)menu;
-(ComPtr<IFrnAutomationPeer>) automationPeer;

-(double) getExtendedTitleBarHeight;
-(void) setIsExtended:(bool)value;
-(void) setStandardButtonsTitleBarHeight:(double)height;
-(void) layoutStandardButtons;
-(void) disconnectParent;
-(bool) isDialog;

-(void) setCanBecomeKeyWindow:(bool)value;
@end
