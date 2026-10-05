#ifndef keytransform_h
#define keytransform_h

#import <cstdint>
#include "common.h"

FrnPhysicalKey PhysicalKeyFromScanCode(uint16_t scanCode);

FrnKey VirtualKeyFromScanCode(uint16_t scanCode, NSEventModifierFlags modifierFlags);

NSString* KeySymbolFromScanCode(uint16_t scanCode, NSEventModifierFlags modifierFlags);

uint16_t MenuCharFromVirtualKey(FrnKey key);

#endif
