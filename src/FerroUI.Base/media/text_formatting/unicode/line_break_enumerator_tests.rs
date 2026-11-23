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

/// The first break of `text` the enumerator gives as required.
fn first_required_enumerated(text: &[u16]) -> Option<LineBreak> {
    LineBreakEnumerator::new(text).find(|line_break| line_break.required())
}

#[test]
fn the_characters_of_a_required_break_are_those_of_the_hard_break_classes() {
    for value in (0..=0x10FFFF_u32).filter(|value| !(0xD800..=0xDFFF).contains(value)) {
        let class = Codepoint::new(value).line_break_class();
        let hard = matches!(
            class,
            LineBreakClass::MandatoryBreak
                | LineBreakClass::CarriageReturn
                | LineBreakClass::LineFeed
                | LineBreakClass::NextLine
        );
        let text = utf16_from_code_points(&[value]);
        assert_eq!(LineBreakEnumerator::first_required(&text).is_some(), hard, "U+{value:04X}");
    }
}

#[test]
fn the_first_required_break_is_the_one_the_enumerator_gives() {
    const PIECES: [&str; 24] = [
        "a", "word", " ", "  ", "\n", "\r", "\r\n", "\u{0B}", "\u{0C}", "\u{85}", "\u{2028}", "\u{2029}", "\u{301}",
        "\u{200D}", "\u{200B}", "\u{A0}", "\t", "-", "1", "\u{4E2D}", "\u{1F1F5}\u{1F1F1}", "\u{1F44D}\u{1F3FD}", ")",
        "\u{3000}",
    ];
    let mut seed = 0x9E37_79B9_7F4A_7C15_u64;
    let mut next = |below: usize| {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed % below as u64) as usize
    };
    let mut required = 0;
    for case in 0..40_000 {
        let pieces = next(14);
        // Half of the texts have their hard breaks late, behind a run of other characters.
        let hard_from = if case % 2 == 0 { 0 } else { pieces / 2 };
        let text: String = (0..pieces)
            .map(|piece| loop {
                let chosen = PIECES[next(PIECES.len())];
                let hard = chosen.chars().any(|c| matches!(c, '\n'..='\r' | '\u{85}' | '\u{2028}' | '\u{2029}'));
                if !hard || piece >= hard_from {
                    break chosen;
                }
            })
            .collect();
        let text = utf16(&text);
        let expected = first_required_enumerated(&text);
        required += usize::from(expected.is_some());
        assert_eq!(LineBreakEnumerator::first_required(&text), expected, "case {case}: {text:04X?}");
    }
    assert!(required > 10_000, "{required} texts have a required break");
}
