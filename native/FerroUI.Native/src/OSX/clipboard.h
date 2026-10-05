#pragma once

#include "common.h"

@interface WriteableClipboardItem : NSObject <NSPasteboardWriting>
- (nonnull instancetype) initWithItem:(nonnull IFrnClipboardDataItem*)item source:(nonnull IFrnClipboardDataSource*)source;
@end
