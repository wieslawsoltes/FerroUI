//! Suggestion request data, shared ownership and native callback access.
use super::{KeyProximity, TextCulture, Utf16Text};
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

/// Explicit construction/copy values; this is not the identity-bearing request object.
#[derive(Clone)]
pub struct SuggestionRequestValues {
    pub word: Option<Utf16Text>,
    pub preceding_text: Option<Utf16Text>,
    pub max_count: i32,
    pub allow_auto_correct: bool,
    pub culture: Option<TextCulture>,
    pub key_proximity: Option<KeyProximity>,
}
impl Default for SuggestionRequestValues {
    fn default() -> Self {
        Self {
            word: Some(Utf16Text::default()),
            preceding_text: Some(Utf16Text::default()),
            max_count: 3,
            allow_auto_correct: true,
            culture: Some(TextCulture::invariant()),
            key_proximity: None,
        }
    }
}
/// A shared request handle. Clone shares the storage; values() produces a field snapshot.
#[derive(Clone)]
pub struct SuggestionRequest {
    values: Rc<RefCell<SuggestionRequestValues>>,
}
pub struct WeakSuggestionRequest {
    values: Weak<RefCell<SuggestionRequestValues>>,
}
impl WeakSuggestionRequest {
    pub fn upgrade(&self) -> Option<SuggestionRequest> {
        self.values
            .upgrade()
            .map(|values| SuggestionRequest { values })
    }
}
impl PartialEq for SuggestionRequest {
    fn eq(&self, other: &Self) -> bool {
        // Read fields separately: existing locale callbacks can reenter the request.
        self.ptr_eq(other)
            || self.word() == other.word()
                && self.preceding_text() == other.preceding_text()
                && self.max_count() == other.max_count()
                && self.allow_auto_correct() == other.allow_auto_correct()
                && {
                    let a = self.culture();
                    let b = other.culture();
                    a == b
                }
                && match (self.key_proximity(), other.key_proximity()) {
                    (None, None) => true,
                    (Some(a), Some(b)) => Rc::ptr_eq(&a, &b),
                    _ => false,
                }
    }
}
impl Default for SuggestionRequest {
    fn default() -> Self {
        Self::from_values(SuggestionRequestValues::default())
    }
}
impl SuggestionRequest {
    pub fn from_values(values: SuggestionRequestValues) -> Self {
        Self {
            values: Rc::new(RefCell::new(values)),
        }
    }
    pub fn values(&self) -> SuggestionRequestValues {
        self.values.borrow().clone()
    }
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.values, &other.values)
    }
    pub fn downgrade(&self) -> WeakSuggestionRequest {
        WeakSuggestionRequest {
            values: Rc::downgrade(&self.values),
        }
    }
    pub fn word(&self) -> Option<Utf16Text> {
        self.values.borrow().word.clone()
    }
    pub fn set_word(&self, value: Option<Utf16Text>) {
        self.values.borrow_mut().word = value;
    }
    pub fn preceding_text(&self) -> Option<Utf16Text> {
        self.values.borrow().preceding_text.clone()
    }
    pub fn set_preceding_text(&self, value: Option<Utf16Text>) {
        self.values.borrow_mut().preceding_text = value;
    }
    pub fn max_count(&self) -> i32 {
        self.values.borrow().max_count
    }
    pub fn set_max_count(&self, value: i32) {
        self.values.borrow_mut().max_count = value;
    }
    pub fn allow_auto_correct(&self) -> bool {
        self.values.borrow().allow_auto_correct
    }
    pub fn set_allow_auto_correct(&self, value: bool) {
        self.values.borrow_mut().allow_auto_correct = value;
    }
    pub fn culture(&self) -> Option<TextCulture> {
        self.values.borrow().culture
    }
    pub fn set_culture(&self, value: Option<TextCulture>) {
        self.values.borrow_mut().culture = value;
    }
    pub fn key_proximity(&self) -> Option<KeyProximity> {
        self.values.borrow().key_proximity.clone()
    }
    pub fn set_key_proximity(&self, value: Option<KeyProximity>) {
        let old = std::mem::replace(&mut self.values.borrow_mut().key_proximity, value);
        drop(old);
    }
}
