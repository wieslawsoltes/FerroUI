#pragma once

#import <Cocoa/Cocoa.h>
#include "FrnAccessibility.h"
NS_ASSUME_NONNULL_BEGIN

class IFrnAutomationPeer;

@interface FrnAccessibilityElement : NSAccessibilityElement <FrnAccessibility>
+ (id _Nullable) acquire:(IFrnAutomationPeer *) peer;
@end

NS_ASSUME_NONNULL_END
