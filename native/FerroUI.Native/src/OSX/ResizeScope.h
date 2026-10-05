//
// Created by Dan Walmsley on 04/05/2022.
//

#ifndef FERRO_NATIVE_OSX_RESIZESCOPE_H
#define FERRO_NATIVE_OSX_RESIZESCOPE_H

#include "ferro-native.h"

@class FrnView;

class ResizeScope
{
public:
    ResizeScope(FrnView* _Nonnull view, FrnPlatformResizeReason reason);

    ~ResizeScope();
private:
    FrnView* _Nonnull _view;
    FrnPlatformResizeReason _restore;
};

#endif //FERRO_NATIVE_OSX_RESIZESCOPE_H
