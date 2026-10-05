use crate::media::fonts::tables::big_endian_binary_reader::FontTableError;
use crate::media::fonts::tables::platform_id::PlatformID;
use crate::utilities::ReadOnlyMemory;

use super::cmap_encoding::CmapEncoding;
use super::cmap_format::CmapFormat;

/// Representation of a subtable entry in the 'cmap' table directory.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct CmapSubtableEntry {
    pub platform: PlatformID,
    pub encoding: CmapEncoding,
    pub offset: i32,
    pub format: CmapFormat,
}

impl CmapSubtableEntry {
    pub const fn new(platform: PlatformID, encoding: CmapEncoding, offset: i32, format: CmapFormat) -> Self {
        Self { platform, encoding, offset, format }
    }

    /// The part of the `cmap` table that starts at this entry's offset.
    pub fn get_subtable_memory(&self, table: &ReadOnlyMemory<u8>) -> Result<ReadOnlyMemory<u8>, FontTableError> {
        if self.offset < 0 || self.offset as usize > table.len() {
            return Err(FontTableError::out_of_range("start"));
        }

        Ok(table.slice_from(self.offset as usize))
    }
}
