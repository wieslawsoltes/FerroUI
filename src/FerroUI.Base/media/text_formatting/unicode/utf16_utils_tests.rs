use super::ucd_test_data::utf16;
use super::*;

#[test]
fn character_offset_to_string_offset() {
    let cases = [
        ("\u{2F832}123", 1usize, 2usize),
        ("\u{2F832}123", 2, 3),
        ("test", 3, 3),
        ("\u{2F832}", 0, 0),
        ("12\u{2F832}12", 2, 2),
        ("12\u{2F832}12", 3, 4),
    ];

    for (s, char_offset, string_offset) in cases {
        assert_eq!(string_offset, Utf16Utils::character_offset_to_string_offset(&utf16(s), char_offset, false), "{s}");
    }
}

#[test]
fn character_offset_to_string_offset_throws_on_out_of_range() {
    for (s, char_offset) in [("\u{2F832}", 2usize), ("12", 2)] {
        let text = utf16(s);
        let result = std::panic::catch_unwind(|| Utf16Utils::character_offset_to_string_offset(&text, char_offset, true));

        assert!(result.is_err(), "{s}");

        // Without the flag the length is returned instead.
        assert_eq!(text.len(), Utf16Utils::character_offset_to_string_offset(&text, char_offset, false));
    }
}
