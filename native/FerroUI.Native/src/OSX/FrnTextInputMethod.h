//
//  FrnTextInputMethod.h
//  FerroUI.Native.OSX
//
//  Created by Benedikt Stebner on 22.11.22.
//

#ifndef FrnTextInputMethod_h
#define FrnTextInputMethod_h

#import <Foundation/Foundation.h>

#include "com.h"
#include "comimpl.h"
#include "ferro-native.h"
#import "FrnTextInputMethodDelegate.h"

class FrnTextInputMethod: public virtual ComObject, public virtual IFrnTextInputMethod{
private:
    id<FrnTextInputMethodDelegate> _inputMethodDelegate;
public:
    FORWARD_IUNKNOWN()
    
    BEGIN_INTERFACE_MAP()
    INTERFACE_MAP_ENTRY(IFrnTextInputMethod, IID_IFrnTextInputMethod)
    END_INTERFACE_MAP()
    
    virtual ~FrnTextInputMethod();
    
    FrnTextInputMethod(id<FrnTextInputMethodDelegate> inputMethodDelegate);
    
    bool IsActive ();
    
    HRESULT SetClient (IFrnTextInputMethodClient* client) override;
    
    virtual void Reset () override;
    
    virtual void SetCursorRect (FrnRect rect) override;
    
    virtual void SetSurroundingText (char* text, int start, int end) override;
    
    virtual void SetSelectionInSurroundingText (int start, int end) override;
    
public:
    ComPtr<IFrnTextInputMethodClient> Client;
};
#endif /* FrnTextInputMethod_h */
