//! Direct coverage for `Codepoint` — surrogate-pair decoding via
//! `Codepoint::read_at`, the small helper properties / methods, and the
//! bitmask-based `is_white_space` path that depends on every used
//! `GeneralCategory` value fitting in 64 bits.

use super::ucd_test_data::utf16;
use super::*;

#[test]
fn read_at_bmp_scalar_returns_char_and_advances_by_one() {
    for (text, index, expected_value, expected_count) in
        [("a", 0usize, 'a' as u32, 1usize), ("abc", 1, 'b' as u32, 1), ("abc", 2, 'c' as u32, 1)]
    {
        let (cp, count) = Codepoint::read_at(&utf16(text), index);

        assert_eq!(expected_value, cp.value());
        assert_eq!(expected_count, count);
    }
}

#[test]
fn read_at_high_surrogate_at_start_decodes_pair() {
    // U+1F600 GRINNING FACE — high surrogate at index 0.
    let text = utf16("😀");

    let (cp, count) = Codepoint::read_at(&text, 0);

    assert_eq!(0x1F600, cp.value());
    assert_eq!(2, count);
}

#[test]
fn read_at_low_surrogate_scans_back_to_high_surrogate() {
    // Reading at the low surrogate position should still return the full
    // supplementary codepoint by looking one index back.
    let text = utf16("😀");

    let (cp, count) = Codepoint::read_at(&text, 1);

    assert_eq!(0x1F600, cp.value());
    assert_eq!(2, count);
}

#[test]
fn read_at_high_surrogate_without_following_low_returns_replacement() {
    // Lone high surrogate at end of string.
    let text = ['a' as u16, 0xD83D];

    let (cp, count) = Codepoint::read_at(&text, 1);

    assert_eq!(Codepoint::replacement_codepoint().value(), cp.value());
    assert_eq!(1, count);
}

#[test]
fn read_at_high_surrogate_followed_by_non_low_returns_replacement() {
    // High surrogate followed by a regular BMP character (invalid pair).
    let text = [0xD83D, 'a' as u16];

    let (cp, count) = Codepoint::read_at(&text, 0);

    assert_eq!(Codepoint::replacement_codepoint().value(), cp.value());
    assert_eq!(1, count);
}

#[test]
fn read_at_lone_low_surrogate_at_start_returns_replacement() {
    // Lone low surrogate with nothing before it.
    let text = [0xDE00, 'b' as u16];

    let (cp, count) = Codepoint::read_at(&text, 0);

    assert_eq!(Codepoint::replacement_codepoint().value(), cp.value());
    assert_eq!(1, count);
}

#[test]
fn read_at_low_surrogate_not_preceded_by_high_returns_replacement() {
    // Low surrogate at index 1, but index 0 is a regular char (not a high surrogate).
    let text = ['a' as u16, 0xDE00];

    let (cp, count) = Codepoint::read_at(&text, 1);

    assert_eq!(Codepoint::replacement_codepoint().value(), cp.value());
    assert_eq!(1, count);
}

#[test]
fn read_at_index_past_length_returns_replacement() {
    let text = utf16("abc");

    let (cp, count) = Codepoint::read_at(&text, 5);

    assert_eq!(Codepoint::replacement_codepoint().value(), cp.value());
    assert_eq!(1, count);
}

#[test]
fn read_at_index_at_length_returns_replacement() {
    let text = utf16("abc");

    let (cp, count) = Codepoint::read_at(&text, 3);

    assert_eq!(Codepoint::replacement_codepoint().value(), cp.value());
    assert_eq!(1, count);
}

#[test]
fn read_at_empty_span_returns_replacement() {
    let (cp, count) = Codepoint::read_at(&[], 0);

    assert_eq!(Codepoint::replacement_codepoint().value(), cp.value());
    assert_eq!(1, count);
}

#[test]
fn codepoint_enumerator_decodes_mixed_bmp_and_supplementary_text() {
    // 'a' + 😀 (U+1F600) + 'b' + ✓ (U+2713) + 🚀 (U+1F680).
    let text = utf16("a😀b✓🚀");
    let expected = ['a' as u32, 0x1F600, 'b' as u32, 0x2713, 0x1F680];

    let mut enumerator = CodepointEnumerator::new(&text);
    let mut actual = Vec::new();

    while let Some(cp) = enumerator.move_next() {
        actual.push(cp.value());
    }

    assert_eq!(expected.as_slice(), actual.as_slice());

    // The Iterator implementation yields the same sequence.
    let iterated: Vec<u32> = CodepointEnumerator::new(&text).map(Codepoint::value).collect();
    assert_eq!(actual, iterated);
}

/// `Codepoint::is_white_space` uses a bitmask trick that assumes every
/// `GeneralCategory` value used in the mask fits in 64 bits.
/// If `Control`, `Format`, or `SpaceSeparator` ever moves past
/// position 63 in the enum, the mask silently produces wrong results. This
/// guards against that.
#[test]
fn is_white_space_all_masked_general_categories_fit_in_bitmask() {
    assert!((GeneralCategory::Control as i32) < 64);
    assert!((GeneralCategory::Format as i32) < 64);
    assert!((GeneralCategory::SpaceSeparator as i32) < 64);
}

#[test]
fn is_white_space_known_codepoints() {
    let cases = [
        (0x0020u32, true), // SPACE
        (0x0009, true),    // TAB (Control)
        (0x000A, true),    // LF (Control)
        (0x000D, true),    // CR (Control)
        (0x00A0, true),    // NBSP (SpaceSeparator)
        (0x200B, true),    // ZWSP (Format)
        (0x0061, false),   // 'a'
        (0x0030, false),   // '0'
        (0x002E, false),   // '.'
    ];

    for (value, expected) in cases {
        assert_eq!(expected, Codepoint::new(value).is_white_space(), "U+{value:04X}");
    }
}

#[test]
fn is_break_char_known_codepoints() {
    let cases = [
        (0x000Au32, true), // LF
        (0x000B, true),    // VT
        (0x000C, true),    // FF
        (0x000D, true),    // CR
        (0x0085, true),    // NEL
        (0x2028, true),    // LINE SEPARATOR
        (0x2029, true),    // PARAGRAPH SEPARATOR
        (0x0020, false),
        (0x0061, false),
        (0x0009, false), // TAB is not a "break" char in this sense
    ];

    for (value, expected) in cases {
        assert_eq!(expected, Codepoint::new(value).is_break_char(), "U+{value:04X}");
    }
}

#[test]
fn is_east_asian_known_codepoints() {
    let cases = [
        (0x0061u32, false), // 'a'
        (0x4E2D, true),     // '中' Wide
        (0xFF21, true),     // 'Ａ' Fullwidth
        (0xFF71, true),     // 'ｱ' Halfwidth
        (0x03B1, false),    // 'α' Ambiguous (not east asian per is_east_asian)
        (0x0020, false),    // ' ' Narrow
    ];

    for (value, expected) in cases {
        assert_eq!(expected, Codepoint::new(value).is_east_asian(), "U+{value:04X}");
    }
}

#[test]
fn get_canonical_type_maps_known_codepoints() {
    let cases = [
        (0x3008u32, 0x2329u32), // 〈 → ⟨
        (0x3009, 0x232A),       // 〉 → ⟩
        (0x0061, 0x0061),       // 'a' → 'a' (unchanged)
        (0x0028, 0x0028),       // '(' → '(' (unchanged)
    ];

    for (input, expected) in cases {
        let actual = Codepoint::get_canonical_type(Codepoint::new(input));

        assert_eq!(expected, actual.value());
    }
}

#[test]
fn try_get_paired_bracket_known_codepoints() {
    let cases = [
        (0x0028u32, true, 0x0029u32), // '(' → ')'
        (0x0029, true, 0x0028),       // ')' → '('
        (0x005B, true, 0x005D),       // '[' → ']'
        (0x005D, true, 0x005B),       // ']' → '['
        (0x0061, false, 0),           // 'a' has no pair
        (0x0020, false, 0),           // ' ' has no pair
    ];

    for (codepoint, expected_success, expected_pair) in cases {
        let result = Codepoint::new(codepoint).try_get_paired_bracket();

        assert_eq!(expected_success, result.is_some());

        if let Some(pair) = result {
            assert_eq!(expected_pair, pair.value());
        }
    }
}

#[test]
fn is_default_ignorable_known_codepoints() {
    let cases = [
        (0x00ADu32, true), // SOFT HYPHEN
        (0x200D, true),    // ZERO WIDTH JOINER
        (0x034F, true),    // COMBINING GRAPHEME JOINER
        (0xFE0F, true),    // VARIATION SELECTOR-16
        (0xFEFF, true),    // ZERO WIDTH NO-BREAK SPACE
        (0xE0100, true),   // VARIATION SELECTOR-17
        (0x0301, false),   // COMBINING ACUTE ACCENT
        (0x20E3, false),   // COMBINING ENCLOSING KEYCAP
        (0x0041, false),   // 'A'
        (0x0020, false),   // ' '
    ];

    for (value, expected) in cases {
        assert_eq!(expected, Codepoint::new(value).is_default_ignorable(), "U+{value:04X}");
    }
}

#[test]
fn is_emoji_known_codepoints() {
    let cases = [
        (0x1F600u32, true), // 😀 GRINNING FACE
        (0x1F4AF, true),    // 💯 HUNDRED POINTS SYMBOL
        (0x2764, true),     // ❤ HEAVY BLACK HEART (text presentation by default)
        (0x0023, true),     // '#' (an emoji only as part of a keycap sequence)
        (0x0030, true),     // '0'
        (0xFE0F, false),    // VARIATION SELECTOR-16 is Emoji_Component, not Emoji
        (0x0041, false),    // 'A'
    ];

    for (value, expected) in cases {
        assert_eq!(expected, Codepoint::new(value).is_emoji(), "U+{value:04X}");
    }
}

#[test]
fn has_emoji_presentation_known_codepoints() {
    let cases = [
        (0x1F600u32, true), // 😀 defaults to emoji presentation
        (0x1F4AF, true),    // 💯 defaults to emoji presentation
        (0x231A, true),     // ⌚ WATCH defaults to emoji presentation
        (0x2764, false),    // ❤ needs U+FE0F to be presented as emoji
        (0x0023, false),    // '#'
        (0x0030, false),    // '0'
        (0x0041, false),    // 'A'
    ];

    for (value, expected) in cases {
        assert_eq!(expected, Codepoint::new(value).has_emoji_presentation(), "U+{value:04X}");
    }
}

#[test]
fn implicit_conversions_round_trip_value() {
    let cp = Codepoint::new(0x1F600);

    let as_int: i32 = cp.into();
    let as_uint: u32 = cp.into();

    assert_eq!(0x1F600, as_int);
    assert_eq!(0x1F600, as_uint);
}

#[test]
fn is_in_range_inclusive_bounds_are_inclusive() {
    assert!(Codepoint::is_in_range_inclusive(Codepoint::new(0x10), 0x10, 0x20));
    assert!(Codepoint::is_in_range_inclusive(Codepoint::new(0x20), 0x10, 0x20));
    assert!(Codepoint::is_in_range_inclusive(Codepoint::new(0x15), 0x10, 0x20));
    assert!(!Codepoint::is_in_range_inclusive(Codepoint::new(0x0F), 0x10, 0x20));
    assert!(!Codepoint::is_in_range_inclusive(Codepoint::new(0x21), 0x10, 0x20));
}
