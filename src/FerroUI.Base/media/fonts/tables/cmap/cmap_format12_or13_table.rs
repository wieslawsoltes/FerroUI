use crate::media::fonts::tables::big_endian_binary_reader::{BigEndianBinaryReader, FontTableError};
use crate::utilities::ReadOnlyMemory;

use super::cmap_format::CmapFormat;
use super::codepoint_range::CodepointRange;

/// A format 12 (segmented coverage) or format 13 (many-to-one) `cmap` subtable.
///
/// Lookups read the table data lazily and never allocate.
#[derive(Clone, Debug)]
pub struct CmapFormat12Or13Table {
    group_count: i32,
    groups: ReadOnlyMemory<u8>,
    format: CmapFormat,
    language: u32,
}

impl CmapFormat12Or13Table {
    /// Parses the subtable header; `Err` when the header is truncated.
    pub fn new(table: ReadOnlyMemory<u8>) -> Result<Self, FontTableError> {
        let mut reader = BigEndianBinaryReader::new(table.span());

        let format = reader.read_uint16()?;
        debug_assert!(format == 12 || format == 13, "Format must be 12 or 13.");
        let format = CmapFormat(format);

        reader.read_uint16()?; // reserved, must be 0

        // Clamp the declared length to the buffer: a corrupt/huge length must not produce an
        // out-of-range slice.
        let length = reader.read_uint32()?;

        let language = reader.read_uint32()?;

        let declared_group_count = reader.read_uint32()?;

        let groups_offset = reader.position() as usize;
        let minimum_table_length = groups_offset.min(table.len());
        let declared_table_length = (length as u64).min(table.len() as u64) as usize;
        let table_length = declared_table_length.max(minimum_table_length);

        let table = table.slice(0, table_length);

        // Each SequentialMapGroup is 12 bytes. Clamp the group count to what the (length-bounded)
        // table actually holds, so a hostile count cannot produce an out-of-range slice.
        let max_groups = if table.len() > groups_offset { (table.len() - groups_offset) / 12 } else { 0 };
        let group_count = (declared_group_count as u64).min(max_groups as u64).min(i32::MAX as u64) as usize;

        let groups = table.slice(groups_offset, group_count * 12);

        Ok(Self { group_count: group_count as i32, groups, format, language })
    }

    pub fn format(&self) -> CmapFormat {
        self.format
    }

    pub fn language(&self) -> u32 {
        self.language
    }

    /// The glyph for the code point, 0 when it is not mapped.
    #[inline]
    pub fn get_glyph(&self, code_point: i32) -> u16 {
        let groups = self.groups.span();
        let group_index = Self::find_group_index_in(self.group_count, code_point, groups);

        if group_index < 0 {
            return 0;
        }

        let start = read_uint32_be(groups, group_index, 0);
        let start_glyph = read_uint32_be(groups, group_index, 8);

        self.calc_effective_glyph(code_point, start, start_glyph)
    }

    #[inline]
    pub fn contains_glyph(&self, code_point: i32) -> bool {
        Self::find_group_index_in(self.group_count, code_point, self.groups.span()) >= 0
    }

    /// Maps every code point to its glyph (0 when unmapped).
    ///
    /// Panics when `glyph_ids` is shorter than `code_points`.
    pub fn get_glyphs(&self, code_points: &[i32], glyph_ids: &mut [u16]) {
        assert!(
            glyph_ids.len() >= code_points.len(),
            "Output span must be at least as long as input span (Parameter 'glyphIds')"
        );

        let groups = self.groups.span();

        // Track last group for locality optimization
        let mut last_group: i32 = -1;
        let mut last_start: u32 = 0;
        let mut last_end: u32 = 0;
        let mut last_start_glyph: u32 = 0;

        for (i, &code_point) in code_points.iter().enumerate() {
            // Optimization: check if codepoint is in the same group as previous
            if last_group >= 0 && code_point as i64 >= last_start as i64 && code_point as i64 <= last_end as i64 {
                glyph_ids[i] = self.calc_effective_glyph(code_point, last_start, last_start_glyph);
                continue;
            }

            // Binary search for group
            let group_index = Self::find_group_index_in(self.group_count, code_point, groups);

            if group_index < 0 {
                glyph_ids[i] = 0;
                last_group = -1;

                continue;
            }

            // Cache group data
            last_group = group_index;
            last_start = read_uint32_be(groups, group_index, 0);
            last_end = read_uint32_be(groups, group_index, 4);
            last_start_glyph = read_uint32_be(groups, group_index, 8);

            glyph_ids[i] = self.calc_effective_glyph(code_point, last_start, last_start_glyph);
        }
    }

    /// The glyph for the code point; `None` when it is not mapped or maps to
    /// glyph 0.
    pub fn try_get_glyph(&self, code_point: i32) -> Option<u16> {
        match self.get_glyph(code_point) {
            0 => None,
            glyph_id => Some(glyph_id),
        }
    }

    pub(crate) fn try_get_range(&self, index: i32) -> Option<CodepointRange> {
        if index < 0 || index >= self.group_count {
            return None;
        }

        let groups = self.groups.span();

        let start = read_uint32_be(groups, index, 0);
        let end = read_uint32_be(groups, index, 4);

        // Group contents are attacker-controlled: consumers iterate the returned range one
        // codepoint at a time, so an end beyond the Unicode range must not get through. Clamp to
        // the Unicode range and map inverted/out-of-range groups to an empty range so that
        // enumeration continues with the remaining groups.
        const MAX_CODEPOINT: u32 = 0x10FFFF;

        if start > end || start > MAX_CODEPOINT {
            return Some(CodepointRange::new(0, -1));
        }

        Some(CodepointRange::new(start as i32, end.min(MAX_CODEPOINT) as i32))
    }

    #[inline]
    fn calc_effective_glyph(&self, code_point: i32, start: u32, start_glyph: u32) -> u16 {
        // Format 13, all codepoints in the group map to a single glyph
        if self.format == CmapFormat::Format13 {
            return start_glyph as u16;
        }

        // Format 12, calculate glyph index
        (start_glyph as i64 + (code_point as i64 - start as i64)) as u16
    }

    /// Binary search to find the group containing the code point.
    #[inline]
    fn find_group_index_in(group_count: i32, code_point: i32, groups: &[u8]) -> i32 {
        let code_point = code_point as i64;
        let mut lo = 0;
        let mut hi = group_count - 1;

        while lo <= hi {
            let mid = (lo + hi) >> 1;
            let start = read_uint32_be(groups, mid, 0) as i64;
            let end = read_uint32_be(groups, mid, 4) as i64;

            if code_point < start {
                hi = mid - 1;
            } else if code_point > end {
                lo = mid + 1;
            } else {
                return mid;
            }
        }

        // Not found
        -1
    }
}

/// Reads a big-endian `u32` field of a group; out of range indices (which the
/// callers rule out) read as 0.
#[inline]
fn read_uint32_be(span: &[u8], group_index: i32, field_offset: usize) -> u32 {
    let byte_index = group_index as usize * 12 + field_offset;

    match span.get(byte_index..byte_index + 4) {
        Some(bytes) => u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        None => 0,
    }
}
