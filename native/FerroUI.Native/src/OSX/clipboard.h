#pragma once

#include "common.h"

@interface WriteableClipboardItem : NSObject <NSPasteboardWriting>
- (nonnull instancetype) initWithItem:(const ComPtr<IFrnClipboardDataItem>&)item source:(const ComPtr<IFrnClipboardDataSource>&)source;
@end
