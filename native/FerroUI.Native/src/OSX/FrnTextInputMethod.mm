//
//  FrnTextInputMethod.mm
//  FerroUI.Native.OSX
//
//  Created by Benedikt Stebner on 23.11.22.
//

#include "FrnTextInputMethod.h"

FrnTextInputMethod::~FrnTextInputMethod() {
    Client = nullptr;
}

FrnTextInputMethod::FrnTextInputMethod(id<FrnTextInputMethodDelegate> inputMethodDelegate) {
    _inputMethodDelegate = inputMethodDelegate;
}

bool FrnTextInputMethod::IsActive() {
    return Client != nullptr;
}

HRESULT FrnTextInputMethod::SetClient(IFrnTextInputMethodClient *client) {
    START_COM_CALL;
    
    Client = client;
    
    return S_OK;
}

void FrnTextInputMethod::Reset() {
    [_inputMethodDelegate resetInputMethod];
}

void FrnTextInputMethod::SetSurroundingText(char* text, int start, int end) {
    // stringWithUTF8String: throws on a null pointer and returns nil for invalid UTF-8.
    NSString* surroundingText = text != nullptr ? [NSString stringWithUTF8String:text] : nil;

    [_inputMethodDelegate setText:surroundingText != nil ? surroundingText : @""];
    [_inputMethodDelegate setSelection: start:end];
}

void FrnTextInputMethod::SetCursorRect(FrnRect rect) {
    [_inputMethodDelegate setCursorRect: rect];
}

void FrnTextInputMethod::SetSelectionInSurroundingText(int start, int end) {
    [_inputMethodDelegate setSelection: start:end];
}
