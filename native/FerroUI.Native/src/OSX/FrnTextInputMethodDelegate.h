//
//  FrnTextInputMethodHost.h
//  FerroUI.Native.OSX
//
//  Created by Benedikt Stebner on 24.11.22.
//

#ifndef FrnTextInputMethodHost_h
#define FrnTextInputMethodHost_h

@protocol FrnTextInputMethodDelegate
@required
-(void) setText:(NSString* _Nonnull) text;
-(void) setCursorRect:(FrnRect) cursorRect;
-(void) setSelection: (int) start : (int) end;
-(void) resetInputMethod;

@end

#endif /* FrnTextInputMethodHost_h */
