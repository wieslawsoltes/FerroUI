//! Characterization tests for the robustness of the cmap parser, using the
//! synthetic font harness: cmap parsing must not overflow, hang or mis-select
//! a subtable.

use crate::media::fonts::tables::testing::{BigEndianBuffer, SyntheticFont};
use crate::media::fonts::tables::FontTableError;

use super::{CharacterToGlyphMap, CmapFormat, CmapTable, CodepointRange};

fn load(cmap: Vec<u8>) -> Result<CharacterToGlyphMap, FontTableError> {
    let mut font = SyntheticFont::new();
    font.replace("cmap", cmap);

    CmapTable::load(&font)
}

// ── cmap format 12 nGroups*12 overflow must not deny the whole font ──

#[test]
fn cmap_format12_ngroups_overflow_does_not_deny_the_font() {
    // numGroups = 0x20000000 would make `numGroups * 12` overflow a 32 bit integer.
    // The group count is clamped to what the table holds, so the bad
    // subtable yields empty coverage rather than failing — the font still loads.
    let map = load(build_cmap_with_overflowing_format12(0x2000_0000)).unwrap();

    assert_eq!(map.format(), CmapFormat::Format12);
    assert!(!map.contains_glyph('A' as i32));
    assert_eq!(map.get_glyph('A' as i32), 0);
    assert_eq!(map.get_mapped_ranges().count(), 0);
}

// ── cmap format 12 hostile group *contents* must not hang range enumeration ──

#[test]
fn cmap_format12_huge_group_end_is_clamped_to_the_unicode_range() {
    // endCharCode = 0x7FFFFFFF must not reach the per-codepoint dictionary loops unclamped.
    // The group is clamped to the Unicode range at the choke point.
    let map = load(build_format12_cmap(&[(0x41, 0x7FFF_FFFF, 1)])).unwrap();

    let dictionary = map.as_read_only_dictionary();

    // Terminates (clamped to ≤ 0x10FFFF) and still exposes the in-range part of the group.
    assert!(dictionary.count() > 0);
    assert!(dictionary.contains_key(0x41));
    assert_eq!(map.get_mapped_ranges().next(), Some(CodepointRange::new(0x41, 0x10FFFF)));
}

#[test]
fn cmap_format12_inverted_group_is_skipped_not_terminal() {
    // The second group is inverted (start > end). It must enumerate as an empty range —
    // not fail, not end enumeration early, and not hide the first (valid) group.
    let map = load(build_format12_cmap(&[(0x41, 0x41, 5), (0x1000, 0x100, 6), (0x2000, 0x2001, 7)])).unwrap();

    let dictionary = map.as_read_only_dictionary();

    assert_eq!(dictionary.count(), 3);
    assert!(dictionary.contains_key(0x41));
    assert_eq!(dictionary.keys().collect::<Vec<_>>(), [0x41, 0x2000, 0x2001]);
    assert_eq!(dictionary.values().collect::<Vec<_>>(), [5, 7, 8]);
    assert_eq!(dictionary.iter().collect::<Vec<_>>(), [(0x41, 5), (0x2000, 7), (0x2001, 8)]);

    let ranges: Vec<CodepointRange> = map.get_mapped_ranges().collect();

    assert_eq!(
        ranges,
        [CodepointRange::new(0x41, 0x41), CodepointRange::new(0, -1), CodepointRange::new(0x2000, 0x2001)]
    );
}

// ── cmap Format-4 subtable selection prefers Unicode over Symbol ──

#[test]
fn format4_subtable_selection_prefers_unicode_over_symbol() {
    // Two Windows-platform Format-4 subtables: a Symbol (encoding 0) one that maps only the
    // PUA codepoint 0xF041, and a Unicode-BMP (encoding 1) one that maps 'A'. The selection
    // scores the Symbol encoding worse than Unicode, so the Unicode subtable wins
    // regardless of directory order.
    for symbol_first in [true, false] {
        let map = load(build_dual_format4_cmap(symbol_first)).unwrap();

        // ASCII 'A' resolves in BOTH orderings — encoding, not directory order, decides.
        assert!(map.contains_glyph('A' as i32), "symbol_first = {symbol_first}");
        assert_eq!(map.get_glyph('A' as i32), 5);
        assert!(!map.contains_glyph(0xF041));
    }
}

// ── loader failures ──

#[test]
fn missing_cmap_table_is_an_error() {
    let error = match CmapTable::load(&SyntheticFont::new()) {
        Ok(_) => panic!("expected the load to fail"),
        Err(error) => error,
    };

    assert_eq!(error.to_string(), "No cmap table found.");
    assert!(error.is_invalid_operation());
}

#[test]
fn cmap_without_a_supported_subtable_is_an_error() {
    // A single format 6 subtable.
    let mut cmap = BigEndianBuffer::new();
    cmap.uint16(0).uint16(1).uint16(3).uint16(1).uint32(12);
    cmap.uint16(6).uint16(10).uint16(0).uint16(0x41).uint16(0);

    let error = load(cmap.to_array()).err().unwrap();

    assert_eq!(error.to_string(), "No suitable cmap subtable found.");

    // No subtables at all.
    let mut cmap = BigEndianBuffer::new();
    cmap.uint16(0).uint16(0);

    assert!(load(cmap.to_array()).is_err());
}

#[test]
fn malformed_cmap_directories_are_errors() {
    // Truncated header.
    assert!(load(vec![0, 0, 0]).err().unwrap().is_invalid_operation());

    // The directory promises a record that is missing.
    assert!(load(vec![0, 0, 0, 1]).err().unwrap().is_invalid_operation());

    // A subtable offset past the table (and one with the high bit set).
    for offset in [0x0000_FFFFu32, 0x8000_0000] {
        let mut cmap = BigEndianBuffer::new();
        cmap.uint16(0).uint16(1).uint16(3).uint16(1).uint32(offset);

        assert!(load(cmap.to_array()).err().unwrap().is_argument_out_of_range());
    }

    // The subtable header is cut short after the format field.
    let mut cmap = BigEndianBuffer::new();
    cmap.uint16(0).uint16(1).uint16(3).uint16(10).uint32(12).uint16(12).uint16(0);

    assert!(load(cmap.to_array()).err().unwrap().is_invalid_operation());

    let mut cmap = BigEndianBuffer::new();
    cmap.uint16(0).uint16(1).uint16(3).uint16(1).uint32(12).uint16(4).uint16(0);

    assert!(load(cmap.to_array()).err().unwrap().is_invalid_operation());
}

// ── lookups ──

#[test]
fn default_map_maps_nothing() {
    let map = CharacterToGlyphMap::default();

    assert_eq!(map.format(), CmapFormat::Format0);
    assert_eq!(map.get_glyph(0x41), 0);
    assert!(!map.contains_glyph(0x41));
    assert_eq!(map.try_get_glyph(0x41), None);
    assert_eq!(map.get_mapped_ranges().count(), 0);
    assert_eq!(map.as_read_only_dictionary().count(), 0);

    let mut glyphs = [7u16; 3];
    map.get_glyphs(&[1, 2], &mut glyphs);

    // The whole output is cleared.
    assert_eq!(glyphs, [0, 0, 0]);
}

#[test]
fn format4_lookups_cover_delta_and_glyph_array_segments() {
    let map = load(wrap_subtable(3, 1, &build_format4())).unwrap();

    assert_eq!(map.format(), CmapFormat::Format4);

    // Segment 0: 'A'..'C' through idDelta.
    assert_eq!(map.get_glyph(0x41), 10);
    assert_eq!(map.get_glyph(0x43), 12);
    // Segment 1: 0x50..0x53 through the glyph id array, with idDelta applied to non-zero entries.
    assert_eq!(map.get_glyph(0x50), 101);
    assert_eq!(map.get_glyph(0x51), 0);
    assert_eq!(map.get_glyph(0x52), 103);
    // The last entry lies past the (truncated) glyph id array.
    assert_eq!(map.get_glyph(0x53), 0);

    // Outside of every segment, including negative and astral code points.
    for code_point in [0, 0x40, 0x44, 0x4F, 0x54, 0xFFFF, 0x1_0041, -1, i32::MIN, i32::MAX] {
        assert_eq!(map.get_glyph(code_point), 0, "code point {code_point:#x}");
        assert!(!map.contains_glyph(code_point));
        assert_eq!(map.try_get_glyph(code_point), None);
    }

    assert!(map.contains_glyph(0x41));
    assert!(map.contains_glyph(0x50));
    assert!(!map.contains_glyph(0x51));
    assert!(!map.contains_glyph(0x53));

    assert_eq!(map.try_get_glyph(0x42), Some(11));
    assert_eq!(map.try_get_glyph(0x52), Some(103));
    assert_eq!(map.try_get_glyph(0x51), None);

    let code_points = [0x41, 0x42, 0x50, 0x52, 0x20, 0x43, 0x53, -5, 0x51, 0x41];
    let mut glyphs = [0xFFFFu16; 10];

    map.get_glyphs(&code_points, &mut glyphs);

    assert_eq!(glyphs, [10, 11, 101, 103, 0, 12, 0, 0, 0, 10]);

    for (code_point, glyph) in code_points.iter().zip(glyphs) {
        assert_eq!(map.get_glyph(*code_point), glyph);
    }

    // The sentinel segment ends the range enumeration.
    let ranges: Vec<CodepointRange> = map.get_mapped_ranges().collect();

    assert_eq!(ranges, [CodepointRange::new(0x41, 0x43), CodepointRange::new(0x50, 0x53)]);

    let dictionary = map.as_read_only_dictionary();

    assert_eq!(dictionary.count(), 5);
    assert_eq!(dictionary.count(), 5);
    assert_eq!(dictionary.keys().collect::<Vec<_>>(), [0x41, 0x42, 0x43, 0x50, 0x52]);
    assert_eq!(dictionary.values().collect::<Vec<_>>(), [10, 11, 12, 101, 103]);
    assert_eq!(dictionary.get(0x52), 103);
    assert_eq!(dictionary.try_get_value(0x51), None);
    assert!(!dictionary.contains_key(0x51));
}

#[test]
#[should_panic(expected = "was not found in the character map")]
fn dictionary_indexer_panics_for_unmapped_code_points() {
    let map = load(wrap_subtable(3, 1, &build_format4())).unwrap();

    map.as_read_only_dictionary().get(0x20);
}

#[test]
#[should_panic(expected = "Output span must be at least as long as input span")]
fn get_glyphs_panics_when_the_output_is_too_short() {
    let map = load(wrap_subtable(3, 1, &build_format4())).unwrap();

    map.get_glyphs(&[0x41, 0x42], &mut [0u16; 1]);
}

#[test]
fn format4_with_hostile_counts_and_lengths_maps_nothing() {
    // segCountX2 claims 0x7FFF segments in a table that only holds the header.
    let mut subtable = BigEndianBuffer::new();
    subtable.uint16(4).uint16(14).uint16(0).uint16(0xFFFE).uint16(0).uint16(0).uint16(0);

    let map = load(wrap_subtable(3, 1, &subtable.to_array())).unwrap();

    assert_eq!(map.get_glyph(0x41), 0);
    assert_eq!(map.get_mapped_ranges().count(), 0);

    // A declared length shorter than the header and one far past the buffer.
    for length in [0, 0xFFFF] {
        let mut subtable = build_format4();
        subtable[2..4].copy_from_slice(&(length as u16).to_be_bytes());

        let map = load(wrap_subtable(3, 1, &subtable)).unwrap();

        if length == 0 {
            assert_eq!(map.get_glyph(0x41), 0);
        } else {
            assert_eq!(map.get_glyph(0x41), 10);
            assert_eq!(map.get_glyph(0x52), 103);
        }
    }

    // Every truncation of a valid subtable (past the header) loads and never panics.
    let subtable = build_format4();

    for length in 14..subtable.len() {
        let map = load(wrap_subtable(3, 1, &subtable[..length])).unwrap();

        for code_point in [0x41, 0x50, 0x52, 0x53, 0xFFFF] {
            map.get_glyph(code_point);
            map.contains_glyph(code_point);
            map.try_get_glyph(code_point);
        }

        map.as_read_only_dictionary().count();
    }
}

#[test]
fn format12_lookups_and_batches() {
    let map = load(build_format12_cmap(&[(0x41, 0x5A, 10), (0x1F600, 0x1F60F, 500)])).unwrap();

    assert_eq!(map.format(), CmapFormat::Format12);
    assert_eq!(map.get_glyph(0x41), 10);
    assert_eq!(map.get_glyph(0x5A), 35);
    assert_eq!(map.get_glyph(0x1F603), 503);
    assert_eq!(map.get_glyph(0x5B), 0);
    assert_eq!(map.get_glyph(-1), 0);
    assert_eq!(map.get_glyph(i32::MAX), 0);
    assert!(map.contains_glyph(0x1F60F));
    assert!(!map.contains_glyph(0x1F610));
    assert_eq!(map.try_get_glyph(0x42), Some(11));
    assert_eq!(map.try_get_glyph(0x40), None);

    let mut glyphs = [0u16; 7];

    map.get_glyphs(&[0x41, 0x42, 0x20, 0x43, 0x1F600, 0x1F601, -7], &mut glyphs);

    assert_eq!(glyphs, [10, 11, 0, 12, 500, 501, 0]);
    assert_eq!(map.as_read_only_dictionary().count(), 26 + 16);
}

#[test]
fn format12_is_preferred_over_format4_and_format13_is_the_last_resort() {
    // Format 4 listed first, format 12 second: format 12 wins.
    let format4 = build_single_char_format4('A' as i32, 5);
    let format12 = build_format12_subtable(12, &[(0x41, 0x41, 9)]);
    let format13 = build_format12_subtable(13, &[(0x0, 0x10FFFF, 3)]);

    let map = load(wrap_subtables(&[(3, 1, &format4), (3, 10, &format12), (0, 6, &format13)])).unwrap();

    assert_eq!(map.format(), CmapFormat::Format12);
    assert_eq!(map.get_glyph(0x41), 9);

    // Format 4 beats format 13.
    let map = load(wrap_subtables(&[(0, 6, &format13), (3, 1, &format4)])).unwrap();

    assert_eq!(map.format(), CmapFormat::Format4);
    assert_eq!(map.get_glyph(0x41), 5);

    // Format 13 alone: every code point of a group maps to the same glyph.
    let map = load(wrap_subtables(&[(0, 6, &format13)])).unwrap();

    assert_eq!(map.format(), CmapFormat::Format13);
    assert_eq!(map.get_glyph(0x41), 3);
    assert_eq!(map.get_glyph(0x10FFFF), 3);
    assert_eq!(map.get_glyph(0x110000), 0);

    let mut glyphs = [0u16; 3];
    map.get_glyphs(&[0x20, 0x21, 0x1F600], &mut glyphs);

    assert_eq!(glyphs, [3, 3, 3]);
}

#[test]
fn format12_selection_prefers_the_unicode_platform_full_repertoire() {
    let windows_bmp = build_format12_subtable(12, &[(0x41, 0x41, 1)]);
    let windows_ucs4 = build_format12_subtable(12, &[(0x41, 0x41, 2)]);
    let unicode_full = build_format12_subtable(12, &[(0x41, 0x41, 3)]);
    let macintosh = build_format12_subtable(12, &[(0x41, 0x41, 4)]);

    let map = load(wrap_subtables(&[
        (1, 0, &macintosh),
        (3, 1, &windows_bmp),
        (3, 10, &windows_ucs4),
        (0, 4, &unicode_full),
    ]))
    .unwrap();

    assert_eq!(map.get_glyph(0x41), 3);

    let map = load(wrap_subtables(&[(1, 0, &macintosh), (3, 1, &windows_bmp), (3, 10, &windows_ucs4)])).unwrap();

    assert_eq!(map.get_glyph(0x41), 2);

    let map = load(wrap_subtables(&[(1, 0, &macintosh)])).unwrap();

    assert_eq!(map.get_glyph(0x41), 4);
}

#[test]
fn format12_declared_length_is_clamped_to_the_buffer() {
    // A declared length with the high bit set, and one shorter than the header.
    for length in [0xFFFF_FFFFu32, 0] {
        let mut subtable = build_format12_subtable(12, &[(0x41, 0x42, 7)]);
        subtable[4..8].copy_from_slice(&length.to_be_bytes());

        let map = load(wrap_subtables(&[(3, 10, &subtable)])).unwrap();

        if length == 0 {
            assert_eq!(map.get_glyph(0x41), 0);
        } else {
            assert_eq!(map.get_glyph(0x42), 8);
        }
    }
}

// --- synthetic table construction ------------------------------------------------------

fn build_cmap_with_overflowing_format12(num_groups: u32) -> Vec<u8> {
    // Format-12 subtable: format(2) reserved(2) length(4) language(4) numGroups(4) groups[…].
    // length is honest about the 16-byte buffer, so the length-slice succeeds and the
    // overflow surfaces at the group-array slice (the exact path under test).
    let mut subtable = BigEndianBuffer::new();

    subtable
        .uint16(12) // format
        .uint16(0) // reserved
        .uint32(16) // length (header only)
        .uint32(0) // language
        .uint32(num_groups); // numGroups

    // cmap header: version(2) numTables(2), then one EncodingRecord: platform(2) encoding(2) offset(4).
    let mut cmap = BigEndianBuffer::new();

    cmap.uint16(0); // version
    cmap.uint16(1); // numTables
    cmap.uint16(3); // platformID = Windows
    cmap.uint16(10); // encodingID = UCS-4 (any value works; format 12 is selected regardless)
    let offset_pos = cmap.reserve_offset32();
    let position = cmap.position() as u32;
    cmap.patch_uint32(offset_pos, position);
    cmap.bytes(&subtable.to_array());

    cmap.to_array()
}

/// Builds a format-12 (or 13) subtable carrying the given sequential map groups
/// `(start, end, start glyph)`, which must be ordered by start code for the
/// lookup binary search.
fn build_format12_subtable(format: i32, groups: &[(u32, u32, u32)]) -> Vec<u8> {
    let mut subtable = BigEndianBuffer::new();

    subtable
        .uint16(format) // format
        .uint16(0) // reserved
        .uint32((16 + groups.len() * 12) as u32) // length
        .uint32(0) // language
        .uint32(groups.len() as u32); // numGroups

    for &(start, end, start_glyph) in groups {
        subtable.uint32(start).uint32(end).uint32(start_glyph);
    }

    subtable.to_array()
}

/// Builds a cmap with a single format-12 subtable carrying the given groups.
fn build_format12_cmap(groups: &[(u32, u32, u32)]) -> Vec<u8> {
    let subtable = build_format12_subtable(12, groups);

    let mut cmap = BigEndianBuffer::new();

    cmap.uint16(0); // version
    cmap.uint16(1); // numTables
    cmap.uint16(3); // platformID = Windows
    cmap.uint16(10); // encodingID = UCS-4
    cmap.uint32(12); // offset: header(4) + one EncodingRecord(8)
    cmap.bytes(&subtable);

    cmap.to_array()
}

/// Builds a cmap with two Windows-platform Format-4 subtables — a Symbol
/// (encoding 0) one mapping the PUA codepoint 0xF041 and a Unicode-BMP
/// (encoding 1) one mapping 'A' — ordered per `symbol_first`.
fn build_dual_format4_cmap(symbol_first: bool) -> Vec<u8> {
    let symbol = build_single_char_format4(0xF041, 7);
    let unicode = build_single_char_format4('A' as i32, 5);

    // Symbol = 0, UnicodeBMP = 1
    if symbol_first {
        wrap_subtables(&[(3, 0, &symbol), (3, 1, &unicode)])
    } else {
        wrap_subtables(&[(3, 1, &unicode), (3, 0, &symbol)])
    }
}

/// Builds a minimal Format-4 cmap subtable mapping a single `char_code` to `glyph`.
fn build_single_char_format4(char_code: i32, glyph: i32) -> Vec<u8> {
    // Two segments: [charCode, charCode] and the mandatory terminal [0xFFFF, 0xFFFF].
    // No glyphIdArray — the glyph comes from idDelta (idRangeOffset = 0). Total length 32.
    let mut subtable = BigEndianBuffer::new();

    subtable
        .uint16(4) // format
        .uint16(32) // length
        .uint16(0) // language
        .uint16(4) // segCountX2 (segCount = 2)
        .uint16(4) // searchRange
        .uint16(1) // entrySelector
        .uint16(0) // rangeShift
        .uint16(char_code)
        .uint16(0xFFFF) // endCode[2]
        .uint16(0) // reservedPad
        .uint16(char_code)
        .uint16(0xFFFF) // startCode[2]
        .uint16((glyph - char_code) & 0xFFFF)
        .uint16(1) // idDelta[2]
        .uint16(0)
        .uint16(0); // idRangeOffset[2]

    subtable.to_array()
}

/// A Format-4 subtable with three segments:
/// `'A'..'C'` → glyphs 10..12 through idDelta, `0x50..0x53` through the glyph
/// id array `[100, 0, 102]` (one entry short) with idDelta 1, and the sentinel.
fn build_format4() -> Vec<u8> {
    let mut subtable = BigEndianBuffer::new();

    subtable
        .uint16(4) // format
        .uint16(16 + 3 * 8 + 3 * 2) // length
        .uint16(0) // language
        .uint16(6) // segCountX2 (segCount = 3)
        .uint16(4) // searchRange
        .uint16(1) // entrySelector
        .uint16(2) // rangeShift
        .uint16(0x43)
        .uint16(0x53)
        .uint16(0xFFFF) // endCode[3]
        .uint16(0) // reservedPad
        .uint16(0x41)
        .uint16(0x50)
        .uint16(0xFFFF) // startCode[3]
        .uint16((10 - 0x41) & 0xFFFF)
        .uint16(1)
        .uint16(1) // idDelta[3]
        .uint16(0)
        .uint16(4) // idRangeOffset[1]: 2 words ahead → glyphIdArray[0]
        .uint16(0) // idRangeOffset[3]
        .uint16(100)
        .uint16(0)
        .uint16(102); // glyphIdArray[3]

    subtable.to_array()
}

fn wrap_subtable(platform: i32, encoding: i32, subtable: &[u8]) -> Vec<u8> {
    wrap_subtables(&[(platform, encoding, subtable)])
}

/// Builds a cmap whose directory lists the given `(platform, encoding, subtable)`
/// entries in order, followed by the subtables.
fn wrap_subtables(subtables: &[(i32, i32, &[u8])]) -> Vec<u8> {
    let mut cmap = BigEndianBuffer::new();

    cmap.uint16(0); // version
    cmap.uint16(subtables.len() as i32); // numTables

    // The 8-byte EncodingRecords follow the 4-byte header.
    let mut offset = 4 + subtables.len() * 8;

    for &(platform, encoding, subtable) in subtables {
        cmap.uint16(platform).uint16(encoding).uint32(offset as u32);
        offset += subtable.len();
    }

    for &(_, _, subtable) in subtables {
        cmap.bytes(subtable);
    }

    cmap.to_array()
}
