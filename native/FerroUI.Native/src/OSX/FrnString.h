//
//  FrnString.h
//  FerroUI.Native.OSX
//

#ifndef FrnString_h
#define FrnString_h

extern ComPtr<IFrnString> CreateFrnString(NSString* string);
extern ComPtr<IFrnStringArray> CreateFrnStringArray(NSArray<NSString*>* array);
extern ComPtr<IFrnStringArray> CreateFrnStringArray(NSArray<NSURL*>* array);
extern ComPtr<IFrnStringArray> CreateFrnStringArray(NSString* string);
extern ComPtr<IFrnString> CreateByteArray(void* data, int len);
extern NSString* GetNSString(const ComPtr<IFrnString>& s);
extern NSArray<NSString*>* GetNSArrayOfStrings(const ComPtr<IFrnStringArray>& array);
#endif /* FrnString_h */
