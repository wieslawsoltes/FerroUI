//! The spell checker of iOS (#1261): UIKit's `UITextChecker` behind [`ISpellChecker`].
//!
//! Not from upstream (its iOS platform has no spell checker). Registered only by
//! [`IosSpellChecker::register`] (or `IosApplicationExtensions::use_platform_spell_checker`), so
//! an application that does not check spelling does not link it.

use ferroui_base::platform::{register_spell_checker, ISpellChecker, SpellCheckCompletion, SpellingError};
use objc2::rc::Retained;
use objc2::MainThreadMarker;
use objc2_foundation::{NSInteger, NSNotFound, NSRange, NSString};
use objc2_ui_kit::UITextChecker;
use std::rc::Rc;

/// The spell checker of iOS: a `UITextChecker` (the words it is told to ignore are its own).
pub struct IosSpellChecker {
    checker: Retained<UITextChecker>,
    mtm: MainThreadMarker,
}

impl IosSpellChecker {
    /// A text checker of UIKit. `None` off the main thread.
    pub fn new() -> Option<Self> {
        let mtm = MainThreadMarker::new()?;
        Some(Self { checker: UITextChecker::new(mtm), mtm })
    }

    /// Registers the spell checker of iOS as the spell checker of the application (on the main
    /// thread); whether it did.
    pub fn register() -> bool {
        match Self::new() {
            Some(checker) => {
                register_spell_checker(Rc::new(checker));
                true
            }
            None => false,
        }
    }

    /// The language of the checker for a BCP 47 tag: the tag in UIKit's form (`en_US`), its
    /// language alone, or else the first language of the user's keyboards UIKit has (UIKit
    /// needs a language).
    fn language(&self, language: Option<&str>) -> Option<Retained<NSString>> {
        let available: Vec<String> =
            UITextChecker::availableLanguages(self.mtm).iter().map(|name| name.to_string()).collect();
        let tag = language.unwrap_or_default().replace('-', "_");
        let primary = tag.split('_').next().unwrap_or_default().to_owned();
        [tag, primary]
            .into_iter()
            .filter(|candidate| !candidate.is_empty())
            .find_map(|candidate| available.iter().find(|name| name.eq_ignore_ascii_case(&candidate)).cloned())
            .or_else(|| available.first().cloned())
            .map(|name| NSString::from_str(&name))
    }
}

impl ISpellChecker for IosSpellChecker {
    fn check_spelling(&self, text: Rc<[u16]>, language: Option<&str>, completion: SpellCheckCompletion) {
        let Some(language) = self.language(language) else {
            completion(Vec::new());
            return;
        };
        // A lone surrogate becomes U+FFFD: one code unit for one, so the ranges stay the same.
        let string = NSString::from_str(&String::from_utf16_lossy(&text));
        let whole = NSRange::new(0, string.length());
        let mut errors = Vec::new();
        let mut offset: NSInteger = 0;
        while (offset as usize) < text.len() {
            let range = self
                .checker
                .rangeOfMisspelledWordInString_range_startingAt_wrap_language(&string, whole, offset, false, &language);
            if range.location == NSNotFound as usize || range.length == 0 {
                break;
            }
            errors.push(SpellingError::new(range.location, range.length));
            offset = (range.location + range.length) as NSInteger;
        }
        completion(errors);
    }

    fn suggestions(&self, word: &str, language: Option<&str>) -> Vec<String> {
        let Some(language) = self.language(language) else { return Vec::new() };
        let string = NSString::from_str(word);
        let range = NSRange::new(0, string.length());
        self.checker
            .guessesForWordRange_inString_language(range, &string, &language)
            .map(|guesses| guesses.iter().map(|guess| guess.to_string()).collect())
            .unwrap_or_default()
    }

    fn ignore_word(&self, word: &str) {
        self.checker.ignoreWord(&NSString::from_str(word));
    }

    fn add_to_dictionary(&self, word: &str) {
        UITextChecker::learnWord(&NSString::from_str(word), self.mtm);
    }
}
