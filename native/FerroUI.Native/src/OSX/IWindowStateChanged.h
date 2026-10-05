//
// Created by Dan Walmsley on 04/05/2022.
//

#ifndef FERRO_NATIVE_OSX_IWINDOWSTATECHANGED_H
#define FERRO_NATIVE_OSX_IWINDOWSTATECHANGED_H

struct IWindowStateChanged: public IUnknown
{
    virtual void WindowStateChanged () = 0;
    virtual void StartStateTransition () = 0;
    virtual void EndStateTransition () = 0;
    virtual SystemDecorations Decorations () = 0;
    virtual FrnWindowState WindowState () = 0;
};

#endif //FERRO_NATIVE_OSX_IWINDOWSTATECHANGED_H
