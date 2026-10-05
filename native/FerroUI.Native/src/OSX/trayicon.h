//
//  trayicon.h
//  FerroUI.Native.OSX
//
//  Created by Dan Walmsley on 09/09/2021.
//

#ifndef trayicon_h
#define trayicon_h

#include "common.h"

class FrnTrayIcon : public ComSingleObject<IFrnTrayIcon, &IID_IFrnTrayIcon>
{
private:
    NSStatusItem* _native;
    bool _isTemplateIcon;

public:
    FORWARD_IUNKNOWN()
    
    FrnTrayIcon();
    
    ~FrnTrayIcon ();
    
    virtual HRESULT SetIcon (void* data, size_t length) override;
    
    virtual HRESULT SetMenu (IFrnMenu* menu) override;
    
    virtual HRESULT SetIsVisible (bool isVisible) override;

    virtual HRESULT SetToolTipText (char* text) override;

    virtual HRESULT SetIsTemplateIcon (bool isTemplateIcon) override;
};

#endif /* trayicon_h */
