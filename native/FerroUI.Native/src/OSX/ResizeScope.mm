#import <AppKit/AppKit.h>
#include "ResizeScope.h"
#include "FrnView.h"

ResizeScope::ResizeScope(FrnView *view, FrnPlatformResizeReason reason) {
    _view = view;
    _restore = [view getResizeReason];
    [view setResizeReason:reason];
}

ResizeScope::~ResizeScope() {
    [_view setResizeReason:_restore];
}
