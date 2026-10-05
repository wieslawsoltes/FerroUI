//
// Created by Dan Walmsley on 05/05/2022.
//
#pragma once
#import <Foundation/Foundation.h>

#import <AppKit/AppKit.h>
#include "common.h"
#include "TopLevelImpl.h"
#include "KeyTransform.h"

@class FrnAccessibilityElement;
@protocol IRenderTarget;

@interface FrnView : NSView<NSTextInputClient, NSDraggingDestination, FrnTextInputMethodDelegate, CALayerDelegate>
-(FrnView* _Nonnull) initWithParent: (TopLevelImpl* _Nonnull) parent;
-(NSEvent* _Nonnull) lastMouseDownEvent;
-(FrnPoint) translateLocalPoint:(FrnPoint)pt;
-(void) onClosed;
-(void) setModifiers:(NSEventModifierFlags)modifierFlags;

-(FrnPlatformResizeReason) getResizeReason;
-(void) setResizeReason:(FrnPlatformResizeReason)reason;
-(void) setRenderTarget:(NSObject<IRenderTarget>* _Nonnull)target;
-(void) raiseAccessibilityChildrenChanged;
@end
