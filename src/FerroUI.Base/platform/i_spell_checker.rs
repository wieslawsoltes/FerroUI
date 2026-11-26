//! The spell checker of the platform (#1261).
//!
//! Not from upstream: WinUI reaches the spell checker of Windows through its text services;
//! FerroUI's text controls reach the checker of each platform through this contract, which
//! the application registers in the [`FerroLocator`](crate::FerroLocator) (the platform crates
//! provide the implementations; nothing registers one unless the application asks for it, so
//! an application that does not check spelling links none of them).

use crate::{FerroLocator, LocatorExtensions};
use std::rc::Rc;

/// A misspelled word of a checked text: its range in UTF-16 code units.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct SpellingError {
    /// The UTF-16 index of the first code unit of the word.
    pub start: usize,
    /// The number of UTF-16 code units of the word.
    pub length: usize,
}

impl SpellingError {
    /// A misspelled word that starts at `start` and is `length` UTF-16 code units long.
    pub const fn new(start: usize, length: usize) -> Self {
        Self { start, length }
    }

    /// The UTF-16 index after the last code unit of the word.
    pub const fn end(&self) -> usize {
        self.start + self.length
    }
}

/// The completion of [`ISpellChecker::check_spelling`]: the misspelled words of the text, in
/// the order of the text.
pub type SpellCheckCompletion = Box<dyn FnOnce(Vec<SpellingError>)>;

/// The spell checker of the platform.
///
/// Every method is called on the UI thread. A language is a BCP 47 tag (`en-US`); `None` is the
/// language of the user's settings.
pub trait ISpellChecker {
    /// Checks the spelling of `text` (UTF-16) and calls `completion` with its misspelled words,
    /// at once or later on the UI thread (a checker of the platform that answers
    /// asynchronously). A completion that is never called means that the text has no result.
    fn check_spelling(
        &self,
        text: Rc<[u16]>,
        language: Option<&str>,
        completion: SpellCheckCompletion,
    );

    /// The corrections the checker suggests for `word`, the best first.
    fn suggestions(&self, word: &str, language: Option<&str>) -> Vec<String>;

    /// Stops reporting `word` as misspelled for the rest of the session ("Ignore").
    fn ignore_word(&self, word: &str);

    /// Adds `word` to the user's dictionary ("Add to dictionary"), so that it is no longer
    /// reported as misspelled. A platform without a user dictionary ignores the word.
    fn add_to_dictionary(&self, word: &str);
}

/// The spell checker the application registered, or `None` when spell checking is not available
/// (the platform has no checker, or the application did not ask for it).
pub fn spell_checker() -> Option<Rc<dyn ISpellChecker>> {
    FerroLocator::current().get_service::<dyn ISpellChecker>()
}

/// Registers `checker` as the spell checker of the application.
pub fn register_spell_checker(checker: Rc<dyn ISpellChecker>) {
    FerroLocator::current_mutable()
        .bind::<dyn ISpellChecker>()
        .to_constant(checker);
}
