//! Spot-checks for `UnicodeData`. The trie generator already round-trips
//! every assigned codepoint against its source dictionary, so these tests focus on
//! pinning the public surface against drift: shift/mask layout, default values for
//! unassigned codepoints, and a few well-known codepoints across the four tries.

use super::*;

#[test]
fn get_general_category_known_codepoints() {
    // Note: codepoints >= HighStart (currently 0x100000) all collapse to a single
    // fallback value because the trie compresses Plane 16 to save space. The
    // resulting GeneralCategory is not the per-codepoint UCD value. See
    // unicode_trie_tests::get_at_and_above_high_start_all_codepoints_share_fallback.
    let cases = [
        (0x0061u32, GeneralCategory::LowercaseLetter), // 'a'
        (0x0041, GeneralCategory::UppercaseLetter),    // 'A'
        (0x0030, GeneralCategory::DecimalNumber),      // '0'
        (0x0020, GeneralCategory::SpaceSeparator),     // ' '
        (0x000A, GeneralCategory::Control),            // '\n'
        (0x0009, GeneralCategory::Control),            // '\t'
        (0x002E, GeneralCategory::OtherPunctuation),   // '.'
        (0x0028, GeneralCategory::OpenPunctuation),    // '('
        (0x0029, GeneralCategory::ClosePunctuation),   // ')'
        (0x0024, GeneralCategory::CurrencySymbol),     // '$'
        (0x002B, GeneralCategory::MathSymbol),         // '+'
        (0x200B, GeneralCategory::Format),             // ZWSP
        (0xE000, GeneralCategory::PrivateUse),         // BMP PUA start
        (0xD800, GeneralCategory::Surrogate),          // high surrogate start (LSCP path)
        (0xDFFF, GeneralCategory::Surrogate),          // low surrogate end (LSCP path)
    ];

    for (codepoint, expected) in cases {
        assert_eq!(expected, UnicodeData::get_general_category(codepoint), "U+{codepoint:04X}");
    }
}

#[test]
fn get_script_known_codepoints() {
    let cases = [
        (0x0061u32, Script::Latin), // 'a'
        (0x0041, Script::Latin),    // 'A'
        (0x044F, Script::Cyrillic), // 'я'
        (0x4E2D, Script::Han),      // '中'
        (0x05D0, Script::Hebrew),   // 'א'
        (0x0627, Script::Arabic),   // 'ا'
        (0x0020, Script::Common),   // ' '
        (0x0030, Script::Common),   // '0'
        (0x1F600, Script::Common),  // 😀 (supplementary, common)
        (0x0300, Script::Inherited), // combining grave (inherited)
    ];

    for (codepoint, expected) in cases {
        assert_eq!(expected, UnicodeData::get_script(codepoint), "U+{codepoint:04X}");
    }
}

#[test]
fn get_bi_di_class_known_codepoints() {
    let cases = [
        (0x0061u32, BidiClass::LeftToRight),    // 'a'
        (0x0041, BidiClass::LeftToRight),       // 'A'
        (0x0039, BidiClass::EuropeanNumber),    // '9'
        (0x0024, BidiClass::EuropeanTerminator), // '$'
        (0x002C, BidiClass::CommonSeparator),   // ','
        (0x0020, BidiClass::WhiteSpace),        // ' '
        (0x0009, BidiClass::SegmentSeparator),  // '\t'
        (0x000A, BidiClass::ParagraphSeparator), // '\n'
        (0x05D0, BidiClass::RightToLeft),       // 'א'
        (0x0627, BidiClass::ArabicLetter),      // 'ا'
    ];

    for (codepoint, expected) in cases {
        assert_eq!(expected, UnicodeData::get_bi_di_class(codepoint), "U+{codepoint:04X}");
    }
}

#[test]
fn get_bi_di_paired_bracket_round_trips_known_pairs() {
    let cases = [
        (0x0028u32, BidiPairedBracketType::Open, 0x0029u32), // '(' → ')'
        (0x0029, BidiPairedBracketType::Close, 0x0028),      // ')' → '('
        (0x005B, BidiPairedBracketType::Open, 0x005D),       // '[' → ']'
        (0x005D, BidiPairedBracketType::Close, 0x005B),      // ']' → '['
        (0x007B, BidiPairedBracketType::Open, 0x007D),       // '{' → '}'
    ];

    for (codepoint, expected_type, expected_pair) in cases {
        assert_eq!(expected_type, UnicodeData::get_bi_di_paired_bracket_type(codepoint));
        assert_eq!(expected_pair, UnicodeData::get_bi_di_paired_bracket(codepoint).value());
    }
}

#[test]
fn get_bi_di_paired_bracket_type_non_bracket_is_none() {
    assert_eq!(BidiPairedBracketType::None, UnicodeData::get_bi_di_paired_bracket_type(0x0061));
    assert_eq!(BidiPairedBracketType::None, UnicodeData::get_bi_di_paired_bracket_type(0x0020));
}

#[test]
fn get_line_break_class_known_codepoints() {
    let cases = [
        (0x0061u32, LineBreakClass::Alphabetic),  // 'a'
        (0x000A, LineBreakClass::LineFeed),       // '\n'
        (0x000D, LineBreakClass::CarriageReturn), // '\r'
        (0x0020, LineBreakClass::Space),          // ' '
        (0x0009, LineBreakClass::BreakAfter),     // '\t'
        (0x002D, LineBreakClass::Hyphen),         // '-'
        (0x0028, LineBreakClass::OpenPunctuation), // '('
        (0x0029, LineBreakClass::CloseParenthesis), // ')'
        (0x0030, LineBreakClass::Numeric),        // '0'
        (0x4E2D, LineBreakClass::Ideographic),    // '中'
        (0x2028, LineBreakClass::MandatoryBreak), // LINE SEPARATOR
        (0x2029, LineBreakClass::MandatoryBreak), // PARAGRAPH SEPARATOR
    ];

    for (codepoint, expected) in cases {
        assert_eq!(expected, UnicodeData::get_line_break_class(codepoint), "U+{codepoint:04X}");
    }
}

#[test]
fn get_word_break_class_known_codepoints() {
    let cases = [
        (0x0061u32, WordBreakClass::ALetter),     // 'a'
        (0x000D, WordBreakClass::CarriageReturn), // '\r'
        (0x000A, WordBreakClass::LineFeed),       // '\n'
        (0x0020, WordBreakClass::WSegSpace),      // ' '
        (0x0030, WordBreakClass::Numeric),        // '0'
        (0x200D, WordBreakClass::ZWJ),            // ZWJ
        (0x05D0, WordBreakClass::HebrewLetter),   // 'א'
        (0x4E2D, WordBreakClass::Other),          // '中' (CJK is WB=Other)
    ];

    for (codepoint, expected) in cases {
        assert_eq!(expected, UnicodeData::get_word_break_class(codepoint), "U+{codepoint:04X}");
    }
}

#[test]
fn get_grapheme_cluster_break_known_codepoints() {
    let cases = [
        (0x000Du32, GraphemeBreakClass::CR),                // '\r'
        (0x000A, GraphemeBreakClass::LF),                   // '\n'
        (0x200D, GraphemeBreakClass::ZWJ),                  // ZWJ
        (0x1F600, GraphemeBreakClass::ExtendedPictographic), // 😀 (overridden by emoji-data.txt)
        (0x1100, GraphemeBreakClass::L),                    // HANGUL CHOSEONG KIYEOK
        (0x1161, GraphemeBreakClass::V),                    // HANGUL JUNGSEONG A
        (0x11A8, GraphemeBreakClass::T),                    // HANGUL JONGSEONG KIYEOK
        (0x0061, GraphemeBreakClass::Other),                // 'a'
        (0x0030, GraphemeBreakClass::Other),                // '0'
    ];

    for (codepoint, expected) in cases {
        assert_eq!(expected, UnicodeData::get_grapheme_cluster_break(codepoint), "U+{codepoint:04X}");
    }
}

#[test]
fn get_east_asian_width_class_known_codepoints() {
    let cases = [
        (0x0061u32, EastAsianWidthClass::Narrow), // 'a'
        (0x0020, EastAsianWidthClass::Narrow),    // ' '
        (0x4E2D, EastAsianWidthClass::Wide),      // '中'
        (0xFF21, EastAsianWidthClass::Fullwidth), // 'Ａ' FULLWIDTH LATIN CAPITAL A
        (0xFF71, EastAsianWidthClass::Halfwidth), // 'ｱ' HALFWIDTH KATAKANA A
        (0x03B1, EastAsianWidthClass::Ambiguous), // 'α'
        (0x200B, EastAsianWidthClass::Neutral),   // ZWSP
    ];

    for (codepoint, expected) in cases {
        assert_eq!(expected, UnicodeData::get_east_asian_width_class(codepoint), "U+{codepoint:04X}");
    }
}

/// Regression test for the BiDi / GraphemeBreak / UnicodeData trie builders'
/// reliance on the seeded default class sitting at int position 0 (caught at
/// generation time by the ABI validator, but only this asserts the runtime
/// behavior of an unassigned codepoint).
#[test]
fn unassigned_codepoint_falls_back_to_seeded_defaults() {
    // U+0378 is an unassigned BMP code point (and has been for decades — stable choice).
    const UNASSIGNED: u32 = 0x0378;

    // Default Bidi class for unassigned codepoints is LeftToRight (seeded at position 0).
    assert_eq!(BidiClass::LeftToRight, UnicodeData::get_bi_di_class(UNASSIGNED));

    // Default grapheme break class is Other (seeded at position 0).
    assert_eq!(GraphemeBreakClass::Other, UnicodeData::get_grapheme_cluster_break(UNASSIGNED));

    // Default line break class is Unknown — set explicitly via initialValue in the
    // UnicodeData trie builder, not via seed position 0.
    assert_eq!(LineBreakClass::Unknown, UnicodeData::get_line_break_class(UNASSIGNED));

    // Default word break class is Other — also set explicitly in the generator's
    // post-pass that maps unset WordBreakClass to Other.
    assert_eq!(WordBreakClass::Other, UnicodeData::get_word_break_class(UNASSIGNED));
}

/// Not upstream: every value stored in the generated tries must be a member of
/// the enum it is decoded into (`from_u32` maps anything else to value 0), and
/// the enum values must match the numbering the tries were generated with.
#[test]
fn every_trie_value_is_a_valid_enum_member() {
    for codepoint in 0..=0x10FFFFu32 {
        let unicode_data = UnicodeDataTrie::trie().get(codepoint);
        assert!(((unicode_data & UnicodeData::CATEGORY_MASK) as usize) < GeneralCategory::COUNT);
        assert!((((unicode_data >> UnicodeData::SCRIPT_SHIFT) & UnicodeData::SCRIPT_MASK) as usize) < Script::COUNT);

        let bidi = BiDiTrie::trie().get(codepoint);
        assert!((((bidi >> UnicodeData::BIDICLASS_SHIFT) & UnicodeData::BIDICLASS_MASK) as usize) < BidiClass::COUNT);
        assert!(
            (((bidi >> UnicodeData::BIDIPAIREDBRACKEDTYPE_SHIFT) & UnicodeData::BIDIPAIREDBRACKEDTYPE_MASK) as usize)
                < BidiPairedBracketType::COUNT
        );

        let segmentation = SegmentationTrie::trie().get(codepoint);
        assert!(((segmentation & UnicodeData::GRAPHEMEBREAK_MASK) as usize) < GraphemeBreakClass::COUNT);
        assert!(
            (((segmentation >> UnicodeData::WORDBREAK_SHIFT) & UnicodeData::WORDBREAK_MASK) as usize) < WordBreakClass::COUNT
        );
        assert!(
            (((segmentation >> UnicodeData::LINEBREAK_SHIFT) & UnicodeData::LINEBREAK_MASK) as usize) < LineBreakClass::COUNT
        );
        assert!(
            (((segmentation >> UnicodeData::SENTENCEBREAK_SHIFT) & UnicodeData::SENTENCEBREAK_MASK) as usize)
                < SentenceBreakClass::COUNT
        );

        assert!((EastAsianWidthTrie::trie().get(codepoint) as usize) < EastAsianWidthClass::COUNT);
    }

    assert_eq!(GeneralCategory::SpaceSeparator as i32, 37);
    assert_eq!(BidiClass::WhiteSpace as i32, 22);
    assert_eq!(LineBreakClass::Virama as i32, 48);
    assert_eq!(Script::ZanabazarSquare as i32, 175);
    assert_eq!(GraphemeBreakClass::from_u32(18), GraphemeBreakClass::ExtendedPictographic);
    assert_eq!(GraphemeBreakClass::from_u32(19), GraphemeBreakClass::Other);
}
