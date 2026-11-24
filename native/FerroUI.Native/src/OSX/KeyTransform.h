#ifndef keytransform_h
#define keytransform_h

#import <cstdint>
#include "common.h"

FrnPhysicalKey PhysicalKeyFromScanCode(uint16_t scanCode);

FrnKey VirtualKeyFromScanCode(uint16_t scanCode, NSEventModifierFlags modifierFlags);

// With useLayoutFallback, an input method without its own layout data uses the keyboard layout that it types with
NSString* KeySymbolFromScanCode(uint16_t scanCode, NSEventModifierFlags modifierFlags, bool useLayoutFallback = false);

uint16_t MenuCharFromVirtualKey(FrnKey key);

// Gets the pressed modifiers that change the key's characters: Shift and Option. Control and Command are never consumed.
// Returns false when no keyboard layout data is available.
bool ConsumedModifiersFromScanCode(uint16_t scanCode, NSEventModifierFlags modifierFlags, FrnInputModifiers* result);

#endif
