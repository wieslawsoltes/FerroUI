//
//  FrnString.h
//  FerroUI.Native.OSX
//
//  Created by Dan Walmsley on 07/11/2018.
//

#ifndef FrnString_h
#define FrnString_h

extern IFrnString* CreateFrnString(NSString* string);
extern IFrnStringArray* CreateFrnStringArray(NSArray<NSString*>* array);
extern IFrnStringArray* CreateFrnStringArray(NSArray<NSURL*>* array);
extern IFrnStringArray* CreateFrnStringArray(NSString* string);
extern IFrnString* CreateByteArray(void* data, int len);
extern NSString* GetNSStringAndRelease(IFrnString* s);
extern NSString* GetNSStringWithoutRelease(IFrnString* s);
extern NSArray<NSString*>* GetNSArrayOfStringsAndRelease(IFrnStringArray* array);
#endif /* FrnString_h */
