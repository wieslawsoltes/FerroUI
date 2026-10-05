use super::ucd_test_data::{read_ucd_file, report};
use super::*;

/// One case of `BidiTest.txt`.
struct BidiTestCase {
    line_number: usize,
    classes: Vec<BidiClass>,
    paragraph_embedding_level: i8,
    levels: Vec<i32>,
}

/// The data generator of the conformance test (reads `BidiTest.txt`).
fn read_test_data(content: &str) -> Vec<BidiTestCase> {
    let mut test_data = Vec::new();

    // Process each line
    let mut levels: Vec<i32> = Vec::new();

    for (index, line) in content.lines().enumerate() {
        let line_number = index + 1;

        if line.starts_with('#') || line.is_empty() {
            continue;
        }

        // Directive?
        if line.starts_with('@') {
            if let Some(rest) = line.strip_prefix("@Levels:") {
                levels = rest
                    .trim()
                    .split(' ')
                    .filter(|x| !x.is_empty())
                    .map(|x| if x == "x" { -1 } else { x.parse().expect("a level") })
                    .collect();
            }

            continue;
        }

        // Split data line
        let mut parts = line.split(';');

        // Get the directions
        let directions: Vec<BidiClass> =
            parts.next().unwrap_or("").split(' ').map(PropertyValueAliasHelper::get_bidi_class).collect();

        // Get the bit set
        let bitset = i32::from_str_radix(parts.next().unwrap_or("").trim(), 16).expect("a bit set");

        let mut bit = 1;
        while bit < 8 {
            if (bitset & bit) == 0 {
                bit <<= 1;
                continue;
            }

            let paragraph_embedding_level: i8 = match bit {
                1 => 2, // Auto
                2 => 0, // LTR
                4 => 1, // RTL
                _ => unreachable!(),
            };

            test_data.push(BidiTestCase {
                line_number,
                classes: directions.clone(),
                paragraph_embedding_level,
                levels: levels.clone(),
            });

            // Like upstream, only the first paragraph level of the bit set is tested.
            break;
        }
    }

    test_data
}

/// Conformance test against `BidiTest.txt`; skipped unless
/// `FERROUI_UCD_DIR` is set (see `ucd_test_data.rs`).
#[test]
fn should_process() {
    let Some(content) = read_ucd_file("BidiTest.txt") else {
        return;
    };

    let mut bidi = BidiAlgorithm::new();
    let mut passed = 0;
    let mut failures = Vec::new();

    for case in read_test_data(&content) {
        let mut classes = case.classes.clone();

        // Run the algorithm...
        bidi.process_classes(&mut classes, &[], &[], case.paragraph_embedding_level, Some(false), None, None, None);

        let result_levels = bidi.resolved_levels();

        // Check the results match
        let pass = result_levels.len() == case.levels.len()
            && case.levels.iter().zip(result_levels).all(|(&expected, &actual)| expected == -1 || actual as i32 == expected);

        if pass {
            passed += 1;
        } else {
            failures.push(format!(
                "Failed line {}\n        Data: {:?}\n Embed Level: {}\n    Expected: {:?}\n      Actual: {:?}\n",
                case.line_number, case.classes, case.paragraph_embedding_level, case.levels, result_levels
            ));
        }
    }

    report("BidiTest.txt", passed, &failures);
}

/// Not upstream (its only test is the data driven one): a few of the worked
/// cases of UAX #9 so the algorithm is exercised without the UCD files.
#[test]
fn resolves_levels_of_simple_paragraphs() {
    use BidiClass::*;

    let mut bidi = BidiAlgorithm::new();

    // Pure LTR text.
    let mut classes = vec![LeftToRight, LeftToRight, WhiteSpace, LeftToRight];
    bidi.process_classes(&mut classes, &[], &[], 2, Some(false), None, None, None);
    assert_eq!(bidi.resolved_levels(), [0, 0, 0, 0]);
    assert_eq!(bidi.resolved_paragraph_embedding_level(), 0);

    // RTL text with a number: the number is level 2 in an RTL paragraph.
    let mut classes = vec![RightToLeft, RightToLeft, WhiteSpace, EuropeanNumber, EuropeanNumber];
    bidi.process_classes(&mut classes, &[], &[], 2, Some(false), None, None, None);
    assert_eq!(bidi.resolved_levels(), [1, 1, 1, 2, 2]);
    assert_eq!(bidi.resolved_paragraph_embedding_level(), 1);
    assert_eq!(bidi.resolve_embedding_level(&classes), 1);

    // LTR paragraph with an embedded RTL word; trailing whitespace returns to the paragraph level.
    let mut classes = vec![LeftToRight, WhiteSpace, RightToLeft, RightToLeft, WhiteSpace];
    bidi.process_classes(&mut classes, &[], &[], 0, Some(false), None, None, None);
    assert_eq!(bidi.resolved_levels(), [0, 0, 1, 1, 0]);

    // RLE ... PDF raises the embedded text to level 1; the removed controls take the neighbouring level.
    let mut classes = vec![LeftToRight, RightToLeftEmbedding, LeftToRight, PopDirectionalFormat, LeftToRight];
    bidi.process_classes(&mut classes, &[], &[], 0, Some(false), None, None, None);
    assert_eq!(bidi.resolved_levels(), [0, 0, 2, 2, 0]);

    // An RLI ... PDI isolate.
    let mut classes = vec![LeftToRight, RightToLeftIsolate, RightToLeft, PopDirectionalIsolate, LeftToRight];
    bidi.process_classes(&mut classes, &[], &[], 0, Some(false), None, None, None);
    assert_eq!(bidi.resolved_levels(), [0, 0, 1, 0, 0]);

    // The caller's buffer receives the levels when one is given.
    let mut classes = vec![RightToLeft, LeftToRight];
    let mut out_levels = [1i8, 1];
    bidi.process_classes(&mut classes, &[], &[], 1, Some(false), None, None, Some(&mut out_levels));
    assert_eq!(out_levels, [1, 2]);
    assert!(bidi.resolved_levels().is_empty());

    // An empty input resets the state.
    bidi.process_classes(&mut [], &[], &[], 2, None, None, None, None);
    assert!(bidi.resolved_levels().is_empty());
    assert_eq!(bidi.resolved_paragraph_embedding_level(), 0);
}
