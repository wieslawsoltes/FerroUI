//! The spell checker of macOS (#1261) on the main thread: AppKit's NSSpellChecker finds the
//! misspelled words of a text in UTF-16 ranges, suggests corrections, and stops reporting a word
//! once it is ignored in the checker's spell document.

#[cfg(not(target_os = "macos"))]
fn main() {
    println!("spell_checker: macOS only, skipped");
}

#[cfg(target_os = "macos")]
fn main() {
    use ferroui_base::platform::{ISpellChecker, SpellingError};
    use ferroui_native::MacOSSpellChecker;
    use std::cell::RefCell;
    use std::rc::Rc;

    let checker = MacOSSpellChecker::new();
    let check = |text: &str| {
        let result = Rc::new(RefCell::new(None));
        let units: Rc<[u16]> = text.encode_utf16().collect::<Vec<_>>().into();
        let sink = result.clone();
        checker.check_spelling(units, Some("en-US"), Box::new(move |errors| *sink.borrow_mut() = Some(errors)));
        let errors = result.borrow_mut().take();
        errors.expect("NSSpellChecker answers at once")
    };

    // "ä" before the misspelled word is one UTF-16 unit, the emoji two.
    let text = "The quick brwon fox j\u{00e4}mps over the lazzy dog \u{1F600} recieve";
    let errors = check(text);
    let words: Vec<String> = errors
        .iter()
        .map(|error: &SpellingError| {
            let units: Vec<u16> = text.encode_utf16().collect();
            String::from_utf16_lossy(&units[error.start..error.end()])
        })
        .collect();
    for word in ["brwon", "lazzy", "recieve"] {
        assert!(words.iter().any(|w| w == word), "{word} is reported in {words:?}");
    }
    assert!(!words.iter().any(|w| w == "quick" || w == "fox"), "{words:?}");

    let suggestions = checker.suggestions("recieve", Some("en-US"));
    assert!(suggestions.iter().any(|s| s == "receive"), "{suggestions:?}");
    // An unknown language falls back to the language of the user's settings.
    let _ = checker.suggestions("recieve", Some("zz-ZZ"));

    checker.ignore_word("brwon");
    let words_after: Vec<SpellingError> = check("brwon fox");
    assert!(words_after.is_empty(), "an ignored word is not reported: {words_after:?}");
    println!("spell_checker: ok ({} errors, {} suggestions)", errors.len(), suggestions.len());
}
