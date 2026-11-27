// Not from upstream: the reading of the results of the spell checker session of Android (#1261).
use super::*;
use std::cell::RefCell;

fn utf16(text: &str) -> Rc<[u16]> {
    text.encode_utf16().collect::<Vec<_>>().into()
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

/// A completion that records what it is called with.
fn recorder() -> (Rc<RefCell<Option<Vec<SpellingError>>>>, SpellCheckCompletion) {
    let result = Rc::new(RefCell::new(None));
    let slot = result.clone();
    (result, Box::new(move |errors| *slot.borrow_mut() = Some(errors)))
}

#[test]
fn a_result_gives_the_misspelled_words_with_their_suggestions() {
    // "I recieve teh mail": two typos, the second with one suggestion.
    let words = misspelled_words(
        18,
        &[2, 7, 10, 3],
        &[2, 1],
        &strings(&["receive", "relieve", "the"]),
    );
    assert_eq!(
        words,
        vec![
            MisspelledWord { error: SpellingError::new(2, 7), suggestions: strings(&["receive", "relieve"]) },
            MisspelledWord { error: SpellingError::new(10, 3), suggestions: strings(&["the"]) },
        ]
    );
}

#[test]
fn words_outside_the_text_or_empty_are_left_out_and_the_rest_sorted_once() {
    let words = misspelled_words(
        12,
        // Outside the text, empty, negative, a word, the same word again, an earlier word.
        &[10, 5, 3, 0, -1, 2, 6, 4, 6, 4, 0, 3],
        &[1, 0, 0, 1, 1, 0],
        &strings(&["far", "first", "again"]),
    );
    assert_eq!(
        words,
        vec![
            MisspelledWord { error: SpellingError::new(0, 3), suggestions: Vec::new() },
            MisspelledWord { error: SpellingError::new(6, 4), suggestions: strings(&["first"]) },
        ]
    );
}

#[test]
fn missing_counts_and_suggestions_give_words_without_suggestions() {
    let words = misspelled_words(20, &[0, 4, 5, 4, 10, 4], &[3], &strings(&["one", "two"]));
    assert_eq!(words.len(), 3);
    assert_eq!(words[0].suggestions, strings(&["one", "two"]));
    assert!(words[1].suggestions.is_empty());
    assert!(words[2].suggestions.is_empty());
    // A range without its length is not a word.
    assert!(misspelled_words(20, &[3], &[], &[]).is_empty());
}

#[test]
fn a_result_answers_the_check_with_its_sequence_number() {
    let mut checks = PendingChecks::default();
    let (first, first_completion) = recorder();
    let (second, second_completion) = recorder();
    let (third, third_completion) = recorder();
    assert_eq!(checks.push(utf16("one"), first_completion), 0);
    assert_eq!(checks.push(utf16("two"), second_completion), 1);
    assert_eq!(checks.push(utf16("three"), third_completion), 2);

    // The second is answered: the first was not answered by the session and is dropped.
    let answered = checks.take_answered(1).expect("the second check is answered");
    assert_eq!(&answered.text[..], &utf16("two")[..]);
    (answered.completion)(Vec::new());
    assert_eq!(*second.borrow(), Some(Vec::new()));
    assert_eq!(*first.borrow(), None);
    assert_eq!(checks.len(), 1);

    // A number that matches no check answers none.
    assert!(checks.take_answered(7).is_none());
    assert_eq!(checks.len(), 1);

    // A result without a number (no words) answers the oldest.
    let answered = checks.take_answered(-1).expect("the oldest check is answered");
    (answered.completion)(vec![SpellingError::new(0, 5)]);
    assert_eq!(*third.borrow(), Some(vec![SpellingError::new(0, 5)]));
    assert_eq!(checks.len(), 0);
    assert!(checks.take_answered(-1).is_none());
}

#[test]
fn a_check_that_could_not_be_sent_is_taken_back() {
    let mut checks = PendingChecks::default();
    let (_, first) = recorder();
    let (_, second) = recorder();
    checks.push(utf16("one"), first);
    let sequence = checks.push(utf16("two"), second);
    let removed = checks.remove(sequence).expect("the check is pending");
    assert_eq!(removed.sequence, sequence);
    assert_eq!(checks.len(), 1);
    assert!(checks.remove(sequence).is_none());
}

#[test]
fn sequence_numbers_are_never_negative() {
    let mut checks = PendingChecks { next_sequence: i32::MAX, pending: VecDeque::new() };
    let (_, first) = recorder();
    let (_, second) = recorder();
    assert_eq!(checks.push(utf16("a"), first), i32::MAX);
    assert_eq!(checks.push(utf16("b"), second), 0);
}

#[test]
fn ignored_words_are_left_out_and_suggestions_are_kept_for_the_menu() {
    let text = utf16("teh recieve teh");
    let mut words = CheckedWords::default();
    let result = misspelled_words(
        text.len(),
        &[0, 3, 4, 7, 12, 3],
        &[1, 1, 1],
        &strings(&["the", "receive", "the"]),
    );
    assert_eq!(
        words.errors(&text, result.clone()),
        vec![SpellingError::new(0, 3), SpellingError::new(4, 7), SpellingError::new(12, 3)]
    );
    assert_eq!(words.suggestions("recieve"), strings(&["receive"]));
    assert!(words.suggestions("unknown").is_empty());

    words.ignore("teh");
    assert!(words.suggestions("teh").is_empty());
    assert_eq!(words.errors(&text, result), vec![SpellingError::new(4, 7)]);
}

#[test]
fn the_kept_suggestions_start_again_when_full() {
    let mut words = CheckedWords::default();
    for index in 0..KEPT_SUGGESTIONS_LIMIT {
        let text = utf16(&format!("w{index}"));
        let result = vec![MisspelledWord {
            error: SpellingError::new(0, text.len()),
            suggestions: strings(&["s"]),
        }];
        words.errors(&text, result);
    }
    assert_eq!(words.suggestions("w0"), strings(&["s"]));
    let text = utf16("new");
    words.errors(&text, vec![MisspelledWord { error: SpellingError::new(0, 3), suggestions: strings(&["now"]) }]);
    assert!(words.suggestions("w0").is_empty());
    assert_eq!(words.suggestions("new"), strings(&["now"]));
}

#[test]
fn a_word_with_non_bmp_text_before_it_keeps_its_utf16_range() {
    // A character outside the basic plane is two UTF-16 code units: the word after it starts at 3.
    let text = utf16("\u{1D400} wrod");
    let mut words = CheckedWords::default();
    let result = misspelled_words(text.len(), &[3, 4], &[1], &strings(&["word"]));
    assert_eq!(words.errors(&text, result), vec![SpellingError::new(3, 4)]);
    assert_eq!(words.suggestions("wrod"), strings(&["word"]));
}
