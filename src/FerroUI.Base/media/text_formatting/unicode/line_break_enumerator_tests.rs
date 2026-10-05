use super::ucd_test_data::{read_break_test_file, read_ucd_file, report, utf16, utf16_from_code_points};
use super::*;

#[test]
fn should_handle_empty_string() {
    let mut line_breaker = LineBreakEnumerator::new(&[]);

    assert!(line_breaker.move_next().is_none());
}

#[test]
fn basic_latin_test() {
    let text = utf16("Hello World\r\nThis is a test.");
    let mut line_breaker = LineBreakEnumerator::new(&text);

    let line_break = line_breaker.move_next().unwrap();
    assert_eq!(6, line_break.position_wrap());
    assert!(!line_break.required());

    let line_break = line_breaker.move_next().unwrap();
    assert_eq!(13, line_break.position_wrap());
    assert!(line_break.required());

    let line_break = line_breaker.move_next().unwrap();
    assert_eq!(18, line_break.position_wrap());
    assert!(!line_break.required());

    let line_break = line_breaker.move_next().unwrap();
    assert_eq!(21, line_break.position_wrap());
    assert!(!line_break.required());

    let line_break = line_breaker.move_next().unwrap();
    assert_eq!(23, line_break.position_wrap());
    assert!(!line_break.required());

    let line_break = line_breaker.move_next().unwrap();
    assert_eq!(28, line_break.position_wrap());
    assert!(!line_break.required());

    assert!(line_breaker.move_next().is_none());
}

#[test]
fn should_find_mandatory_breaks() {
    for (text, position_measure, position_wrap) in
        [("Hello\nWorld", 5usize, 6usize), ("Hello\rWorld", 5, 6), ("Hello\r\nWorld", 5, 7)]
    {
        let text = utf16(text);
        let line_breaker = LineBreakEnumerator::new(&text);

        let breaks = get_breaks(line_breaker);

        assert_eq!(2, breaks.len());

        let first_break = breaks[0];

        assert!(first_break.required());

        assert_eq!(position_measure, first_break.position_measure());

        assert_eq!(position_wrap, first_break.position_wrap());
    }
}

#[test]
fn forward_text_with_outer_whitespace() {
    let text = utf16(" Apples Pears Bananas   ");
    let line_breaker = LineBreakEnumerator::new(&text);
    let positions_f = get_breaks(line_breaker);

    assert_eq!(1, positions_f[0].position_wrap());
    assert_eq!(0, positions_f[0].position_measure());
    assert_eq!(8, positions_f[1].position_wrap());
    assert_eq!(7, positions_f[1].position_measure());
    assert_eq!(14, positions_f[2].position_wrap());
    assert_eq!(13, positions_f[2].position_measure());
    assert_eq!(24, positions_f[3].position_wrap());
    assert_eq!(21, positions_f[3].position_measure());
}

/// Conformance test against `auxiliary/LineBreakTest.txt`; skipped unless
/// `FERROUI_UCD_DIR` is set (see `ucd_test_data.rs`).
#[test]
fn should_find_breaks() {
    let Some(content) = read_ucd_file("auxiliary/LineBreakTest.txt") else {
        return;
    };

    let mut passed = 0;
    let mut failures = Vec::new();

    for (line_number, code_points, break_points, rules) in read_break_test_file(&content) {
        let text = utf16_from_code_points(&code_points);

        // The expected positions are in UTF-16 code units.
        let expected_breaks: Vec<usize> = break_points
            .iter()
            .map(|&count| code_points[..count].iter().map(|&x| if x > u16::MAX as u32 { 2 } else { 1 }).sum())
            .collect();

        let found_breaks: Vec<usize> = LineBreakEnumerator::new(&text).map(|line_break| line_break.position_wrap()).collect();

        if found_breaks == expected_breaks {
            passed += 1;
        } else {
            failures.push(format!(
                "Failed test on line {line_number}\n    Code Points: {code_points:X?}\nExpected Breaks: {expected_breaks:?}\n  Actual Breaks: {found_breaks:?}\n     Char Props: {:?}\n     Rules: {rules}\n",
                code_points.iter().map(|&x| Codepoint::new(x).line_break_class()).collect::<Vec<_>>()
            ));
        }
    }

    report("LineBreakTest.txt", passed, &failures);
}

fn get_breaks(mut line_breaker: LineBreakEnumerator<'_>) -> Vec<LineBreak> {
    let mut breaks = Vec::new();

    while let Some(line_break) = line_breaker.move_next() {
        breaks.push(line_break);
    }

    breaks
}
