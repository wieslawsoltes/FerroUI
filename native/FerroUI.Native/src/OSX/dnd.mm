#include "common.h"

extern FrnDragDropEffects ConvertDragDropEffects(NSDragOperation nsop)
{
    int effects = 0;
    if((nsop & NSDragOperationCopy) != 0)
        effects |= (int)FrnDragDropEffects::Copy;
    if((nsop & NSDragOperationMove) != 0)
        effects |= (int)FrnDragDropEffects::Move;
    if((nsop & NSDragOperationLink) != 0)
        effects |= (int)FrnDragDropEffects::Link;
    return (FrnDragDropEffects)effects;
};

extern NSString* GetFrnCustomDataType()
{
    static NSString* result = nil;
    
    if (result == nil)
    {
        const size_t bufferSize = 256;
        char buffer[bufferSize];
        snprintf(buffer, bufferSize, "net.ferroui.inproc.uti.n%in", getpid());
        result = [NSString stringWithUTF8String:buffer];
    }
    
    return result;
}

@interface FrnDndSource : NSObject<NSDraggingSource>

@end

@implementation FrnDndSource
{
    NSDragOperation _operation;
    ComPtr<IFrnDndResultCallback> _cb;
    void* _sourceHandle;
};

- (NSDragOperation)draggingSession:(nonnull NSDraggingSession *)session sourceOperationMaskForDraggingContext:(NSDraggingContext)context
{
    return _operation;
}

- (FrnDndSource*) initWithOperation: (NSDragOperation)operation
                        andCallback: (IFrnDndResultCallback*) cb
                    andSourceHandle: (void*) handle
{
    self = [super init];
    _operation = operation;
    _cb = cb;
    _sourceHandle = handle;
    return self;
}

- (void)draggingSession:(NSDraggingSession *)session
           endedAtPoint:(NSPoint)screenPoint
              operation:(NSDragOperation)operation
{
    if(_cb != nil)
    {
        auto cb = _cb;
        _cb = nil;
        cb->OnDragAndDropComplete(ConvertDragDropEffects(operation));
    }
    if(_sourceHandle != nil)
    {
        FreeFrnGCHandle(_sourceHandle);
        _sourceHandle = nil;
    }
}

- (void*) gcHandle
{
    return _sourceHandle;
}

@end

extern NSObject<NSDraggingSource>* CreateDraggingSource(NSDragOperation op, IFrnDndResultCallback* cb, void* handle)
{
    return [[FrnDndSource alloc] initWithOperation:op andCallback:cb andSourceHandle:handle];
};

extern void* GetFrnDataObjectHandleFromDraggingInfo(NSObject<NSDraggingInfo>* info)
{
    id obj = [info draggingSource];
    if(obj == nil)
        return nil;
    if([obj isKindOfClass: [FrnDndSource class]])
    {
        auto src = (FrnDndSource*)obj;
        return [src gcHandle];
    }
    return nil;
}
