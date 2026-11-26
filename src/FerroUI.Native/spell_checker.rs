//! The spell checker of macOS (#1261): AppKit's shared `NSSpellChecker`, one spell document per
//! checker (the words it is told to ignore), behind [`ISpellChecker`].
//!
//! Not from upstream (its macOS platform has no spell checker). Registered only by
//! [`MacOSSpellChecker::register`] (the desktop entry point's `use_platform_spell_checker`), so an
//! application that does not check spelling does not link it.

use ferroui_base::platform::{register_spell_checker, ISpellChecker, SpellCheckCompletion, SpellingError};
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{class, msg_send};
use objc2_foundation::{NSArray, NSInteger, NSNotFound, NSRange, NSString};
use std::rc::Rc;

/// The spell checker of macOS: `NSSpellChecker`.
pub struct MacOSSpellChecker {
    checker: Retained<AnyObject>,
    /// The spell document of the checker (`uniqueSpellDocumentTag`): the words ignored in it.
    tag: NSInteger,
}

impl MacOSSpellChecker {
    /// The shared spell checker of AppKit with a spell document of its own. Call on the main
    /// thread.
    pub fn new() -> Self {
        // SAFETY: class methods of AppKit's NSSpellChecker without arguments.
        let checker: Retained<AnyObject> = unsafe { msg_send![class!(NSSpellChecker), sharedSpellChecker] };
        let tag: NSInteger = unsafe { msg_send![class!(NSSpellChecker), uniqueSpellDocumentTag] };
        Self { checker, tag }
    }

    /// Registers the spell checker of macOS as the spell checker of the application.
    pub fn register() {
        register_spell_checker(Rc::new(Self::new()));
    }

    /// The language of the checker for a BCP 47 tag: the tag in AppKit's form (`en_US`), or its
    /// language alone (`en`), if the checker has a dictionary for it; else `None` (the language
    /// of the user's settings).
    fn language(&self, language: Option<&str>) -> Option<Retained<NSString>> {
        let language = language?.replace('-', "_");
        // SAFETY: an instance method without arguments that returns an array of strings.
        let available: Retained<NSArray<NSString>> = unsafe { msg_send![&*self.checker, availableLanguages] };
        let available: Vec<String> = available.iter().map(|name| name.to_string()).collect();
        let primary = language.split('_').next().unwrap_or_default().to_owned();
        [language, primary]
            .into_iter()
            .find(|candidate| available.iter().any(|name| name.eq_ignore_ascii_case(candidate)))
            .map(|candidate| NSString::from_str(&candidate))
    }
}

impl Default for MacOSSpellChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for MacOSSpellChecker {
    fn drop(&mut self) {
        // SAFETY: closes the spell document this checker opened.
        let _: () = unsafe { msg_send![&*self.checker, closeSpellDocumentWithTag: self.tag] };
    }
}

impl ISpellChecker for MacOSSpellChecker {
    fn check_spelling(&self, text: Rc<[u16]>, language: Option<&str>, completion: SpellCheckCompletion) {
        // A lone surrogate becomes U+FFFD: one code unit for one, so the ranges stay the same.
        let string = NSString::from_str(&String::from_utf16_lossy(&text));
        let language = self.language(language);
        let language = language.as_deref();
        let mut errors = Vec::new();
        let mut offset: NSInteger = 0;
        while (offset as usize) < text.len() {
            // SAFETY: `string` and `language` are live; the word count pointer may be null.
            let range: NSRange = unsafe {
                msg_send![
                    &*self.checker,
                    checkSpellingOfString: &*string,
                    startingAt: offset,
                    language: language,
                    wrap: false,
                    inSpellDocumentWithTag: self.tag,
                    wordCount: std::ptr::null_mut::<NSInteger>()
                ]
            };
            if range.location == NSNotFound as usize || range.length == 0 {
                break;
            }
            errors.push(SpellingError::new(range.location, range.length));
            offset = (range.location + range.length) as NSInteger;
        }
        completion(errors);
    }

    fn suggestions(&self, word: &str, language: Option<&str>) -> Vec<String> {
        let string = NSString::from_str(word);
        let language = self.language(language);
        let range = NSRange::new(0, string.length());
        // SAFETY: `string` and `language` are live; the result is an array of strings or nil.
        let guesses: Option<Retained<NSArray<NSString>>> = unsafe {
            msg_send![
                &*self.checker,
                guessesForWordRange: range,
                inString: &*string,
                language: language.as_deref(),
                inSpellDocumentWithTag: self.tag
            ]
        };
        guesses.map(|guesses| guesses.iter().map(|guess| guess.to_string()).collect()).unwrap_or_default()
    }

    fn ignore_word(&self, word: &str) {
        let string = NSString::from_str(word);
        // SAFETY: `string` is live; the tag is the spell document of this checker.
        let _: () = unsafe { msg_send![&*self.checker, ignoreWord: &*string, inSpellDocumentWithTag: self.tag] };
    }

    fn add_to_dictionary(&self, word: &str) {
        let string = NSString::from_str(word);
        // SAFETY: `string` is live.
        let _: () = unsafe { msg_send![&*self.checker, learnWord: &*string] };
    }
}
