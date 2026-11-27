//! The spell checker of Android (#1261): a `SpellCheckerSession` of the text services of the
//! system behind [`ISpellChecker`](ferroui_base::platform::ISpellChecker).
//!
//! Not from upstream (its Android platform has no spell checker). Registered only by
//! `AndroidSpellChecker::register` (or `AndroidApplicationExtensions::use_platform_spell_checker`),
//! so an application that does not check spelling does not link it, and the Java class of the
//! session (`SpellCheckerHelper`) is looked up and its native method registered only then.
//!
//! A session answers asynchronously: `getSentenceSuggestions` sends a text, and the listener of
//! the session receives its result later on the looper of the UI thread, which completes the
//! check. The suggestions of the misspelled words come with the result and are kept for
//! [`ISpellChecker::suggestions`], which is synchronous; the session has no ignored words, so the
//! words "Ignore" names are kept here and left out of the results. What reads the results runs on
//! every system, for the tests; the session itself exists on Android only.

use ferroui_base::platform::{SpellCheckCompletion, SpellingError};
use std::collections::{HashMap, HashSet, VecDeque};
use std::rc::Rc;

/// The number of suggestions a result has for each misspelled word (the text views of the system
/// ask for five).
pub(crate) const SUGGESTIONS_LIMIT: i32 = 5;

/// The most words whose suggestions are kept; the words start again when it is reached.
const KEPT_SUGGESTIONS_LIMIT: usize = 512;

/// A misspelled word of a result, with the corrections the spell checker suggests for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MisspelledWord {
    pub error: SpellingError,
    pub suggestions: Vec<String>,
}

/// Reads a result of `SpellCheckerHelper.onGetSentenceSuggestions` for a text of `text_length`
/// UTF-16 code units: `ranges` holds an offset and a length for each word that looks like a typo,
/// `counts` the number of its suggestions, and `suggestions` the suggestions of the words one
/// after the other. A word outside the text or without code units is left out; the words are in
/// the order of the text, each once.
pub(crate) fn misspelled_words(
    text_length: usize,
    ranges: &[i32],
    counts: &[i32],
    suggestions: &[String],
) -> Vec<MisspelledWord> {
    let mut words = Vec::with_capacity(ranges.len() / 2);
    let mut next_suggestion = 0usize;
    for (index, range) in ranges.chunks_exact(2).enumerate() {
        let count = counts.get(index).copied().unwrap_or(0).max(0) as usize;
        let end = (next_suggestion + count).min(suggestions.len());
        let word_suggestions = suggestions[next_suggestion.min(end)..end].to_vec();
        next_suggestion = end;
        let (Ok(start), Ok(length)) = (usize::try_from(range[0]), usize::try_from(range[1])) else {
            continue;
        };
        let error = SpellingError::new(start, length);
        if length == 0 || error.end() > text_length {
            continue;
        }
        words.push(MisspelledWord { error, suggestions: word_suggestions });
    }
    words.sort_by_key(|word| (word.error.start, word.error.length));
    words.dedup_by_key(|word| word.error);
    words
}

/// A text sent to a session that has not been answered yet.
pub(crate) struct PendingCheck {
    pub sequence: i32,
    pub text: Rc<[u16]>,
    pub completion: SpellCheckCompletion,
}

/// The checks sent to a session and not answered yet, in the order they were sent.
#[derive(Default)]
pub(crate) struct PendingChecks {
    next_sequence: i32,
    pending: VecDeque<PendingCheck>,
}

impl PendingChecks {
    /// Keeps a check that is being sent; its sequence number, which its result comes back with.
    pub fn push(&mut self, text: Rc<[u16]>, completion: SpellCheckCompletion) -> i32 {
        let sequence = self.next_sequence;
        // Never negative: -1 is the number of a result that has none.
        self.next_sequence = self.next_sequence.wrapping_add(1) & i32::MAX;
        self.pending.push_back(PendingCheck { sequence, text, completion });
        sequence
    }

    /// Takes back the check with `sequence` (a text that could not be sent).
    pub fn remove(&mut self, sequence: i32) -> Option<PendingCheck> {
        let index = self.pending.iter().position(|check| check.sequence == sequence)?;
        self.pending.remove(index)
    }

    /// The check a result answers: the one with its sequence number, or the oldest one when the
    /// result has no number (-1: it has no words to carry it). The checks sent before the one
    /// that is answered were not answered by the session and never will be: they are dropped
    /// (a check without a result). A result whose number matches no check answers none.
    pub fn take_answered(&mut self, sequence: i32) -> Option<PendingCheck> {
        if sequence < 0 {
            return self.pending.pop_front();
        }
        let index = self.pending.iter().position(|check| check.sequence == sequence)?;
        self.pending.drain(..index);
        self.pending.pop_front()
    }

    /// The number of checks not answered yet.
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.pending.len()
    }
}

/// The words a spell checker keeps across its sessions: the words it was told to ignore and the
/// suggestions of the misspelled words of the latest results.
#[derive(Default)]
pub(crate) struct CheckedWords {
    ignored: HashSet<String>,
    suggestions: HashMap<String, Vec<String>>,
}

impl CheckedWords {
    /// The misspelled words of a result for `text`, without the ignored ones; the suggestions of
    /// each word are kept for [`CheckedWords::suggestions`].
    pub fn errors(&mut self, text: &[u16], words: Vec<MisspelledWord>) -> Vec<SpellingError> {
        let mut errors = Vec::with_capacity(words.len());
        for word in words {
            let Some(units) = text.get(word.error.start..word.error.end()) else { continue };
            let spelling = String::from_utf16_lossy(units);
            if self.ignored.contains(&spelling) {
                continue;
            }
            if self.suggestions.len() >= KEPT_SUGGESTIONS_LIMIT && !self.suggestions.contains_key(&spelling) {
                self.suggestions.clear();
            }
            self.suggestions.insert(spelling, word.suggestions);
            errors.push(word.error);
        }
        errors
    }

    /// The suggestions the latest result that reported `word` had for it, the best first.
    pub fn suggestions(&self, word: &str) -> Vec<String> {
        self.suggestions.get(word).cloned().unwrap_or_default()
    }

    /// Leaves `word` out of the results from now on.
    pub fn ignore(&mut self, word: &str) {
        self.suggestions.remove(word);
        self.ignored.insert(word.to_owned());
    }
}

#[cfg(target_os = "android")]
pub use android::AndroidSpellChecker;
#[cfg(target_os = "android")]
pub(crate) use android::on_sentence_suggestions;

#[cfg(target_os = "android")]
mod android {
    use super::{misspelled_words, CheckedWords, PendingChecks, SUGGESTIONS_LIMIT};
    use crate::ferro_android_application::FerroAndroidApplication;
    use crate::interop::java::{
        call_boolean, call_static_boolean, call_static_object, call_void, JavaClass, JavaObject, JavaValue,
    };
    use crate::interop::natives::{next_handle, register_spell_checker_natives, SPELL_CHECKER_HELPER};
    use ferroui_base::platform::{register_spell_checker, ISpellChecker, SpellCheckCompletion};
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::{Rc, Weak};

    ferroui_base::ferro_thread_local! {
        /// The open sessions, by the number their listener calls back with.
        static SESSIONS: RefCell<HashMap<i64, Weak<Session>>> = RefCell::new(HashMap::new());
    }

    /// A session of the spell checker of the system for one language.
    struct Session {
        /// The `SpellCheckerHelper` of the session (its listener).
        helper: JavaObject,
        handle: i64,
        checks: RefCell<PendingChecks>,
        words: Rc<RefCell<CheckedWords>>,
    }

    impl Drop for Session {
        fn drop(&mut self) {
            let _ = SESSIONS.try_with(|sessions| sessions.borrow_mut().remove(&self.handle));
            call_void(&self.helper, "close", "()V", &[]);
        }
    }

    /// The spell checker of Android: the sessions of the spell checker of the system (one per
    /// language), opened by the first check in their language.
    pub struct AndroidSpellChecker {
        context: JavaObject,
        sessions: RefCell<HashMap<Option<String>, Rc<Session>>>,
        words: Rc<RefCell<CheckedWords>>,
    }

    impl AndroidSpellChecker {
        /// A spell checker of the application context. `None` before the application was
        /// created. Call on the UI thread.
        pub fn new() -> Option<Self> {
            let context = FerroAndroidApplication::try_context()?;
            register_spell_checker_natives();
            Some(Self { context, sessions: RefCell::new(HashMap::new()), words: Rc::default() })
        }

        /// Registers the spell checker of Android as the spell checker of the application (on
        /// the UI thread, once the application was created); whether it did.
        pub fn register() -> bool {
            match Self::new() {
                Some(checker) => {
                    register_spell_checker(Rc::new(checker));
                    true
                }
                None => false,
            }
        }

        /// The session for a language, opened the first time; `None` when the system has no
        /// spell checker or the user turned it off (asked again by the next check).
        fn session(&self, language: Option<&str>) -> Option<Rc<Session>> {
            let key = language.map(str::to_owned);
            if let Some(session) = self.sessions.borrow().get(&key) {
                return Some(session.clone());
            }
            let handle = next_handle();
            let tag = match language {
                Some(tag) => JavaValue::String(tag),
                None => JavaValue::Object(None),
            };
            let helper = call_static_object(
                &JavaClass::find(SPELL_CHECKER_HELPER),
                "open",
                "(Landroid/content/Context;Ljava/lang/String;J)Lorg/ferroui/android/SpellCheckerHelper;",
                &[JavaValue::Object(Some(&self.context)), tag, JavaValue::Long(handle)],
            )?;
            let session = Rc::new(Session {
                helper: helper.to_global(),
                handle,
                checks: RefCell::default(),
                words: self.words.clone(),
            });
            SESSIONS.with(|sessions| sessions.borrow_mut().insert(handle, Rc::downgrade(&session)));
            self.sessions.borrow_mut().insert(key, session.clone());
            Some(session)
        }
    }

    impl ISpellChecker for AndroidSpellChecker {
        fn check_spelling(&self, text: Rc<[u16]>, language: Option<&str>, completion: SpellCheckCompletion) {
            if text.is_empty() {
                completion(Vec::new());
                return;
            }
            let Some(session) = self.session(language) else {
                completion(Vec::new());
                return;
            };
            // A lone surrogate becomes U+FFFD: one code unit for one, so the ranges stay the same.
            let string = String::from_utf16_lossy(&text);
            let sequence = session.checks.borrow_mut().push(text, completion);
            let sent = call_boolean(
                &session.helper,
                "check",
                "(Ljava/lang/String;II)Z",
                &[JavaValue::String(&string), JavaValue::Int(sequence), JavaValue::Int(SUGGESTIONS_LIMIT)],
            );
            if !sent {
                let check = session.checks.borrow_mut().remove(sequence);
                if let Some(check) = check {
                    (check.completion)(Vec::new());
                }
            }
        }

        fn suggestions(&self, word: &str, _language: Option<&str>) -> Vec<String> {
            self.words.borrow().suggestions(word)
        }

        fn ignore_word(&self, word: &str) {
            self.words.borrow_mut().ignore(word);
        }

        fn add_to_dictionary(&self, word: &str) {
            // The dialog of the settings adds the word to the user's dictionary; until the spell
            // checker reads it, the word is ignored here.
            self.words.borrow_mut().ignore(word);
            call_static_boolean(
                &JavaClass::find(SPELL_CHECKER_HELPER),
                "addToDictionary",
                "(Landroid/content/Context;Ljava/lang/String;)Z",
                &[JavaValue::Object(Some(&self.context)), JavaValue::String(word)],
            );
        }
    }

    /// A result of the session with `handle` (`SpellCheckerHelper.onGetSentenceSuggestions`, on
    /// the UI thread): completes the check it answers.
    pub(crate) fn on_sentence_suggestions(
        handle: i64,
        sequence: i32,
        ranges: &[i32],
        counts: &[i32],
        suggestions: &[String],
    ) {
        let Some(session) = SESSIONS.with(|sessions| sessions.borrow().get(&handle).and_then(Weak::upgrade)) else {
            return;
        };
        let check = session.checks.borrow_mut().take_answered(sequence);
        let Some(check) = check else { return };
        let words = misspelled_words(check.text.len(), ranges, counts, suggestions);
        let errors = session.words.borrow_mut().errors(&check.text, words);
        (check.completion)(errors);
    }
}

#[cfg(test)]
#[path = "spell_checker_tests.rs"]
mod tests;
