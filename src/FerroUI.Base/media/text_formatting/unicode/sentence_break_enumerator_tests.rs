use super::ucd_test_data::{read_break_test_file, read_ucd_file, report, utf16, utf16_from_code_points};
use super::*;

/// Conformance test against `auxiliary/SentenceBreakTest.txt`; skipped unless
/// `FERROUI_UCD_DIR` is set (see `ucd_test_data.rs`).
#[test]
fn should_find_breaks() {
    let Some(content) = read_ucd_file("auxiliary/SentenceBreakTest.txt") else {
        return;
    };

    let mut passed = 0;
    let mut failures = Vec::new();

    for (line_number, code_points, break_points, rules) in read_break_test_file(&content) {
        let text = utf16_from_code_points(&code_points);
        let mut enumerator = SentenceBreakEnumerator::new(&text);
        let mut found_breaks = vec![0];
        let mut current_position = 0;

        while let Some(segment) = enumerator.move_next() {
            let mut text_span = segment.text();
            while !text_span.is_empty() {
                let (_, consumed) = Codepoint::read_at(text_span, 0);
                text_span = &text_span[consumed..];
                current_position += 1;
            }

            found_breaks.push(current_position);
        }

        if found_breaks == break_points {
            passed += 1;
        } else {
            failures.push(format!(
                "Failed test on line {line_number}\n    Code Points: {code_points:X?}\nExpected Breaks: {break_points:?}\n  Actual Breaks: {found_breaks:?}\n     Char Props: {:?}\n          Rules: {rules}\n",
                code_points.iter().map(|&x| Codepoint::new(x).sentence_break_class()).collect::<Vec<_>>()
            ));
        }
    }

    report("SentenceBreakTest.txt", passed, &failures);
}

// ── Targeted tests ────────────────────────────────────────────────────────

#[test]
fn empty_string_returns_no_segments() {
    let mut enumerator = SentenceBreakEnumerator::new(&[]);
    assert!(enumerator.move_next().is_none());
}

#[test]
fn single_char_returns_single_segment() {
    let text = utf16("A");
    let mut enumerator = SentenceBreakEnumerator::new(&text);
    let seg = enumerator.move_next().unwrap();
    assert_eq!(0, seg.offset());
    assert_eq!(1, seg.text().len());
    assert!(enumerator.move_next().is_none());
}

#[test]
fn simple_terminator_splits_at_sentence_boundary() {
    // "Hello. World" → ["Hello. ", "World"]
    let text = "Hello. World";
    let segments = collect_segments(text);
    assert_eq!(2, segments.len());
    assert_eq!("Hello. ", segments[0].text);
    assert_eq!("World", segments[1].text);
}

#[test]
fn exclamation_mark_splits_at_sentence_boundary() {
    let text = "Hello! World";
    let segments = collect_segments(text);
    assert_eq!(2, segments.len());
    assert_eq!("Hello! ", segments[0].text);
    assert_eq!("World", segments[1].text);
}

#[test]
fn question_mark_splits_at_sentence_boundary() {
    let text = "What? Well";
    let segments = collect_segments(text);
    assert_eq!(2, segments.len());
    assert_eq!("What? ", segments[0].text);
    assert_eq!("Well", segments[1].text);
}

#[test]
fn a_term_before_lower_no_break() {
    // "Hello. world":  SB8: ATerm × lowercase → no break
    // "Hello.  world": SB8: ATerm Sp* × lowercase → no break
    for text in ["Hello. world", "Hello.  world"] {
        // According to SB8, a period followed by lowercase should NOT split.
        let segments = collect_segments(text);
        assert_eq!(1, segments.len());
    }
}

#[test]
fn decimal_no_break() {
    // SB6: ATerm × Numeric → no break (2.5 stays together)
    let text = "The value is 2.5 meters.";
    let segments = collect_segments(text);
    // Should be 1 sentence (the decimal point is not a sentence terminator)
    assert_eq!(1, segments.len());
}

#[test]
fn abbreviation_sb7_no_break() {
    // SB7: (Upper | Lower) ATerm × Upper → no break
    // "U.S" — Upper (U) ATerm (.) Upper (S) must NOT break between '.' and 'S'.
    // Use "U.S. Treasury" where "Treasury" starts with Upper, so SB8 does NOT suppress
    // (SB8 requires a subsequent Lower; "Treasury" starts Upper which is a SB8 blocker).
    let text = "U.S. Treasury has funds.";
    let segments = collect_segments(text);
    // "U.S." followed by space and Upper ("Treasury") → SB11 fires → break before "Treasury"
    // The important thing: no spurious break between 'U', '.', 'S', '.' inside "U.S."
    assert_eq!(2, segments.len());
    assert!(segments[0].text.starts_with("U.S."));
}

#[test]
fn abbreviation_followed_by_lower_sb8_no_break() {
    // SB8: ATerm Close* Sp* × (¬(OLetter|Upper|Lower|ParaSep|SATerm))* Lower
    // "U.S.A. has" — ATerm + Sp + Lower("h") → SB8 suppresses the break.
    // Per UAX-29 this is ONE sentence.
    let text = "U.S.A. has";
    let segments = collect_segments(text);
    assert_eq!(1, segments.len());
}

#[test]
fn paragraph_separator_sep_splits_immediately() {
    // U+2029 PARAGRAPH SEPARATOR is SentenceBreakClass::Sep → SB4 break after it
    let text = "Hello\u{2029}World";
    let segments = collect_segments(text);
    assert_eq!(2, segments.len());
    assert_eq!("Hello\u{2029}", segments[0].text);
    assert_eq!("World", segments[1].text);
}

#[test]
fn cr_lf_treated_as_single_break() {
    // SB3: CR × LF (no break between them); SB4: break after LF
    let text = "Hello\r\nWorld";
    let segments = collect_segments(text);
    assert_eq!(2, segments.len());
    assert_eq!("Hello\r\n", segments[0].text);
    assert_eq!("World", segments[1].text);
}

#[test]
fn sb9_close_and_space_stay_with_terminator() {
    // SB9: (STerm | ATerm) Close* × (Close | Sp | Sep | CR | LF)
    // The closing quote and trailing space should stay with the sentence terminator.
    let text = "He said \"Hello.\" She replied.";
    let segments = collect_segments(text);
    // There are 2 sentences; the closing quote stays with the first.
    assert_eq!(2, segments.len());
    assert!(segments[0].text.contains("Hello.\""));
}

#[test]
fn surrogate_pair_handled_correctly() {
    // 𝄞 (U+1D11E MUSICAL SYMBOL G CLEF) is encoded as a surrogate pair.
    let text = "𝄞. Hi";
    let segments = collect_segments(text);
    // Should produce 2 segments; the surrogate pair is one codepoint
    assert_eq!(2, segments.len());
}

#[test]
fn all_segments_cover_whole_text() {
    let text = utf16("First sentence. Second sentence! Third?");
    let mut enumerator = SentenceBreakEnumerator::new(&text);
    let mut covered = 0;
    while let Some(segment) = enumerator.move_next() {
        assert_eq!(covered, segment.offset());
        covered += segment.text().len();
    }

    assert_eq!(text.len(), covered);
}

// ── Helpers ───────────────────────────────────────────────────────────────

struct SegmentInfo {
    #[allow(dead_code)] // part of the upstream helper record
    offset: usize,
    text: String,
}

fn collect_segments(text: &str) -> Vec<SegmentInfo> {
    let text = utf16(text);
    let mut result = Vec::new();
    let mut enumerator = SentenceBreakEnumerator::new(&text);
    while let Some(segment) = enumerator.move_next() {
        result.push(SegmentInfo { offset: segment.offset(), text: String::from_utf16(segment.text()).unwrap() });
    }

    result
}
