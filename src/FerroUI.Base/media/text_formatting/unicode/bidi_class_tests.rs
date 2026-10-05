use super::ucd_test_data::{read_ucd_file, report, utf16, utf16_from_utf32};
use super::*;

/// One case of `BidiCharacterTest.txt`.
struct BidiClassData {
    line_number: usize,
    code_points: Vec<u32>,
    paragraph_level: i8,
    resolved_paragraph_level: i8,
    resolved_levels: Vec<i8>,
    #[allow(dead_code)] // parsed like upstream, which does not check the order either
    resolved_order: Vec<i32>,
}

/// The data generator of the conformance test (reads `BidiCharacterTest.txt`).
fn read_data(content: &str) -> Vec<BidiClassData> {
    let mut test_data = Vec::new();

    // Process each line
    for (index, line) in content.lines().enumerate() {
        let line_number = index + 1;

        if line.starts_with('#') || line.is_empty() {
            continue;
        }

        // Split into fields
        let fields: Vec<&str> = line.split(';').collect();

        let items = |field: &str| -> Vec<String> {
            field.split(' ').map(str::trim).filter(|x| !x.is_empty()).map(str::to_string).collect()
        };

        // Parse field 0 - code points
        let code_points = items(fields[0]).iter().map(|x| u32::from_str_radix(x, 16).expect("a code point")).collect();

        // Parse field 1 - paragraph level
        let paragraph_level = fields[1].trim().parse().expect("a paragraph level");

        // Parse field 2 - resolved paragraph level
        let resolved_paragraph_level = fields[2].trim().parse().expect("a resolved paragraph level");

        // Parse field 3 - resolved levels
        let resolved_levels =
            items(fields[3]).iter().map(|x| if x == "x" { -1 } else { x.parse().expect("a level") }).collect();

        // Parse field 4 - resolved levels
        let resolved_order = items(fields[4]).iter().map(|x| x.parse().expect("an index")).collect();

        test_data.push(BidiClassData {
            line_number,
            code_points,
            paragraph_level,
            resolved_paragraph_level,
            resolved_levels,
            resolved_order,
        });
    }

    test_data
}

/// Conformance test against `BidiCharacterTest.txt`; skipped unless
/// `FERROUI_UCD_DIR` is set (see `ucd_test_data.rs`).
#[test]
fn should_resolve() {
    let Some(content) = read_ucd_file("BidiCharacterTest.txt") else {
        return;
    };

    let mut passed = 0;
    let mut failures = Vec::new();

    for case in read_data(&content) {
        let mut bidi = BidiAlgorithm::new();
        let mut bidi_data = BidiData::new();
        bidi_data.set_paragraph_embedding_level(case.paragraph_level);

        let text = utf16_from_utf32(&case.code_points);

        // Append
        bidi_data.append(&text);

        // Act
        for _ in 0..10 {
            bidi.process(&mut bidi_data);
        }

        let result_levels = bidi.resolved_levels();
        let result_paragraph_level = bidi.resolved_paragraph_embedding_level();

        let mut pass = case.resolved_paragraph_level as i32 == result_paragraph_level;

        for (i, &expected_level) in case.resolved_levels.iter().enumerate() {
            if expected_level == -1 {
                continue;
            }

            if result_levels.get(i) != Some(&expected_level) {
                pass = false;
            }
        }

        if pass {
            passed += 1;
        } else {
            failures.push(format!(
                "Failed line {}\n Code Points: {:X?}\n Embed Level: {}\n    Expected: {} {:?}\n      Actual: {} {:?}\n",
                case.line_number,
                case.code_points,
                case.paragraph_level,
                case.resolved_paragraph_level,
                case.resolved_levels,
                result_paragraph_level,
                result_levels
            ));
        }
    }

    report("BidiCharacterTest.txt", passed, &failures);
}

/// Not upstream (its only test is the data driven one): `BidiData` and
/// `BidiAlgorithm` together on real text, including reuse after `reset`.
#[test]
fn resolves_levels_of_text() {
    let mut bidi = BidiAlgorithm::new();
    let mut bidi_data = BidiData::new();

    // "abc (אבג) 12": Hebrew inside brackets in an LTR paragraph.
    let text = utf16("abc (\u{05D0}\u{05D1}\u{05D2}) 12");
    bidi_data.set_paragraph_embedding_level(2);
    bidi_data.append(&text);

    assert_eq!(text.len(), bidi_data.length());
    assert_eq!(Some(true), bidi_data.has_brackets());
    assert_eq!(None, bidi_data.has_embeddings());
    assert_eq!(BidiPairedBracketType::Open, bidi_data.paired_bracket_types()[4]);
    assert_eq!(BidiPairedBracketType::Close, bidi_data.paired_bracket_types()[8]);
    assert_eq!(bidi_data.paired_bracket_values()[4], bidi_data.paired_bracket_values()[8]);

    bidi.process(&mut bidi_data);

    assert_eq!(0, bidi.resolved_paragraph_embedding_level());
    // The brackets resolve to the paragraph direction (N0); the digits keep EN
    // after the Hebrew (W7) and so end up at level 2 (I1).
    assert_eq!(bidi.resolved_levels(), [0, 0, 0, 0, 0, 1, 1, 1, 0, 0, 2, 2]);
    assert_eq!(0, bidi.resolve_embedding_level(bidi_data.classes()));

    // A surrogate pair is one entry.
    bidi_data.reset();
    bidi.reset();
    assert!(bidi.resolved_levels().is_empty());
    assert_eq!(0, bidi_data.length());
    assert_eq!(Some(false), bidi_data.has_brackets());

    let text = utf16("\u{05D0}\u{1F600}a");
    bidi_data.set_paragraph_embedding_level(2);
    bidi_data.append(&text);
    assert_eq!(3, bidi_data.length());
    assert_eq!(bidi_data.classes(), [BidiClass::RightToLeft, BidiClass::OtherNeutral, BidiClass::LeftToRight]);

    bidi.process(&mut bidi_data);
    assert_eq!(1, bidi.resolved_paragraph_embedding_level());
    assert_eq!(bidi.resolved_levels(), [1, 1, 2]);

    // Saved types survive an override of the classes.
    bidi_data.save_types();
    bidi_data.classes_mut().fill(BidiClass::OtherNeutral);
    bidi_data.restore_types();
    assert_eq!(bidi_data.classes(), [BidiClass::RightToLeft, BidiClass::OtherNeutral, BidiClass::LeftToRight]);
    assert_eq!(4, bidi_data.get_temp_level_buffer(4).len());
}
