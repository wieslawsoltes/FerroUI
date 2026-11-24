#pragma once

#import <Cocoa/Cocoa.h>
#include "FrnAccessibility.h"
NS_ASSUME_NONNULL_BEGIN

@interface FrnAccessibilityElement : NSAccessibilityElement <FrnAccessibility>
+ (id _Nullable) acquire:(const ComPtr<IFrnAutomationPeer>&) peer;
@end

NS_ASSUME_NONNULL_END
