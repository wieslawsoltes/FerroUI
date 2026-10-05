//
// Created by Dan Walmsley on 04/05/2022.
//

#ifndef FERRO_NATIVE_OSX_INSWINDOWHOLDER_H
#define FERRO_NATIVE_OSX_INSWINDOWHOLDER_H

@class FrnView;

struct INSWindowHolder
{
    virtual NSWindow* _Nonnull GetNSWindow () = 0;
};

struct INSViewHolder
{
    virtual FrnView* _Nonnull GetNSView () = 0;
};

#endif //FERRO_NATIVE_OSX_INSWINDOWHOLDER_H
