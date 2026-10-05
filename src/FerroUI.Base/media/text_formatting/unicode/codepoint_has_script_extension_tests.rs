use super::*;

#[test]
fn returns_true_when_script_matches_primary_script() {
    let latin_a = Codepoint::new('A' as u32);

    assert!(latin_a.has_script_extension(Script::Latin));
}

#[test]
fn returns_false_when_script_does_not_match() {
    let latin_a = Codepoint::new('A' as u32);

    assert!(!latin_a.has_script_extension(Script::Hiragana));
}

#[test]
fn returns_false_for_unknown_script() {
    let latin_a = Codepoint::new('A' as u32);

    assert!(!latin_a.has_script_extension(Script::Unknown));
}

#[test]
fn hrkt_shared_codepoints_match_both_hiragana_and_katakana() {
    // U+30FC Katakana-Hiragana Prolonged Sound Mark: scx={Hira, Kana}.
    // U+3031 Vertical Kana Repeat Mark: scx={Hira, Kana}.
    // U+30A0 Katakana-Hiragana Double Hyphen: scx={Hira, Kana}.
    for value in [0x30FCu32, 0x3031, 0x30A0] {
        let cp = Codepoint::new(value);

        assert!(cp.has_script_extension(Script::Hiragana));
        assert!(cp.has_script_extension(Script::Katakana));
    }
}

#[test]
fn hrkt_codepoint_does_not_claim_latin_extension() {
    let cp = Codepoint::new(0x30FC);

    assert!(!cp.has_script_extension(Script::Latin));
}

#[test]
fn arabic_tatweel_reports_all_listed_scripts() {
    // U+0640 ARABIC TATWEEL has primary Script=Common with scx covering several
    // Arabic-derived scripts (incl. Arabic, Syriac, Mandaic, ...). Primary Common is
    // *not* part of the extensions set.
    let cp = Codepoint::new(0x0640);

    assert!(cp.has_script_extension(Script::Arabic));
    assert!(cp.has_script_extension(Script::Syriac));
    assert!(!cp.has_script_extension(Script::Common));
    assert!(!cp.has_script_extension(Script::Latin));
}

#[test]
fn codepoint_without_extensions_falls_back_to_primary_script() {
    // U+05D0 HEBREW LETTER ALEF has primary Script=Hebrew and no Script_Extensions entry.
    let cp = Codepoint::new(0x05D0);

    assert_eq!(Script::Hebrew, cp.script());
    assert!(cp.has_script_extension(Script::Hebrew));
    assert!(!cp.has_script_extension(Script::Arabic));
}
