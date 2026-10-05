//
// Created by Dan Walmsley on 05/05/2022.
//

#pragma once

#import <Foundation/Foundation.h>
#include "ferro-native.h"

@interface AutoFitContentView : NSView
-(AutoFitContentView* _Nonnull) initWithContent: (NSView* _Nonnull) content;
-(void) ShowTitleBar: (bool) show;
-(void) SetTitleBarHeightHint: (double) height;

-(void) ShowBlur: (bool) show;
@end