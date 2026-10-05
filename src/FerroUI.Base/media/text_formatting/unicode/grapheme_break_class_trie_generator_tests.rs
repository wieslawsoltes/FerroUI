//! Conformance tests for `GraphemeEnumerator` against the Unicode
//! GraphemeBreakTest data.

use super::ucd_test_data::{read_ucd_file, report, utf16, utf16_from_utf32};
use super::*;

/// The data generator of the conformance test (reads `auxiliary/GraphemeBreakTest.txt`):
/// `(line, line number, first grapheme, first two graphemes)`.
fn read_data(content: &str) -> Vec<(String, usize, Vec<u16>, Vec<u16>)> {
    let mut test_data = Vec::new();

    for (index, line) in content.lines().enumerate() {
        let line_number = index + 1;

        if line.starts_with('#') || line.is_empty() {
            continue;
        }

        let data = line.split('#').next().unwrap_or("").replace("÷\t", "÷");
        let elements: Vec<&str> = data.trim_matches('÷').split('÷').collect();

        let parse = |element: &str| -> Vec<u32> {
            element
                .replace(" × ", " ")
                .split(' ')
                .filter(|x| !x.is_empty() && *x != "×")
                .map(|x| u32::from_str_radix(x, 16).expect("a code point"))
                .collect()
        };

        let grapheme = parse(elements[0]);
        let mut codepoints = grapheme.clone();

        if elements.len() > 1 {
            codepoints.extend(parse(elements[1]));
        }

        test_data.push((line.to_string(), line_number, utf16_from_utf32(&grapheme), utf16_from_utf32(&codepoints)));
    }

    test_data
}

/// Conformance test against `auxiliary/GraphemeBreakTest.txt`; skipped unless
/// `FERROUI_UCD_DIR` is set (see `ucd_test_data.rs`).
#[test]
fn should_enumerate() {
    let Some(content) = read_ucd_file("auxiliary/GraphemeBreakTest.txt") else {
        return;
    };

    let mut passed = 0;
    let mut failures = Vec::new();

    for (line, line_number, grapheme, text) in read_data(&content) {
        let mut enumerator = GraphemeEnumerator::new(&text);

        let g = enumerator.move_next().unwrap_or_default();

        let actual = &text[g.offset()..g.offset() + g.length()];

        if actual == grapheme.as_slice() {
            passed += 1;
        } else {
            failures.push(format!(
                "Failed line {line_number}\n       Text: {text:X?}\n   Grapheme: {grapheme:X?}\n     Actual: {actual:X?}\n       Line: {line}\n"
            ));
        }
    }

    report("GraphemeBreakTest.txt", passed, &failures);
}

#[test]
fn should_enumerate_other() {
    let text = utf16("ABCDEFGHIJ");

    let mut enumerator = GraphemeEnumerator::new(&text);

    let mut count = 0;

    while let Some(grapheme) = enumerator.move_next() {
        assert_eq!(1, grapheme.length());

        count += 1;
    }

    assert_eq!(10, count);
}

/// Not upstream (its data driven test needs the UCD files): the main cluster
/// rules on a few well known sequences.
#[test]
fn enumerates_well_known_clusters() {
    fn clusters(text: &str) -> Vec<String> {
        let text = utf16(text);
        GraphemeEnumerator::new(&text)
            .map(|g| String::from_utf16(&text[g.offset()..g.offset() + g.length()]).unwrap())
            .collect()
    }

    // GB3: CR LF stay together; GB4/GB5: breaks around controls.
    assert_eq!(clusters("a\r\nb"), ["a", "\r\n", "b"]);
    // GB9: combining marks attach to their base.
    assert_eq!(clusters("e\u{0301}x"), ["e\u{0301}", "x"]);
    // GB6-GB8: Hangul syllable sequences.
    assert_eq!(clusters("\u{1100}\u{1161}\u{11A8}\u{AC00}"), ["\u{1100}\u{1161}\u{11A8}", "\u{AC00}"]);
    // GB11: emoji ZWJ sequences.
    assert_eq!(clusters("\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}!"), ["\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}", "!"]);
    // GB12/GB13: regional indicators pair up.
    assert_eq!(clusters("\u{1F1E9}\u{1F1EA}\u{1F1EB}\u{1F1F7}\u{1F1E9}"), ["\u{1F1E9}\u{1F1EA}", "\u{1F1EB}\u{1F1F7}", "\u{1F1E9}"]);
    // GB9c: Indic conjuncts (consonant, virama, consonant).
    assert_eq!(clusters("\u{0915}\u{094D}\u{0937}\u{0915}"), ["\u{0915}\u{094D}\u{0937}", "\u{0915}"]);

    // The first codepoint and offsets are reported.
    let text = utf16("a\u{1F600}");
    let mut enumerator = GraphemeEnumerator::new(&text);
    let first = enumerator.move_next().unwrap();
    assert_eq!((first.first_codepoint().value(), first.offset(), first.length()), ('a' as u32, 0, 1));
    let second = enumerator.move_next().unwrap();
    assert_eq!((second.first_codepoint().value(), second.offset(), second.length()), (0x1F600, 1, 2));
    assert!(enumerator.move_next().is_none());
}
