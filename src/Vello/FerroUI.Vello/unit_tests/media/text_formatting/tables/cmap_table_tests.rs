//! Port of upstream's `Media/TextFormatting/Tables/CmapTableTests.cs` of the
//! Skia unit tests, with its `CmapTestHelper`.
//!
//! The indexer `cmap[cp]` is `get_glyph`; `BuildFormat4Subtable` panics
//! where upstream throws `ArgumentException`.

use ferroui_base::media::fonts::tables::cmap::CmapFormat4Table;
use ferroui_base::utilities::ReadOnlyMemory;

#[test]
fn build_format4_subtable_should_map_range() {
    // Build a subtable mapping U+0030–U+0039 (digits 0–9) to glyphs 1–10
    let subtable = CmapTestHelper::build_format4_subtable(0x0030, 0x0039, 1);

    let cmap = CmapFormat4Table::new(ReadOnlyMemory::from_vec(subtable)).unwrap();

    for i in 0..10 {
        let cp = 0x30 + i;
        let glyph = cmap.get_glyph(cp);
        let expected_glyph = (i + 1) as u16;
        assert_eq!(expected_glyph, glyph, "U+{cp:04X}");
    }

    // Outside range should map to 0
    assert_eq!(0u16, cmap.get_glyph(0x0041)); // 'A'
}

pub(crate) struct CmapTestHelper;

impl CmapTestHelper {
    /// Builds a Format 4 subtable for a TrueType font's 'cmap' table, which maps a range of
    /// character codes to glyph indices.
    ///
    /// The Format 4 subtable is used in TrueType fonts to define mappings from character
    /// codes to glyph indices for a contiguous range of character codes. This method generates
    /// a minimal Format 4 subtable with one segment for the specified range and a sentinel
    /// segment, as required by the TrueType specification.
    ///
    /// The generated subtable includes the necessary header fields, segment arrays, and delta
    /// values to ensure that the specified range of character codes maps correctly to the
    /// corresponding glyph indices.
    ///
    /// `first_glyph_id` is the glyph index corresponding to `start_code`; subsequent character
    /// codes in the range map to consecutive glyph indices (upstream's default is 1). Returns
    /// the bytes of the subtable, which can be embedded in a TrueType font's 'cmap' table.
    ///
    /// # Panics
    ///
    /// When `end_code` is less than `start_code` (upstream throws `ArgumentException`).
    pub(crate) fn build_format4_subtable(start_code: u16, end_code: u16, first_glyph_id: u16) -> Vec<u8> {
        if end_code < start_code {
            panic!("endCode must be >= startCode");
        }

        // We will build exactly one real segment + sentinel
        let seg_count: u16 = 2; // one real + one sentinel
        let seg_count_x2 = seg_count * 2;

        // Correct search parameters (searchRange = 2 * (2^floor(log2(segCount))))
        let mut highest_power_of_two: i32 = 1;
        while highest_power_of_two * 2 <= seg_count as i32 {
            highest_power_of_two *= 2;
        }
        let search_range = (2 * highest_power_of_two) as u16;
        let entry_selector = (highest_power_of_two as f64).log2() as u16;
        let range_shift = seg_count_x2.wrapping_sub(search_range);

        // idDelta so that startCode maps to firstGlyphId
        let id_delta = first_glyph_id.wrapping_sub(start_code) as i16;

        // Calculate length: header (14) + endCode(segCount*2) + reservedPad(2) + startCode(segCount*2)
        // + idDelta(segCount*2) + idRangeOffset(segCount*2) + (no glyphIdArray)
        let header_size: i32 = 14;
        let seg_arrays_size: i32 = seg_count as i32 * 2 /*endCode*/ + 2 /*reservedPad*/ + seg_count as i32 * 2 /*startCode*/
            + seg_count as i32 * 2 /*idDelta*/ + seg_count as i32 * 2 /*idRangeOffset*/;
        let length = header_size + seg_arrays_size;

        let mut buffer = Vec::with_capacity(length as usize);

        let write_uint16 = |buffer: &mut Vec<u8>, v: u16| buffer.extend_from_slice(&v.to_be_bytes());
        let write_int16 = |buffer: &mut Vec<u8>, v: i16| buffer.extend_from_slice(&v.to_be_bytes());

        // Header
        write_uint16(&mut buffer, 4); // format
        write_uint16(&mut buffer, length as u16); // length
        write_uint16(&mut buffer, 0); // language
        write_uint16(&mut buffer, seg_count_x2);
        write_uint16(&mut buffer, search_range);
        write_uint16(&mut buffer, entry_selector);
        write_uint16(&mut buffer, range_shift);

        // endCode[] (one real segment then sentinel)
        write_uint16(&mut buffer, end_code);
        write_uint16(&mut buffer, 0xFFFF);

        write_uint16(&mut buffer, 0); // reservedPad

        // startCode[]
        write_uint16(&mut buffer, start_code);
        write_uint16(&mut buffer, 0xFFFF);

        // idDelta[]
        write_int16(&mut buffer, id_delta);
        write_int16(&mut buffer, 1); // sentinel delta (commonly 1)

        // idRangeOffset[]
        write_uint16(&mut buffer, 0);
        write_uint16(&mut buffer, 0);

        buffer
    }
}
