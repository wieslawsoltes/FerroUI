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

fn levels_of(bidi: &mut BidiAlgorithm, classes: &[BidiClass], paragraph_embedding_level: i8) -> Vec<i8> {
    let mut classes = classes.to_vec();
    bidi.process_classes(&mut classes, &[], &[], paragraph_embedding_level, Some(false), None, None, None);
    bidi.resolved_levels().to_vec()
}

/// Not upstream: rule W2 by itself. A European number becomes an Arabic number
/// when the first strong class before it is an Arabic letter. Both kinds of
/// number take the same level, so each case ends in a European terminator,
/// which rule W5 joins to a European number only: after an Arabic number it
/// stays a neutral and takes the level of the paragraph.
#[test]
fn a_european_number_becomes_arabic_after_an_arabic_letter() {
    use BidiClass::*;

    let mut bidi = BidiAlgorithm::new();

    // The Arabic letter is the first strong class before the number.
    assert_eq!(levels_of(&mut bidi, &[ArabicLetter, EuropeanNumber, EuropeanTerminator], 0), [1, 2, 0]);

    // Another strong class is: the number stays European and takes the terminator with it.
    assert_eq!(levels_of(&mut bidi, &[RightToLeft, EuropeanNumber, EuropeanTerminator], 0), [1, 2, 2]);
    assert_eq!(levels_of(&mut bidi, &[LeftToRight, EuropeanNumber, EuropeanTerminator], 0), [0, 0, 0]);

    // Nothing strong before the number: the start of the sequence is never an Arabic letter.
    assert_eq!(levels_of(&mut bidi, &[EuropeanNumber, EuropeanTerminator, ArabicLetter], 0), [0, 0, 1]);

    // The nearest strong class decides, not an earlier one.
    assert_eq!(
        levels_of(&mut bidi, &[ArabicLetter, LeftToRight, EuropeanNumber, EuropeanTerminator], 0),
        [1, 0, 0, 0]
    );
    assert_eq!(
        levels_of(&mut bidi, &[ArabicLetter, LeftToRight, ArabicLetter, EuropeanNumber, EuropeanTerminator], 0),
        [1, 0, 1, 2, 0]
    );

    // A number the rule has changed is not strong: the one after it still sees the letter.
    assert_eq!(
        levels_of(&mut bidi, &[ArabicLetter, EuropeanNumber, EuropeanNumber, EuropeanTerminator], 0),
        [1, 2, 2, 0]
    );

    // Neutrals and the marks rule W1 resolved lie between the letter and the number.
    assert_eq!(
        levels_of(&mut bidi, &[ArabicLetter, WhiteSpace, OtherNeutral, EuropeanNumber, EuropeanTerminator], 0),
        [1, 1, 1, 2, 0]
    );
    assert_eq!(
        levels_of(&mut bidi, &[ArabicLetter, NonspacingMark, EuropeanNumber, EuropeanTerminator], 0),
        [1, 1, 2, 0]
    );

    // An isolate is a sequence of its own: the number in it does not see the letter before
    // the isolate, the number after it does.
    assert_eq!(
        levels_of(
            &mut bidi,
            &[
                ArabicLetter,
                RightToLeftIsolate,
                EuropeanNumber,
                EuropeanTerminator,
                PopDirectionalIsolate,
                EuropeanNumber,
                EuropeanTerminator
            ],
            0
        ),
        [1, 1, 2, 2, 1, 2, 0]
    );

    // A right to left paragraph: the number before the letter stays European.
    assert_eq!(
        levels_of(
            &mut bidi,
            &[EuropeanNumber, EuropeanTerminator, ArabicLetter, EuropeanNumber, EuropeanTerminator],
            1
        ),
        [2, 2, 1, 2, 1]
    );
}

/// Not upstream: a paragraph of numbers and spaces, which has no strong class
/// for rule W2 to stop at. The rule is one pass over the run; a search backward
/// from each number takes minutes on this input.
#[test]
fn resolves_a_long_paragraph_of_numbers() {
    use BidiClass::*;

    const WORDS: usize = 50_000;
    let word = [EuropeanNumber, EuropeanNumber, EuropeanNumber, EuropeanNumber, EuropeanNumber, WhiteSpace];
    let numbers: Vec<BidiClass> = word.iter().copied().cycle().take(WORDS * word.len()).collect();

    let mut bidi = BidiAlgorithm::new();

    // Left to right: every number takes the direction of the paragraph.
    let levels = levels_of(&mut bidi, &numbers, 0);
    assert_eq!(levels.len(), numbers.len());
    assert!(levels.iter().all(|&level| level == 0));

    // Right to left: the numbers are two levels up, the spaces between them one.
    let levels = levels_of(&mut bidi, &numbers, 1);
    assert!(levels.chunks(word.len()).all(|levels| levels == [2, 2, 2, 2, 2, 1]));

    // An Arabic letter at the start reaches every number of the paragraph. The space at the
    // end returns to the level of the paragraph.
    let mut after_letter = vec![ArabicLetter];
    after_letter.extend_from_slice(&numbers);
    let levels = levels_of(&mut bidi, &after_letter, 0);
    assert_eq!(levels[0], 1);
    let (last, words) = levels[1..].split_last().expect("the paragraph has words");
    assert_eq!(*last, 0);
    assert!(words.chunks(word.len()).all(|levels| levels.iter().take(5).all(|&level| level == 2)));
    assert!(words.chunks_exact(word.len()).all(|levels| levels[5] == 1));
}

/// Not upstream: a paragraph of many isolates and of many embeddings, which is
/// a paragraph of many level runs. Each run is marked when a sequence takes it
/// and the run a matching isolate end starts is found by halving; removing the
/// runs from the list and reading the list for each isolate grows with the
/// square of their count.
#[test]
fn resolves_a_long_paragraph_of_isolates_and_embeddings() {
    use BidiClass::*;

    const WORDS: usize = 50_000;
    let repeat = |word: &[BidiClass]| -> Vec<BidiClass> { word.iter().copied().cycle().take(WORDS * word.len()).collect() };

    let mut bidi = BidiAlgorithm::new();

    // The isolates and their ends are neutrals between the letters of the paragraph; the
    // letter in each isolate is a sequence of its own, one level up.
    let isolates = repeat(&[LeftToRight, RightToLeftIsolate, RightToLeft, PopDirectionalIsolate]);
    let levels = levels_of(&mut bidi, &isolates, 0);
    assert_eq!(levels.len(), isolates.len());
    assert!(levels.chunks(4).all(|levels| levels == [0, 0, 1, 0]));

    let isolates = repeat(&[RightToLeft, LeftToRightIsolate, LeftToRight, PopDirectionalIsolate]);
    let levels = levels_of(&mut bidi, &isolates, 1);
    assert!(levels.chunks(4).all(|levels| levels == [1, 1, 2, 1]));

    // An isolate in an isolate: the sequence of the outer one continues after the inner one.
    let nested = repeat(&[
        LeftToRight,
        RightToLeftIsolate,
        RightToLeft,
        LeftToRightIsolate,
        LeftToRight,
        PopDirectionalIsolate,
        RightToLeft,
        PopDirectionalIsolate,
    ]);
    let levels = levels_of(&mut bidi, &nested, 0);
    assert!(levels.chunks(8).all(|levels| levels == [0, 0, 1, 1, 2, 1, 1, 0]));

    // The embedding controls take a level from their neighbours; the letters are what is read.
    let embeddings = repeat(&[LeftToRight, RightToLeftEmbedding, RightToLeft, PopDirectionalFormat]);
    let levels = levels_of(&mut bidi, &embeddings, 0);
    assert_eq!(levels.len(), embeddings.len());
    assert!(levels.chunks(4).all(|levels| levels[0] == 0 && levels[2] == 1));
}

/// The classes that make a level other than 0 in a left-to-right paragraph.
fn is_right_to_left_class(class: BidiClass) -> bool {
    matches!(
        class,
        BidiClass::RightToLeft
            | BidiClass::ArabicLetter
            | BidiClass::ArabicNumber
            | BidiClass::LeftToRightEmbedding
            | BidiClass::RightToLeftEmbedding
            | BidiClass::LeftToRightOverride
            | BidiClass::RightToLeftOverride
            | BidiClass::PopDirectionalFormat
            | BidiClass::LeftToRightIsolate
            | BidiClass::RightToLeftIsolate
            | BidiClass::FirstStrongIsolate
            | BidiClass::PopDirectionalIsolate
    )
}

#[test]
fn no_character_before_the_hebrew_block_has_a_right_to_left_class() {
    let mut data = BidiData::new();
    for unit in 0..BidiAlgorithm::FIRST_RIGHT_TO_LEFT {
        data.reset();
        data.append(&[unit]);
        assert!(!is_right_to_left_class(data.classes()[0]), "U+{unit:04X} is {:?}", data.classes()[0]);
    }
    assert!(BidiAlgorithm::is_left_to_right_text(&[]));
    assert!(BidiAlgorithm::is_left_to_right_text(&[BidiAlgorithm::FIRST_RIGHT_TO_LEFT - 1; 200]));
    for at in [0, 63, 64, 199] {
        let mut text = [0x0041; 200];
        text[at] = BidiAlgorithm::FIRST_RIGHT_TO_LEFT;
        assert!(!BidiAlgorithm::is_left_to_right_text(&text), "a Hebrew code unit at {at}");
    }
}

#[test]
fn a_left_to_right_text_has_the_levels_the_rules_give() {
    // Texts of code units below the Hebrew block: letters, numbers with their separators and
    // terminators, brackets, marks, spaces, separators of paragraphs and segments, controls.
    const PIECES: &[&str] = &[
        "word", "A", "12", "3.5", "1,000", "+", "-", "$", "%", "#", "(", ")", "[", "]", "{", "}", "<", ">", " ", "  ",
        "\t", "\n", "\r\n", "\u{1c}", "\u{1f}", "\u{85}", "\u{a0}", "\u{ad}", "\u{301}", "\u{483}", "\u{b2}",
        "\u{b0}", "\u{e9}", "\u{3b1}", "\u{416}", "\u{2b9}", "/", ":", ";", ".", ",", "!", "\u{0}", "\u{7f}",
        "\u{58f}", "\u{58a}",
    ];
    let mut state = 0x2545_F491_4F6C_DD1Du64;
    let mut next = move |below: usize| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state % below as u64) as usize
    };
    let mut data = BidiData::new();
    let mut bidi = BidiAlgorithm::new();
    let mut fast = BidiAlgorithm::new();
    for case in 0..20_000 {
        let count = 1 + next(40);
        let text: String = (0..count).map(|_| PIECES[next(PIECES.len())]).collect();
        let units: Vec<u16> = text.encode_utf16().collect();
        assert!(BidiAlgorithm::is_left_to_right_text(&units), "case {case}: {text:?}");

        data.reset();
        data.set_paragraph_embedding_level(0);
        data.append(&units);
        bidi.process(&mut data);
        fast.process_left_to_right(units.len());

        assert_eq!(fast.resolved_levels(), bidi.resolved_levels(), "case {case}: {text:?}");
        assert_eq!(bidi.resolve_embedding_level(data.classes()), 0, "case {case}: {text:?}");
    }
}
