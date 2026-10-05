#pragma once
#import <Cocoa/Cocoa.h>
#import "ferro-native.h"

// Defines the interface between FrnAutomationNode and objects which implement
// NSAccessibility such as FrnAccessibilityElement or FrnWindow.
@protocol FrnAccessibility <NSAccessibility>
@required
- (void) raiseChildrenChanged;
- (void) raiseFocusChanged;
- (void) raisePropertyChanged:(FrnAutomationProperty)property;
@end
