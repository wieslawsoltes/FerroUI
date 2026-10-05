use super::ucd_test_data::{read_break_test_file, read_ucd_file, report, utf16, utf16_from_code_points};
use super::*;

#[test]
fn should_report_codepoint_readouts() {
    let text = utf16("hello world");
    let mut word_breaker = WordBreakEnumerator::new(&text);
    let mut segments = Vec::new();

    while let Some(segment) = word_breaker.move_next() {
        segments.push((
            segment.offset(),
            segment.length(),
            segment.codepoint_offset(),
            segment.codepoint_length(),
            String::from_utf16(segment.text()).unwrap(),
        ));
    }

    assert_eq!(
        vec![
            (0, 5, 0, 5, "hello".to_string()),
            (5, 1, 5, 1, " ".to_string()),
            (6, 5, 6, 5, "world".to_string())
        ],
        segments
    );
}

#[test]
fn should_keep_readouts_consistent_with_text() {
    // Mixes a combining mark (WB4-absorbed), a surrogate pair and a CRLF so the
    // code-unit and code-point readouts diverge and every member is exercised.
    let text = utf16("a\u{0301}b \u{1D51E}x cr\r\nlf");
    let mut word_breaker = WordBreakEnumerator::new(&text);

    let mut offset = 0;
    let mut codepoint_offset = 0;

    while let Some(segment) = word_breaker.move_next() {
        assert_eq!(offset, segment.offset());
        assert_eq!(codepoint_offset, segment.codepoint_offset());
        assert_eq!(segment.text().len(), segment.length());
        assert_eq!(&text[segment.offset()..segment.offset() + segment.length()], segment.text());

        let mut codepoint_count = 0;
        let mut span = segment.text();
        while !span.is_empty() {
            let (_, consumed) = Codepoint::read_at(span, 0);
            span = &span[consumed..];
            codepoint_count += 1;
        }

        assert_eq!(codepoint_count, segment.codepoint_length());

        offset += segment.length();
        codepoint_offset += segment.codepoint_length();
    }

    assert_eq!(text.len(), offset);
}

/// Conformance test against `auxiliary/WordBreakTest.txt`; skipped unless
/// `FERROUI_UCD_DIR` is set (see `ucd_test_data.rs`).
#[test]
fn should_find_breaks() {
    let Some(content) = read_ucd_file("auxiliary/WordBreakTest.txt") else {
        return;
    };

    let mut passed = 0;
    let mut failures = Vec::new();

    for (line_number, code_points, break_points, rules) in read_break_test_file(&content) {
        let text = utf16_from_code_points(&code_points);
        let mut word_breaker = WordBreakEnumerator::new(&text);
        let mut found_breaks = vec![0];
        let mut current_position = 0;

        while let Some(segment) = word_breaker.move_next() {
            current_position += segment.codepoint_length();
            found_breaks.push(current_position);
        }

        if found_breaks == break_points {
            passed += 1;
        } else {
            failures.push(format!(
                "Failed test on line {line_number}\n    Code Points: {code_points:X?}\nExpected Breaks: {break_points:?}\n  Actual Breaks: {found_breaks:?}\n     Char Props: {:?}\n     Rules: {rules}\n",
                code_points.iter().map(|&x| Codepoint::new(x).word_break_class()).collect::<Vec<_>>()
            ));
        }
    }

    report("WordBreakTest.txt", passed, &failures);
}

// Regional indicators pair up from the start of their run (WB15, WB16), so where a
// boundary falls depends on how many of them precede the current one.
const REGIONAL_D: &str = "\u{1F1E9}";
const REGIONAL_E: &str = "\u{1F1EA}";
const REGIONAL_F: &str = "\u{1F1EB}";
const REGIONAL_R: &str = "\u{1F1F7}";

#[test]
fn two_flags_are_two_segments() {
    let segments = collect_segments(&[REGIONAL_D, REGIONAL_E, REGIONAL_F, REGIONAL_R].concat());

    assert_eq!(vec![[REGIONAL_D, REGIONAL_E].concat(), [REGIONAL_F, REGIONAL_R].concat()], segments);
}

#[test]
fn odd_regional_indicator_run_leaves_the_last_indicator_on_its_own() {
    let segments = collect_segments(&[REGIONAL_D, REGIONAL_E, REGIONAL_F].concat());

    assert_eq!(vec![[REGIONAL_D, REGIONAL_E].concat(), REGIONAL_F.to_string()], segments);
}

#[test]
fn regional_indicators_after_a_letter_pair_from_the_start_of_their_run() {
    let segments = collect_segments(&["A", REGIONAL_D, REGIONAL_E, REGIONAL_F, REGIONAL_R].concat());

    assert_eq!(
        vec!["A".to_string(), [REGIONAL_D, REGIONAL_E].concat(), [REGIONAL_F, REGIONAL_R].concat()],
        segments
    );
}

#[test]
fn regional_indicator_run_interrupted_by_space_starts_pairing_again() {
    let segments = collect_segments(&[REGIONAL_D, " ", REGIONAL_E, REGIONAL_F].concat());

    assert_eq!(vec![REGIONAL_D.to_string(), " ".to_string(), [REGIONAL_E, REGIONAL_F].concat()], segments);
}

#[test]
fn combining_mark_between_regional_indicators_does_not_split_the_pair() {
    let text = [REGIONAL_D, "\u{0301}", REGIONAL_E].concat();
    let segments = collect_segments(&text);

    assert_eq!(vec![text], segments);
}

#[test]
fn long_regional_indicator_run_pairs_all_the_way_through() {
    let flag = [REGIONAL_D, REGIONAL_E].concat();
    let segments = collect_segments(&flag.repeat(8));

    assert_eq!(8, segments.len());
    for segment in &segments {
        assert_eq!(&flag, segment);
    }
}

fn collect_segments(text: &str) -> Vec<String> {
    let text = utf16(text);
    let mut result = Vec::new();
    let mut enumerator = WordBreakEnumerator::new(&text);

    while let Some(segment) = enumerator.move_next() {
        result.push(String::from_utf16(&text[segment.offset()..segment.offset() + segment.length()]).unwrap());
    }

    result
}
