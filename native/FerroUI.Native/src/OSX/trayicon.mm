#include "common.h"
#include "trayicon.h"
#include "menu.h"

extern ComPtr<IFrnTrayIcon> CreateTrayIcon()
{
    return comnew<FrnTrayIcon>();
}

FrnTrayIcon::FrnTrayIcon()
{
    _native = [[NSStatusBar systemStatusBar] statusItemWithLength: NSSquareStatusItemLength];
    
}

FrnTrayIcon::~FrnTrayIcon()
{
    if(_native != nullptr)
    {
        [[_native statusBar] removeStatusItem:_native];
        _native = nullptr;
    }
}

HRESULT FrnTrayIcon::SetIcon (void* data, size_t length)
{
    @autoreleasepool
    {
        if(data != nullptr)
        {
            NSData *imageData = [NSData dataWithBytes:data length:length];
            NSImage *image = [[NSImage alloc] initWithData:imageData];
            
            NSSize originalSize = [image size];
             
            NSSize size;
            size.height = floor([[NSFont menuFontOfSize:0] pointSize] * 1.333333);

            auto scaleFactor = size.height / originalSize.height;
            size.width = floor(originalSize.width * scaleFactor);
            
            [image setSize: size];
            [image setTemplate: _isTemplateIcon];
            [_native setImage:image];
        }
        else
        {
            [_native setImage:nullptr];
        }
        return S_OK;
    }
}

HRESULT FrnTrayIcon::SetMenu (const ComPtr<IFrnMenu>& menu)
{
    @autoreleasepool
    {
        auto appMenu = menu.dynamicCast<FrnAppMenu>();
        
        if(appMenu != nullptr)
        {
            [_native setMenu:appMenu->GetNative()];	
        }
    }
    
    return  S_OK;
}

HRESULT FrnTrayIcon::SetIsVisible(bool isVisible)
{
    @autoreleasepool
    {
        [_native setVisible:isVisible];
    }
    
    return  S_OK;
}

HRESULT FrnTrayIcon::SetToolTipText(char* text)
{
    @autoreleasepool
    {
        if (text != nullptr)
        {
            [[_native button] setToolTip:[NSString stringWithUTF8String:(const char*)text]];
        }
    }
    
    return  S_OK;
}

HRESULT FrnTrayIcon::SetIsTemplateIcon(bool isTemplateIcon)
{
    @autoreleasepool
    {
        if (_isTemplateIcon != isTemplateIcon)
        {
            _isTemplateIcon = isTemplateIcon;

            NSImage *image = [_native image];
            if (image)
            {
                [image setTemplate: _isTemplateIcon];
            }
        }
    }
    
    return  S_OK;
}