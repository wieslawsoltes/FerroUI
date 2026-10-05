use crate::media::fonts::tables::big_endian_binary_reader::{BigEndianBinaryReader, FontTableError};
use crate::utilities::ReadOnlyMemory;

use super::codepoint_range::CodepointRange;

/// A format 4 `cmap` subtable (segment mapping to delta values).
///
/// Lookups read the table data lazily and never allocate.
#[derive(Clone, Debug)]
pub struct CmapFormat4Table {
    seg_count: i32,
    end_codes: ReadOnlyMemory<u8>,
    start_codes: ReadOnlyMemory<u8>,
    id_deltas: ReadOnlyMemory<u8>,
    id_range_offsets: ReadOnlyMemory<u8>,
    glyph_id_array: ReadOnlyMemory<u8>,
    language: u16,
}

impl CmapFormat4Table {
    /// Parses the subtable header; `Err` when the header is truncated.
    pub fn new(table: ReadOnlyMemory<u8>) -> Result<Self, FontTableError> {
        let mut reader = BigEndianBinaryReader::new(table.span());

        let format = reader.read_uint16()?; // must be 4

        debug_assert!(format == 4, "Format must be 4.");

        let length = reader.read_uint16()?; // length in bytes of this subtable

        let language = reader.read_uint16()?; // language code, 0 for non-language-specific

        let seg_count_x2 = reader.read_uint16()?; // 2 * segCount
        let mut seg_count = (seg_count_x2 / 2) as i32;

        reader.read_uint16()?; // searchRange = 2 * (2^floor(log2(segCount)))
        reader.read_uint16()?; // entrySelector = log2(searchRange/2)
        reader.read_uint16()?; // rangeShift = segCountX2 - searchRange

        // Compute offsets
        let end_code_offset = reader.position();

        // Clamp the declared length to the buffer (and to at least the header just read):
        // a corrupt/short length must not produce an out-of-range slice. Mirrors the
        // format-12 clamp. (The header reads above guarantee the buffer holds the header.)
        let table_length = (length as i32).max(end_code_offset).min(table.len().min(i32::MAX as usize) as i32);

        let table = table.slice(0, table_length as usize);

        // Clamp the segment count to what the (length-bounded) table actually holds — the
        // four parallel arrays below take segCount * 8 bytes (+ 2 for reservedPad), and a
        // hostile segCount must not produce an out-of-range slice.
        let max_seg_count = ((table_length - end_code_offset - 2) / 8).max(0);
        seg_count = seg_count.min(max_seg_count);

        // Clamp each derived offset to tableLength so that zero-length slices (e.g. when
        // segCount was driven to 0 by a too-short declared length) always start at a valid
        // position. The reservedPad (+2) between endCodes and startCodes is what makes
        // startCodeOffset land past the end of a 14-byte clamped table without this guard.
        let start_code_offset = (end_code_offset + seg_count * 2 + 2).min(table_length); // + reservedPad
        let id_delta_offset = (start_code_offset + seg_count * 2).min(table_length); // after startCodes
        let id_range_offset_offset = (id_delta_offset + seg_count * 2).min(table_length); // after idDeltas
        let glyph_id_array_offset = (id_range_offset_offset + seg_count * 2).min(table_length); // after idRangeOffsets

        let segment_bytes = (seg_count * 2) as usize;

        // Slice directly
        let end_codes = table.slice(end_code_offset as usize, segment_bytes);
        let start_codes = table.slice(start_code_offset as usize, segment_bytes);
        let id_deltas = table.slice(id_delta_offset as usize, segment_bytes);
        let id_range_offsets = table.slice(id_range_offset_offset as usize, segment_bytes);

        // Whatever remains belongs to glyphIdArray; a truncated table yields a shorter
        // (possibly empty) array and lookups bounds-check against it.
        let glyph_count = ((table_length - glyph_id_array_offset) / 2).max(0);

        let glyph_id_array = table.slice(glyph_id_array_offset as usize, (glyph_count * 2) as usize);

        Ok(Self { seg_count, end_codes, start_codes, id_deltas, id_range_offsets, glyph_id_array, language })
    }

    pub fn language(&self) -> u16 {
        self.language
    }

    /// The glyph for the code point, 0 when it is not mapped.
    #[inline]
    pub fn get_glyph(&self, code_point: i32) -> u16 {
        // Find the segment containing the code point
        let segment_index = self.find_segment_index(code_point);

        if segment_index < 0 {
            return 0;
        }

        Self::map_glyph(
            self.seg_count,
            segment_index,
            code_point,
            self.start_codes.span(),
            self.id_deltas.span(),
            self.id_range_offsets.span(),
            self.glyph_id_array.span(),
        )
    }

    pub fn contains_glyph(&self, code_point: i32) -> bool {
        let seg = self.find_segment_index(code_point);

        if seg < 0 || seg >= self.seg_count {
            return false;
        }

        let id_range_offset = read_uint16_be(self.id_range_offsets.span(), seg);
        let id_delta = read_uint16_be(self.id_deltas.span(), seg);

        if id_range_offset == 0 {
            // Always maps to something (possibly .notdef via delta)
            return (code_point.wrapping_add(id_delta as i32) & 0xFFFF) != 0;
        }

        let start = read_uint16_be(self.start_codes.span(), seg) as i32;
        let ro = (id_range_offset >> 1) as i32;
        let idx = (code_point - start) + ro - (self.seg_count - seg);

        if idx < 0 || idx >= (self.glyph_id_array.len() >> 1) as i32 {
            return false;
        }

        read_uint16_be(self.glyph_id_array.span(), idx) != 0
    }

    /// Maps every code point to its glyph (0 when unmapped).
    ///
    /// Panics when `glyph_ids` is shorter than `code_points`.
    pub fn get_glyphs(&self, code_points: &[i32], glyph_ids: &mut [u16]) {
        assert!(
            glyph_ids.len() >= code_points.len(),
            "Output span must be at least as long as input span (Parameter 'glyphIds')"
        );

        // Cache all spans once
        let start_codes = self.start_codes.span();
        let end_codes = self.end_codes.span();
        let id_deltas = self.id_deltas.span();
        let id_range_offsets = self.id_range_offsets.span();
        let glyph_id_array = self.glyph_id_array.span();

        // Track last segment for locality optimization
        let mut last_segment: i32 = -1;

        for (i, &code_point) in code_points.iter().enumerate() {
            let mut segment_index = -1;

            // Optimization: check if codepoint is in the same segment as previous
            if last_segment >= 0 && last_segment < self.seg_count {
                let last_start = read_uint16_be(start_codes, last_segment) as i32;
                let last_end = read_uint16_be(end_codes, last_segment) as i32;

                if code_point >= last_start && code_point <= last_end {
                    segment_index = last_segment;
                }
            }

            if segment_index < 0 {
                // Binary search for segment
                segment_index = Self::find_segment_index_in(self.seg_count, code_point, start_codes, end_codes);

                if segment_index < 0 {
                    glyph_ids[i] = 0;
                    continue;
                }

                last_segment = segment_index;
            }

            glyph_ids[i] = Self::map_glyph(
                self.seg_count,
                segment_index,
                code_point,
                start_codes,
                id_deltas,
                id_range_offsets,
                glyph_id_array,
            );
        }
    }

    /// The glyph for the code point; `None` when it is not mapped or maps to
    /// glyph 0.
    pub fn try_get_glyph(&self, code_point: i32) -> Option<u16> {
        let seg = self.find_segment_index(code_point);

        if seg < 0 || seg >= self.seg_count {
            return None;
        }

        let glyph_id = Self::map_glyph(
            self.seg_count,
            seg,
            code_point,
            self.start_codes.span(),
            self.id_deltas.span(),
            self.id_range_offsets.span(),
            self.glyph_id_array.span(),
        );

        if glyph_id != 0 {
            Some(glyph_id)
        } else {
            None
        }
    }

    pub(crate) fn try_get_range(&self, index: i32) -> Option<CodepointRange> {
        if index < 0 || index >= self.seg_count {
            return None;
        }

        let start = read_uint16_be(self.start_codes.span(), index) as i32;
        let end = read_uint16_be(self.end_codes.span(), index) as i32;

        // Skip sentinel segment (0xFFFF)
        if start == 0xFFFF && end == 0xFFFF {
            return None;
        }

        Some(CodepointRange::new(start, end))
    }

    /// Resolves the glyph ID for a given code point within a specific segment.
    #[inline]
    fn map_glyph(
        seg_count: i32,
        segment_index: i32,
        code_point: i32,
        start_codes: &[u8],
        id_deltas: &[u8],
        id_range_offsets: &[u8],
        glyph_id_array: &[u8],
    ) -> u16 {
        let id_range_offset = read_uint16_be(id_range_offsets, segment_index);
        let id_delta = read_uint16_be(id_deltas, segment_index);

        // If idRangeOffset is 0, glyphId = (codePoint + idDelta) % 65536
        if id_range_offset == 0 {
            return (code_point.wrapping_add(id_delta as i32) & 0xFFFF) as u16;
        }

        let start = read_uint16_be(start_codes, segment_index) as i32;
        let ro = (id_range_offset / 2) as i32; // words
        // The index into the glyphIdArray
        let idx = (code_point - start) + ro - (seg_count - segment_index);

        // Ensure index is within bounds of glyphIdArray
        let glyph_array_words = (glyph_id_array.len() / 2) as i32;

        if idx >= 0 && idx < glyph_array_words {
            let glyph_id = read_uint16_be(glyph_id_array, idx);

            // If glyphId is not 0, apply idDelta
            if glyph_id != 0 {
                return glyph_id.wrapping_add(id_delta);
            }

            return glyph_id;
        }

        // Not found or maps to missing glyph
        0
    }

    #[inline]
    fn find_segment_index(&self, code_point: i32) -> i32 {
        Self::find_segment_index_in(self.seg_count, code_point, self.start_codes.span(), self.end_codes.span())
    }

    /// Binary search over endCodes (sorted ascending) that works directly with cached spans.
    #[inline]
    fn find_segment_index_in(seg_count: i32, code_point: i32, start_codes: &[u8], end_codes: &[u8]) -> i32 {
        let mut lo = 0;
        let mut hi = seg_count - 1;

        while lo <= hi {
            let mid = (lo + hi) >> 1;
            let end = read_uint16_be(end_codes, mid) as i32;

            if code_point > end {
                lo = mid + 1;
            } else {
                hi = mid - 1;
            }
        }

        // lo is now the first segment whose endCode >= codePoint
        if lo < seg_count {
            let start = read_uint16_be(start_codes, lo) as i32;

            if code_point >= start {
                return lo;
            }
        }

        -1 // not found
    }
}

/// Reads a big-endian `u16` from the specified word index; out of range
/// indices (which the callers rule out) read as 0.
#[inline]
fn read_uint16_be(span: &[u8], word_index: i32) -> u16 {
    let byte_index = word_index as usize * 2;

    match span.get(byte_index..byte_index + 2) {
        Some(bytes) => u16::from_be_bytes([bytes[0], bytes[1]]),
        None => 0,
    }
}
