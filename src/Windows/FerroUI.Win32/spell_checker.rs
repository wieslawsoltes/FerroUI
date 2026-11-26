//! The spell checker of Windows (#1261): `ISpellCheckerFactory` of the spell checking API
//! (Windows 8 and later), one `ISpellChecker` per language, behind [`ISpellChecker`].
//!
//! Not from upstream (its Windows platform has no spell checker; WinUI's text controls reach the
//! same service through the text services of RichEdit). Registered only by
//! [`Win32SpellChecker::register`] (the desktop entry point's `use_platform_spell_checker`), so an
//! application that does not check spelling does not link it.

use crate::interop::unmanaged_methods::{co_create_instance, co_task_mem_free};
use crate::win32_com::spellcheck::{IEnumSpellingError, IEnumString, ISpellChecker as ISystemSpellChecker};
use crate::win32_com::spellcheck::{ISpellCheckerFactory, ISpellingError};
use ferroui_base::platform::{register_spell_checker, ISpellChecker, SpellCheckCompletion, SpellingError};
use ferroui_microcom::{ComPtr, Guid, Interface};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// `CLSID_SpellCheckerFactory`.
const CLSID_SPELL_CHECKER_FACTORY: Guid = Guid::from_u128(0x7AB36653_1796_484B_BDFA_E74F1DB7C1DC);
/// `CLSCTX_INPROC_SERVER`.
const CLSCTX_INPROC_SERVER: u32 = 1;
/// `CORRECTIVE_ACTION_DELETE`: a repeated word, not a misspelled one.
const CORRECTIVE_ACTION_DELETE: i32 = 3;

/// The spell checker of Windows.
pub struct Win32SpellChecker {
    factory: ComPtr<ISpellCheckerFactory>,
    /// The checker of each language asked for (the empty tag: the language of the user's
    /// settings), `None` for a language Windows has no checker for.
    checkers: RefCell<HashMap<String, Option<ComPtr<ISystemSpellChecker>>>>,
}

impl Win32SpellChecker {
    /// The spell checker factory of Windows, or `None` before Windows 8 or when COM is not
    /// initialized on the thread (the platform initializes OLE on the UI thread).
    pub fn new() -> Option<Self> {
        let instance =
            co_create_instance(&CLSID_SPELL_CHECKER_FACTORY, CLSCTX_INPROC_SERVER, &ISpellCheckerFactory::IID).ok()?;
        // SAFETY: the pointer was just returned for the identifier of the interface, with a
        // reference the value takes over.
        let factory = unsafe { ComPtr::<ISpellCheckerFactory>::from_raw(instance.cast()) }?;
        Some(Self { factory, checkers: RefCell::new(HashMap::new()) })
    }

    /// Registers the spell checker of Windows as the spell checker of the application, if
    /// Windows has one; whether it did.
    pub fn register() -> bool {
        match Self::new() {
            Some(checker) => {
                register_spell_checker(Rc::new(checker));
                true
            }
            None => false,
        }
    }

    /// The checker of a language: the tag itself, its language alone, or the language of the
    /// user's settings (the first language of the factory) when Windows has none for it.
    fn checker(&self, language: Option<&str>) -> Option<ComPtr<ISystemSpellChecker>> {
        let key = language.unwrap_or_default().to_owned();
        if let Some(checker) = self.checkers.borrow().get(&key) {
            return checker.clone();
        }
        let primary = key.split('-').next().unwrap_or_default().to_owned();
        let checker = [key.clone(), primary]
            .into_iter()
            .filter(|tag| !tag.is_empty())
            .find_map(|tag| self.create(&tag))
            .or_else(|| self.first_supported_language().and_then(|tag| self.create(&tag)));
        self.checkers.borrow_mut().insert(key, checker.clone());
        checker
    }

    fn create(&self, tag: &str) -> Option<ComPtr<ISystemSpellChecker>> {
        let tag = wide(tag);
        // SAFETY: a terminated string of this frame.
        let supported = unsafe { self.factory.is_supported(tag.as_ptr()) }.ok()?;
        if supported == 0 {
            return None;
        }
        // SAFETY: a terminated string of this frame.
        unsafe { self.factory.create_spell_checker(tag.as_ptr()) }.ok().flatten()
    }

    fn first_supported_language(&self) -> Option<String> {
        let languages = self.factory.get_supported_languages().ok().flatten()?;
        enum_strings(&languages).into_iter().next()
    }
}

impl ISpellChecker for Win32SpellChecker {
    fn check_spelling(&self, text: Rc<[u16]>, language: Option<&str>, completion: SpellCheckCompletion) {
        let Some(checker) = self.checker(language) else {
            completion(Vec::new());
            return;
        };
        let mut terminated = Vec::with_capacity(text.len() + 1);
        terminated.extend_from_slice(&text);
        // A NUL inside the text would end it early: checked up to there.
        terminated.push(0);
        let mut errors = Vec::new();
        // SAFETY: a terminated string of this frame.
        if let Ok(Some(enumeration)) = unsafe { checker.check(terminated.as_ptr()) } {
            while let Some(error) = next_error(&enumeration) {
                let (Ok(start), Ok(length)) = (error.get_start_index(), error.get_length()) else { continue };
                if error.get_corrective_action().is_ok_and(|action| action == CORRECTIVE_ACTION_DELETE) {
                    continue;
                }
                errors.push(SpellingError::new(start as usize, length as usize));
            }
        }
        completion(errors);
    }

    fn suggestions(&self, word: &str, language: Option<&str>) -> Vec<String> {
        let Some(checker) = self.checker(language) else { return Vec::new() };
        let word = wide(word);
        // SAFETY: a terminated string of this frame.
        match unsafe { checker.suggest(word.as_ptr()) } {
            Ok(Some(suggestions)) => enum_strings(&suggestions),
            _ => Vec::new(),
        }
    }

    fn ignore_word(&self, word: &str) {
        let word = wide(word);
        for checker in self.checkers.borrow().values().flatten() {
            // SAFETY: a terminated string of this frame.
            let _ = unsafe { checker.ignore(word.as_ptr()) };
        }
    }

    fn add_to_dictionary(&self, word: &str) {
        let word = wide(word);
        for checker in self.checkers.borrow().values().flatten() {
            // SAFETY: a terminated string of this frame.
            let _ = unsafe { checker.add(word.as_ptr()) };
        }
    }
}

/// The next error of an enumeration, `None` at its end (`S_FALSE`) or on a failure.
fn next_error(enumeration: &IEnumSpellingError) -> Option<ComPtr<ISpellingError>> {
    let mut error = std::ptr::null_mut();
    // SAFETY: the enumeration writes an interface pointer it hands over to the pointer of this
    // frame.
    let result = unsafe { enumeration.next(&mut error) };
    if result != 0 {
        return None;
    }
    // SAFETY: an interface pointer of the error with the reference the value takes over.
    unsafe { ComPtr::from_raw(error.cast()) }
}

/// The strings of an enumeration, each freed with the COM allocator once copied.
fn enum_strings(enumeration: &IEnumString) -> Vec<String> {
    let mut strings = Vec::new();
    loop {
        let mut string: *mut u16 = std::ptr::null_mut();
        let mut fetched = 0u32;
        // SAFETY: one string of the COM allocator is written to the pointer of this frame.
        let result = unsafe { enumeration.next(1, &mut string, &mut fetched) };
        if fetched != 1 || string.is_null() {
            break;
        }
        // SAFETY: a terminated string of the COM allocator, read up to its terminator and freed.
        unsafe {
            let mut length = 0;
            while *string.add(length) != 0 {
                length += 1;
            }
            strings.push(String::from_utf16_lossy(std::slice::from_raw_parts(string, length)));
            co_task_mem_free(string.cast());
        }
        if result != 0 {
            break;
        }
    }
    strings
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}
