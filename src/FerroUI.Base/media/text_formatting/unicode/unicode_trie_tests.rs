//! Direct coverage for `UnicodeTrie` and `UnicodeTrieBuilder`.
//! The production tries (UnicodeData, BiDi, Segmentation, EastAsianWidth) are
//! used to exercise the four branches of `UnicodeTrie::get`; a small
//! synthetic trie covers the error-value path (which is otherwise unreachable
//! because the committed tries are generated with `error_value == 0`) and the
//! builder round-trip.

use super::*;

#[test]
fn get_bmp_non_surrogate_returns_value_matching_unicode_data_wrapper() {
    // 0x0020 ASCII space, 0x0061 'a', 0x4E2D '中' (BMP CJK), 0xFFFF last BMP non-surrogate
    for codepoint in [0x0020u32, 0x0061, 0x4E2D, 0xFFFF] {
        // Walking the trie directly and re-applying the published shift/mask must
        // produce the same answer as the public UnicodeData wrapper. This catches
        // packing-layout drift between generator (writes packed bits) and
        // UnicodeData::get_* (reads packed bits).
        let packed = UnicodeDataTrie::trie().get(codepoint);

        let category_from_trie = GeneralCategory::from_u32(packed & UnicodeData::CATEGORY_MASK);
        let script_from_trie = Script::from_u32((packed >> UnicodeData::SCRIPT_SHIFT) & UnicodeData::SCRIPT_MASK);

        assert_eq!(UnicodeData::get_general_category(codepoint), category_from_trie);
        assert_eq!(UnicodeData::get_script(codepoint), script_from_trie);
    }
}

#[test]
fn get_surrogate_range_resolves_via_lscp_index() {
    // first/mid/last high surrogate, first/last low surrogate
    for codepoint in [0xD800u32, 0xDB00, 0xDBFF, 0xDC00, 0xDFFF] {
        // Surrogates have a dedicated index region (LSCP_INDEX_2_OFFSET) in the
        // trie. The general category for every codepoint in this range is
        // Surrogate; this asserts the LSCP branch returns the correct row.
        assert_eq!(GeneralCategory::Surrogate, UnicodeData::get_general_category(codepoint));
    }
}

#[test]
fn get_supplementary_below_high_start_resolves_via_two_level_lookup() {
    // first supplementary, 😀, CJK compatibility supplement
    for codepoint in [0x10000u32, 0x1F600, 0x2F800] {
        // Just walking the trie and reapplying the published mask must match the
        // wrapper — same guarantee as the BMP test, but exercises the two-level
        // supplementary lookup branch.
        let packed = BiDiTrie::trie().get(codepoint);
        let bidi_from_trie = BidiClass::from_u32((packed >> UnicodeData::BIDICLASS_SHIFT) & UnicodeData::BIDICLASS_MASK);

        assert_eq!(UnicodeData::get_bi_di_class(codepoint), bidi_from_trie);
    }
}

#[test]
fn get_at_and_above_high_start_all_codepoints_share_fallback() {
    // Every codepoint >= HighStart short-circuits to the trie's last data
    // block. The committed tries set HighStart at 0x100000, so all of Plane
    // 16 collapses to one fallback value — this is a compression artifact
    // of the trie format. The test verifies the SHAPE of that contract (one
    // value for the whole high range) rather than asserting any specific
    // per-codepoint property: callers querying Plane 16 should not rely on
    // PUA / Unassigned distinctions surviving the trie.
    let v100000 = UnicodeDataTrie::trie().get(0x100000);
    assert_eq!(v100000, UnicodeDataTrie::trie().get(0x100001));
    assert_eq!(v100000, UnicodeDataTrie::trie().get(0x10FFFD));
    assert_eq!(v100000, UnicodeDataTrie::trie().get(0x10FFFF));

    let bidi100000 = BiDiTrie::trie().get(0x100000);
    assert_eq!(bidi100000, BiDiTrie::trie().get(0x10FFFF));
}

#[test]
fn get_beyond_max_codepoint_is_handled_gracefully_by_production_tries() {
    // The committed tries are generated with error_value == 0, so codepoints
    // past 0x10FFFF return 0 (Other category, LeftToRight bidi, etc.). This
    // documents that contract; the synthetic-trie test below covers the
    // case where error_value is non-zero.
    const BEYOND_RANGE: u32 = 0x110000;

    assert_eq!(0, UnicodeDataTrie::trie().get(BEYOND_RANGE));
    assert_eq!(0, BiDiTrie::trie().get(BEYOND_RANGE));
    assert_eq!(0, SegmentationTrie::trie().get(BEYOND_RANGE));
    assert_eq!(0, EastAsianWidthTrie::trie().get(BEYOND_RANGE));
}

#[test]
fn builder_round_trips_set_values() {
    let mut builder = UnicodeTrieBuilder::new(7, 0);
    builder.set(0x0061, 0xAA);
    builder.set(0x4E2D, 0xBB);
    builder.set(0x1F600, 0xCC);

    let trie = builder.freeze();

    assert_eq!(0xAA, trie.get(0x0061));
    assert_eq!(0xBB, trie.get(0x4E2D));
    assert_eq!(0xCC, trie.get(0x1F600));
}

#[test]
fn builder_set_range_applies_value_to_every_codepoint_in_range() {
    let mut builder = UnicodeTrieBuilder::new(0, 0);
    builder.set_range(0x2000, 0x2010, 0x42);

    let trie = builder.freeze();

    for cp in 0x2000u32..=0x2010 {
        assert_eq!(0x42, trie.get(cp));
    }

    // Just outside the range stays at the initial value (0 by default).
    assert_eq!(0, trie.get(0x1FFF));
    assert_eq!(0, trie.get(0x2011));
}

#[test]
fn builder_unassigned_codepoints_get_initial_value() {
    let mut builder = UnicodeTrieBuilder::new(0xDEAD, 0);
    builder.set(0x0061, 0xBEEF);

    let trie = builder.freeze();

    assert_eq!(0xBEEF, trie.get(0x0061));
    assert_eq!(0xDEAD, trie.get(0x0062));
    assert_eq!(0xDEAD, trie.get(0x4E2D));
    assert_eq!(0xDEAD, trie.get(0x1F600));
}

#[test]
fn get_out_of_range_returns_configured_error_value() {
    // Build with a non-zero error_value so the "> 0x10FFFF" branch produces a
    // distinguishable result. This is the only feasible test of that branch —
    // the committed tries all use error_value == 0 which collides with the
    // happy-path zero value.
    let mut builder = UnicodeTrieBuilder::new(0, 0xFFFF);
    builder.set(0x0061, 0x11);

    let trie = builder.freeze();

    assert_eq!(0xFFFF, trie.get(0x110000));
    assert_eq!(0xFFFF, trie.get(0xFFFFFFFF));
}

#[test]
fn get_at_and_above_high_start_on_synthetic_trie_uses_high_fallback() {
    // set_range across a huge supplementary span forces the builder to allocate
    // a high block. Codepoints at and above the resulting HighStart should
    // return the value that covers the high range.
    let mut builder = UnicodeTrieBuilder::new(0, 0);
    builder.set_range(0x80000, 0x10FFFF, 0x55);

    let trie = builder.freeze();

    assert_eq!(0x55, trie.get(0x100000));
    assert_eq!(0x55, trie.get(0x10FFFF));

    // Below the high range — still the initial value.
    assert_eq!(0, trie.get(0x1000));
}

/// Not upstream: the serialized form of a rebuilt trie must be readable for
/// every code point, including the builder's own lookups before freezing.
#[test]
fn builder_get_matches_frozen_trie() {
    let mut builder = UnicodeTrieBuilder::new(3, 9);
    builder.set_range(0x0041, 0x005A, 10);
    builder.set_range_with_overwrite(0x0050, 0x0060, 11, false);
    builder.set(0xD801, 12);
    builder.set_range(0x20000, 0x2A6DF, 13);

    let expected: Vec<u32> =
        [0x41, 0x50, 0x5B, 0x60, 0x61, 0xD801, 0x1FFFF, 0x20000, 0x2A6DF, 0x2A6E0, 0x10FFFF].iter().map(|&c| builder.get(c)).collect();
    assert_eq!(expected, [10, 10, 11, 11, 3, 12, 3, 13, 13, 3, 3]);

    let trie = builder.freeze();
    let actual: Vec<u32> = [0x41u32, 0x50, 0x5B, 0x60, 0x61, 0xD801, 0x1FFFF, 0x20000, 0x2A6DF, 0x2A6E0, 0x10FFFF]
        .iter()
        .map(|&c| trie.get(c))
        .collect();
    assert_eq!(expected, actual);
    assert_eq!(9, trie.get(0x110000));
}
