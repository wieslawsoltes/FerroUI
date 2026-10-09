use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{self, Read};

use crate::media::fonts::OpenTypeTag;
use crate::media::IFontMemory;
use crate::utilities::ReadOnlyMemory;

/// Owns the bytes of a font file and serves its OpenType tables.
///
/// Upstream this type copies the font into unmanaged memory and hands out
/// pinnable views of it. Here the font is an owned, reference counted buffer
/// ([`ReadOnlyMemory<u8>`]): tables are views sharing that buffer, so nothing
/// needs pinning (`Pin`/`Unpin` and the pin count have no counterpart) and a
/// table obtained before [`dispose`](IFontMemory::dispose) stays valid.
#[allow(dead_code)] // used by font backends, which are separate crates
pub(crate) struct UnmanagedFontMemory {
    /// The font data; empty after disposal.
    memory: RefCell<ReadOnlyMemory<u8>>,
    table_cache: RefCell<HashMap<OpenTypeTag, ReadOnlyMemory<u8>>>,
}

#[allow(dead_code)] // used by font backends, which are separate crates
impl UnmanagedFontMemory {
    fn new(memory: ReadOnlyMemory<u8>) -> Self {
        Self { memory: RefCell::new(memory), table_cache: RefCell::new(HashMap::new()) }
    }

    /// Loads a font from a stream. The stream is read to its end.
    pub fn load_from_stream(stream: &mut dyn Read) -> io::Result<UnmanagedFontMemory> {
        let mut data = Vec::new();

        stream.read_to_end(&mut data)?;

        Ok(Self::create_from_bytes(data))
    }

    /// Creates a font memory owning `data`.
    pub fn create_from_bytes(data: Vec<u8>) -> UnmanagedFontMemory {
        Self::new(ReadOnlyMemory::from_vec(data))
    }

    /// The font data (C# `Memory`); empty after disposal.
    pub fn memory(&self) -> ReadOnlyMemory<u8> {
        self.memory.borrow().clone()
    }

    /// The font data (C# `GetSpan`); empty after disposal.
    pub fn get_span(&self) -> ReadOnlyMemory<u8> {
        self.memory()
    }
}

impl IFontMemory for UnmanagedFontMemory {
    /// Attempts to retrieve the memory region corresponding to the specified
    /// OpenType table tag by parsing the sfnt table directory.
    fn try_get_table(&self, tag: OpenTypeTag) -> Option<ReadOnlyMemory<u8>> {
        // Validate tag
        if tag == OpenTypeTag::NONE {
            return None;
        }

        let memory = self.memory.borrow();
        let font_data = memory.span();

        // Minimal SFNT header: 4 (sfnt) + 2 (numTables) + 6 (rest) = 12
        if font_data.len() < 12 {
            return None;
        }

        // Check cache first
        if let Some(cached) = self.table_cache.borrow().get(&tag) {
            return Some(cached.clone());
        }

        // Parse table directory
        let num_tables = u16::from_be_bytes([font_data[4], font_data[5]]) as usize;
        let records_start = 12;
        let required_directory_bytes = records_start + num_tables * 16;

        if font_data.len() < required_directory_bytes {
            return None;
        }

        let read_u32 = |offset: usize| {
            u32::from_be_bytes([font_data[offset], font_data[offset + 1], font_data[offset + 2], font_data[offset + 3]])
        };

        for i in 0..num_tables {
            let entry_offset = records_start + i * 16;
            let entry_tag = OpenTypeTag::new(read_u32(entry_offset));

            if entry_tag != tag {
                continue;
            }

            let offset = read_u32(entry_offset + 8) as u64;
            let length = read_u32(entry_offset + 12) as u64;

            // Bounds checks - ensure values fit within the data
            if offset + length > font_data.len() as u64 {
                return None;
            }

            let table = memory.slice(offset as usize, length as usize);

            // Cache the result for faster subsequent lookups
            self.table_cache.borrow_mut().insert(tag, table.clone());

            return Some(table);
        }

        None
    }

    fn dispose(&self) {
        *self.memory.borrow_mut() = ReadOnlyMemory::empty();
        self.table_cache.borrow_mut().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_font(tag: OpenTypeTag, table_data: &[u8]) -> Vec<u8> {
        const RECORDS_START: usize = 12;
        const NUM_TABLES: usize = 1;

        let directory_bytes = RECORDS_START + NUM_TABLES * 16; // 12 + 16 = 28
        let offset = directory_bytes;

        let mut result = vec![0u8; offset + table_data.len()];

        // Simple SFNT header (version 0x00010000)
        result[1] = 1;

        // numTables (big-endian); the rest of the header (6 bytes) is left as zero
        result[5] = 1;

        // Table record at offset 12; the checksum (4 bytes) is left as zero
        result[12..16].copy_from_slice(&tag.value().to_be_bytes());
        result[20..24].copy_from_slice(&(offset as u32).to_be_bytes());
        result[24..28].copy_from_slice(&(table_data.len() as u32).to_be_bytes());

        result[offset..].copy_from_slice(table_data);

        result
    }

    #[test]
    fn try_get_table_returns_table_data_when_exists() {
        let tag = OpenTypeTag::parse("test");
        let data = [1u8, 2, 3, 4, 5];
        let font = build_font(tag, &data);

        let mem = UnmanagedFontMemory::load_from_stream(&mut font.as_slice()).unwrap();

        let table = mem.try_get_table(tag).unwrap();

        assert_eq!(table.span(), data);

        // Second call should also succeed (cache path)
        let table2 = mem.try_get_table(tag).unwrap();

        assert_eq!(table.len(), table2.len());

        // Ensure both instances reference the same underlying memory
        assert!(std::ptr::eq(table.span().as_ptr(), table2.span().as_ptr()));

        mem.dispose();
    }

    #[test]
    fn try_get_table_returns_false_for_unknown_tag() {
        let tag = OpenTypeTag::parse("TEST");
        let other = OpenTypeTag::parse("OTHR");
        let font = build_font(tag, &[9, 8, 7]);

        let mem = UnmanagedFontMemory::load_from_stream(&mut font.as_slice()).unwrap();

        assert!(mem.try_get_table(other).is_none());
        assert!(mem.try_get_table(OpenTypeTag::NONE).is_none());
    }

    #[test]
    fn try_get_table_returns_false_for_invalid_font() {
        // Too short to be a valid SFNT
        let short_data = [0u8; 8];

        let mem = UnmanagedFontMemory::load_from_stream(&mut short_data.as_slice()).unwrap();

        assert!(mem.try_get_table(OpenTypeTag::parse("test")).is_none());
    }

    #[test]
    fn get_span_returns_underlying_data() {
        let tag = OpenTypeTag::parse("span");
        let table_data: Vec<u8> = (0..64).collect();
        let font = build_font(tag, &table_data);

        let mem = UnmanagedFontMemory::load_from_stream(&mut font.as_slice()).unwrap();

        let span = mem.get_span();

        assert_eq!(span.len(), font.len());
        assert_eq!(span.span(), font);
    }

    #[test]
    fn hostile_directories_give_none() {
        let tag = OpenTypeTag::parse("test");

        // The directory claims more records than the data holds.
        let mut font = build_font(tag, &[1, 2, 3]);
        font[4] = 0xFF;
        font[5] = 0xFF;
        assert!(UnmanagedFontMemory::create_from_bytes(font).try_get_table(tag).is_none());

        // The table lies outside of the data.
        let mut font = build_font(tag, &[1, 2, 3]);
        font[24..28].copy_from_slice(&u32::MAX.to_be_bytes());
        assert!(UnmanagedFontMemory::create_from_bytes(font).try_get_table(tag).is_none());

        let mut font = build_font(tag, &[1, 2, 3]);
        font[20..24].copy_from_slice(&u32::MAX.to_be_bytes());
        assert!(UnmanagedFontMemory::create_from_bytes(font).try_get_table(tag).is_none());
    }

    #[test]
    fn dispose_releases_the_data_and_keeps_served_tables_valid() {
        let tag = OpenTypeTag::parse("test");
        let font = build_font(tag, &[1, 2, 3]);

        let mem = UnmanagedFontMemory::load_from_stream(&mut font.as_slice()).unwrap();
        let table = mem.try_get_table(tag).unwrap();

        mem.dispose();

        assert!(mem.get_span().is_empty());
        assert!(mem.try_get_table(tag).is_none());
        assert_eq!(table.span(), [1, 2, 3]);

        // Disposing again is harmless.
        mem.dispose();
    }
}
