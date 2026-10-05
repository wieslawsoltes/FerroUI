use crate::media::fonts::OpenTypeTag;
use crate::utilities::ReadOnlyMemory;

/// Represents a memory manager for font data.
pub trait IFontMemory {
    /// Attempts to retrieve the memory region corresponding to the specified
    /// OpenType table tag.
    ///
    /// Returns `None` when the table is not present.
    fn try_get_table(&self, tag: OpenTypeTag) -> Option<ReadOnlyMemory<u8>>;

    /// Releases the font data (C# `IDisposable.Dispose`).
    fn dispose(&self);
}
