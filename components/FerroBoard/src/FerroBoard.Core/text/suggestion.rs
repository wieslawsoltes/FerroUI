//! Candidate values with explicit shared ownership. Clone copies values; Rc shares an instance.
use super::{SuggestionKind, Utf16Text};
#[derive(Clone, Debug)]
pub struct SuggestionValues {
    pub text: Option<Utf16Text>,
    pub kind: SuggestionKind,
    pub confidence: f64,
    pub is_auto_commit: bool,
    pub description: Option<Utf16Text>,
    pub is_removable: bool,
}
impl SuggestionValues {
    pub fn new(text: Utf16Text, kind: SuggestionKind) -> Self {
        Self::new_nullable(Some(text), kind, 0.0, false)
    }
    pub fn new_nullable(
        text: Option<Utf16Text>,
        kind: SuggestionKind,
        confidence: f64,
        is_auto_commit: bool,
    ) -> Self {
        Self {
            text,
            kind,
            confidence,
            is_auto_commit,
            description: None,
            is_removable: false,
        }
    }
}
impl PartialEq for SuggestionValues {
    fn eq(&self, other: &Self) -> bool {
        self.text == other.text
            && self.kind == other.kind
            && (self.confidence == other.confidence
                || self.confidence.is_nan() && other.confidence.is_nan())
            && self.is_auto_commit == other.is_auto_commit
            && self.description == other.description
            && self.is_removable == other.is_removable
    }
}
impl Eq for SuggestionValues {}
impl std::hash::Hash for SuggestionValues {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.text.hash(state);
        self.kind.hash(state);
        let bits = if self.confidence.is_nan() {
            f64::NAN.to_bits()
        } else if self.confidence == 0.0 {
            0
        } else {
            self.confidence.to_bits()
        };
        bits.hash(state);
        self.is_auto_commit.hash(state);
        self.description.hash(state);
        self.is_removable.hash(state)
    }
}
#[derive(Debug)]
pub struct Suggestion {
    values: parking_lot::RwLock<SuggestionValues>,
}

impl Clone for Suggestion {
    fn clone(&self) -> Self {
        Self::from_values(self.values())
    }
}
impl PartialEq for Suggestion {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other) || self.values() == other.values()
    }
}
impl Eq for Suggestion {}
impl std::hash::Hash for Suggestion {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.values().hash(state)
    }
}
impl Suggestion {
    pub fn new(text: Utf16Text, kind: SuggestionKind) -> Self {
        Self::from_values(SuggestionValues::new(text, kind))
    }
    pub fn new_nullable(
        text: Option<Utf16Text>,
        kind: SuggestionKind,
        confidence: f64,
        is_auto_commit: bool,
    ) -> Self {
        Self::from_values(SuggestionValues::new_nullable(
            text,
            kind,
            confidence,
            is_auto_commit,
        ))
    }
    pub fn from_values(values: SuggestionValues) -> Self {
        Self {
            values: parking_lot::RwLock::new(values),
        }
    }
    pub fn values(&self) -> SuggestionValues {
        self.values.read().clone()
    }
    pub fn text(&self) -> Option<Utf16Text> {
        self.values.read().text.clone()
    }
    pub fn set_text(&self, value: Option<Utf16Text>) {
        self.values.write().text = value;
    }
    pub fn kind(&self) -> SuggestionKind {
        self.values.read().kind
    }
    pub fn set_kind(&self, value: SuggestionKind) {
        self.values.write().kind = value;
    }
    pub fn confidence(&self) -> f64 {
        self.values.read().confidence
    }
    pub fn set_confidence(&self, value: f64) {
        self.values.write().confidence = value;
    }
    pub fn is_auto_commit(&self) -> bool {
        self.values.read().is_auto_commit
    }
    pub fn set_is_auto_commit(&self, value: bool) {
        self.values.write().is_auto_commit = value;
    }
    pub fn description(&self) -> Option<Utf16Text> {
        self.values.read().description.clone()
    }
    pub fn set_description(&self, value: Option<Utf16Text>) {
        self.values.write().description = value;
    }
    pub fn is_removable(&self) -> bool {
        self.values.read().is_removable
    }
    pub fn set_is_removable(&self, value: bool) {
        self.values.write().is_removable = value;
    }
}
