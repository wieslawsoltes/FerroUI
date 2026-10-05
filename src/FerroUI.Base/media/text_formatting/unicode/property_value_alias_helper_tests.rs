//! Lightweight round-trip checks for the generated `PropertyValueAliasHelper`.
//! Catches regressions in the alias-helper writer (e.g. wrong type name, missing
//! entries, casing mismatches) without re-asserting the UCD aliases themselves.

use super::*;

#[test]
fn get_bidi_class_known_tags() {
    for (tag, expected) in [
        ("L", BidiClass::LeftToRight),
        ("R", BidiClass::RightToLeft),
        ("AL", BidiClass::ArabicLetter),
        ("EN", BidiClass::EuropeanNumber),
    ] {
        assert_eq!(expected, PropertyValueAliasHelper::get_bidi_class(tag));
    }
}

#[test]
fn get_bidi_class_unknown_tag_falls_back_to_left_to_right() {
    // The generator emits LeftToRight as the fallback for unknown tags.
    assert_eq!(BidiClass::LeftToRight, PropertyValueAliasHelper::get_bidi_class("not-a-real-tag"));
}

#[test]
fn get_script_known_tags() {
    for (tag, expected) in [
        ("Latn", Script::Latin),
        ("Cyrl", Script::Cyrillic),
        ("Hani", Script::Han),
        ("Hebr", Script::Hebrew),
        ("Arab", Script::Arabic),
        ("Zyyy", Script::Common),
    ] {
        assert_eq!(expected, PropertyValueAliasHelper::get_script(tag));
    }
}

#[test]
fn get_tag_round_trips_script_through_get_script() {
    // Tag -> enum -> tag should round-trip for every script the helper knows.
    // Pick a handful so we catch obvious typoes in the writer without needing
    // a full mirror of UCD here.
    for script in [Script::Latin, Script::Cyrillic, Script::Han, Script::Hebrew, Script::Arabic] {
        let tag = PropertyValueAliasHelper::get_tag(script);
        assert_eq!(script, PropertyValueAliasHelper::get_script(tag));
    }

    // Not upstream: the round trip holds for every script.
    for value in 0..Script::COUNT as u32 {
        let script = Script::from_u32(value);
        assert_eq!(value as i32, script as i32);
        assert_eq!(script, PropertyValueAliasHelper::get_script(PropertyValueAliasHelper::get_tag(script)));
    }
}

#[test]
fn get_general_category_known_tags() {
    for (tag, expected) in [
        ("Lu", GeneralCategory::UppercaseLetter),
        ("Ll", GeneralCategory::LowercaseLetter),
        ("Nd", GeneralCategory::DecimalNumber),
        ("Zs", GeneralCategory::SpaceSeparator),
        ("Cc", GeneralCategory::Control),
    ] {
        assert_eq!(expected, PropertyValueAliasHelper::get_general_category(tag));
    }
}

#[test]
fn get_line_break_class_known_tags() {
    for (tag, expected) in [
        ("AL", LineBreakClass::Alphabetic),
        ("LF", LineBreakClass::LineFeed),
        ("CR", LineBreakClass::CarriageReturn),
        ("XX", LineBreakClass::Unknown),
    ] {
        assert_eq!(expected, PropertyValueAliasHelper::get_line_break_class(tag));
    }
}

#[test]
fn get_word_break_class_known_tags() {
    for (tag, expected) in [
        ("LE", WordBreakClass::ALetter),
        ("CR", WordBreakClass::CarriageReturn),
        ("LF", WordBreakClass::LineFeed),
        ("XX", WordBreakClass::Other),
    ] {
        assert_eq!(expected, PropertyValueAliasHelper::get_word_break_class(tag));
    }
}

#[test]
fn get_bidi_paired_bracket_type_known_tags() {
    for (tag, expected) in [
        ("o", BidiPairedBracketType::Open),
        ("c", BidiPairedBracketType::Close),
        ("n", BidiPairedBracketType::None),
    ] {
        assert_eq!(expected, PropertyValueAliasHelper::get_bidi_paired_bracket_type(tag));
    }
}
