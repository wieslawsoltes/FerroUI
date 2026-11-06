//! Suggestion contracts and DictionarySuggestionProvider from Text/Suggestions.cs.
//! UserDictionary is split into user_dictionary.rs; source candidate/notification order is retained.
use super::{
    DictionaryError, FuzzyMatch, NormalizationError, PredictionList, PredictionListBuffer,
    Suggestion, SuggestionList, SuggestionRequest, SuggestionValues, TextCaseError, TextUtils,
    UserDictionary, Utf16Text, WordDictionary, ordinal_ignore_case_equal,
    text_properties::properties,
};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
    sync::Arc,
};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SuggestionKind(pub i32);
#[allow(non_upper_case_globals)]
impl SuggestionKind {
    pub const Typed: Self = Self(0);
    pub const Completion: Self = Self(1);
    pub const Correction: Self = Self(2);
    pub const Prediction: Self = Self(3);
    pub const Emoji: Self = Self(4);
    pub const Glide: Self = Self(5);
    pub const Clipboard: Self = Self(6);
}
pub type KeyProximity = Rc<dyn Fn(u16, u16) -> f64>;
#[derive(Debug)]
pub enum SuggestionError {
    NullReference,
    ArgumentNull(&'static str),
    Collection(crate::layouts::ResourceTextListError),
    Case(TextCaseError),
    Dictionary(DictionaryError),
    Normalization(NormalizationError),
    NullDefaultPredictions,
    PredictionIndex(usize),
    SuggestionIndex(usize),
    Custom(String),
}
impl std::fmt::Display for SuggestionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Custom(message) => f.write_str(message),
            Self::PredictionIndex(index) => write!(f, "prediction index {index} is out of bounds"),
            Self::SuggestionIndex(index) => write!(f, "suggestion index {index} is out of bounds"),
            error => write!(f, "{error:?}"),
        }
    }
}
impl std::error::Error for SuggestionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Collection(error) => Some(error),
            Self::Case(error) => Some(error),
            Self::Dictionary(error) => Some(error),
            Self::Normalization(error) => Some(error),
            _ => None,
        }
    }
}
pub type SuggestionResult<T> = Result<T, SuggestionError>;
impl From<TextCaseError> for SuggestionError {
    fn from(value: TextCaseError) -> Self {
        Self::Case(value)
    }
}
impl From<DictionaryError> for SuggestionError {
    fn from(value: DictionaryError) -> Self {
        Self::Dictionary(value)
    }
}
impl From<NormalizationError> for SuggestionError {
    fn from(value: NormalizationError) -> Self {
        Self::Normalization(value)
    }
}
pub trait ISuggestionProvider: std::any::Any {
    fn suggest(
        &self,
        request: Option<&SuggestionRequest>,
    ) -> SuggestionResult<Option<SuggestionList>>;
    fn learn(
        &self,
        word: Option<&Utf16Text>,
        previous_word: Option<&Utf16Text>,
    ) -> SuggestionResult<()>;
    fn forget(&self, word: Option<&Utf16Text>) -> SuggestionResult<()>;
}
pub struct DictionarySuggestionProvider {
    dictionary: Option<Arc<WordDictionary>>,
    user_dictionary: UserDictionary,
    max_correction_distance: Cell<f64>,
    auto_correct_threshold: Cell<f64>,
    rare_word_frequency_gap: Cell<i32>,
    default_predictions: RefCell<Option<PredictionList>>,
}
impl DictionarySuggestionProvider {
    pub fn new(dictionary: Arc<WordDictionary>, user_dictionary: Option<UserDictionary>) -> Self {
        Self::new_nullable(Some(dictionary), user_dictionary)
    }
    pub fn new_nullable(
        dictionary: Option<Arc<WordDictionary>>,
        user_dictionary: Option<UserDictionary>,
    ) -> Self {
        Self {
            dictionary,
            user_dictionary: user_dictionary.unwrap_or_default(),
            max_correction_distance: Cell::new(2.0),
            auto_correct_threshold: Cell::new(0.55),
            rare_word_frequency_gap: Cell::new(70),
            default_predictions: RefCell::new(Some(Rc::new(PredictionListBuffer::from_nullable(
                vec![Some("I".into()), Some("The".into()), Some("Thanks".into())],
            )))),
        }
    }
    pub fn dictionary(&self) -> Option<Arc<WordDictionary>> {
        self.dictionary.clone()
    }
    pub fn user_dictionary(&self) -> UserDictionary {
        self.user_dictionary.clone()
    }
    pub fn max_correction_distance(&self) -> f64 {
        self.max_correction_distance.get()
    }
    pub fn set_max_correction_distance(&self, value: f64) {
        self.max_correction_distance.set(value);
    }
    pub fn auto_correct_threshold(&self) -> f64 {
        self.auto_correct_threshold.get()
    }
    pub fn set_auto_correct_threshold(&self, value: f64) {
        self.auto_correct_threshold.set(value);
    }
    pub fn rare_word_frequency_gap(&self) -> i32 {
        self.rare_word_frequency_gap.get()
    }
    pub fn set_rare_word_frequency_gap(&self, value: i32) {
        self.rare_word_frequency_gap.set(value);
    }
    pub fn default_predictions(&self) -> Option<PredictionList> {
        self.default_predictions.borrow().clone()
    }
    pub fn set_default_predictions(&self, value: Option<PredictionList>) {
        let old = self.default_predictions.replace(value);
        drop(old);
    }
    pub fn suggest(&self, request: &SuggestionRequest) -> SuggestionResult<SuggestionList> {
        self.suggest_nullable(Some(request))
    }
    pub fn suggest_nullable(
        &self,
        request: Option<&SuggestionRequest>,
    ) -> SuggestionResult<SuggestionList> {
        self.compute_suggestions(request).map(SuggestionList::new)
    }
    fn compute_suggestions(
        &self,
        request: Option<&SuggestionRequest>,
    ) -> SuggestionResult<Vec<Rc<Suggestion>>> {
        let request = request.ok_or(SuggestionError::NullReference)?;
        let word_value = request.word();
        let culture = request.culture();
        let max = 1.max(request.max_count());
        let word = word_value.as_ref().ok_or(SuggestionError::NullReference)?;
        if word.is_empty() {
            return self.predict(request, max);
        }
        // Keep first encounter order for stable confidence ties and slot-preserving upgrades.
        let mut candidate_indices = HashMap::<Utf16Text, usize>::new();
        let mut candidates = Vec::<Rc<Suggestion>>::new();
        let dictionary = self
            .dictionary
            .as_ref()
            .ok_or(SuggestionError::NullReference)?;
        let typed_frequency = dictionary.get_frequency(word)?;
        let typed_learned =
            self.user_dictionary.get_count(word) >= self.user_dictionary.learn_threshold();
        let mut typed_known = typed_frequency > 0 || typed_learned;
        let max_freq = 1.0f64.max(dictionary.max_frequency() as f64);
        let mut add = |text: &Utf16Text, kind, confidence, removable| -> SuggestionResult<()> {
            if self.user_dictionary.is_blocked(text) {
                return Ok(());
            }
            let cased = TextUtils::match_case_nullable(text, word, culture)?;
            if &cased == word && kind != SuggestionKind::Typed {
                return Ok(());
            }
            let existing = candidate_indices.get(&cased).copied();
            if existing.is_none_or(|index| candidates[index].confidence() < confidence) {
                let suggestion = Rc::new(Suggestion::from_values(SuggestionValues {
                    text: Some(cased.clone()),
                    kind,
                    confidence,
                    is_auto_commit: false,
                    description: None,
                    is_removable: removable,
                }));
                if let Some(index) = existing {
                    candidates[index] = suggestion;
                } else {
                    candidate_indices.insert(cased, candidates.len());
                    candidates.push(suggestion);
                }
            }
            Ok(())
        };
        for (w, f) in dictionary.complete(word, max.wrapping_add(3))? {
            if w.len() <= word.len()
                && WordDictionary::normalize(&w)? == WordDictionary::normalize(word)?
            {
                if !ordinal_ignore_case_equal(&w, word) {
                    add(
                        &w,
                        SuggestionKind::Correction,
                        0.8 + 0.2 * f as f64 / max_freq,
                        false,
                    )?;
                }
                continue;
            }
            let length_penalty =
                1.0 / (1.0 + 0.15 * (w.len() as i32).wrapping_sub(word.len() as i32) as f64);
            add(
                &w,
                SuggestionKind::Completion,
                0.5 * (f as f64 / max_freq) * length_penalty + 0.1,
                false,
            )?;
        }
        for (w, count) in self.user_dictionary.complete(word)? {
            add(
                &w,
                SuggestionKind::Completion,
                0.9f64.min(0.4 + 0.05 * count as f64),
                true,
            )?;
        }
        let distance = if word.len() <= 2 {
            0.0
        } else if word.len() <= 4 {
            1.0
        } else {
            self.max_correction_distance()
        };
        let mut best_correction: Option<FuzzyMatch> = None;
        let mut best_confidence = 0.0;
        if distance > 0.0 {
            for m in
                dictionary.find_similar(word, distance, 12, request.key_proximity().as_deref())?
            {
                if ordinal_ignore_case_equal(&m.word, word) {
                    continue;
                }
                let freq_score = m.frequency as f64 / max_freq;
                let mut confidence = (0.85 * freq_score * (-1.2 * m.distance).exp()
                    + if m.distance <= 1.0 { 0.2 } else { 0.0 })
                .clamp(0.0, 1.0);
                if typed_known
                    && m.frequency.wrapping_sub(typed_frequency) < self.rare_word_frequency_gap()
                {
                    confidence *= 0.3;
                }
                add(&m.word, SuggestionKind::Correction, confidence, false)?;
                if confidence > best_confidence {
                    best_confidence = confidence;
                    best_correction = Some(m);
                }
            }
        }
        let mut ordered = candidates.clone();
        ordered.sort_by(|a, b| double_compare(b.confidence(), a.confidence()));
        ordered.truncate(max as usize);
        let correctable = !typed_learned
            && (typed_frequency == 0
                || best_correction.as_ref().is_some_and(|bc| {
                    bc.distance <= 1.0
                        && bc.frequency.wrapping_sub(typed_frequency)
                            >= self.rare_word_frequency_gap()
                }));
        if request.allow_auto_correct()
            && correctable
            && let Some(best) = best_correction
            && best_confidence >= self.auto_correct_threshold() * 0.5
        {
            let cased = TextUtils::match_case_nullable(&best.word, word, culture)?;
            let mut index = ordered.iter().position(|s| s.text() == Some(cased.clone()));
            if index.is_none()
                && let Some(&candidate_index) = candidate_indices.get(&cased)
            {
                ordered.insert(0, candidates[candidate_index].clone());
                index = Some(0);
            }
            if let Some(index) = index {
                let candidate = ordered.remove(index);
                ordered.insert(
                    0,
                    Rc::new(Suggestion::from_values(SuggestionValues {
                        is_auto_commit: true,
                        ..candidate.values()
                    })),
                );
                typed_known = false;
            }
        }
        if typed_known
            && !ordered.iter().any(|s| s.is_auto_commit())
            && !ordered.iter().any(|s| s.text().as_ref() == Some(word))
        {
            ordered.insert(
                0,
                Rc::new(Suggestion::from_values(SuggestionValues {
                    confidence: 1.0,
                    ..SuggestionValues::new(word.clone(), SuggestionKind::Typed)
                })),
            );
            if ordered.len() > max as usize {
                ordered.pop();
            }
        }
        if ordered.first().is_some_and(|s| s.is_auto_commit())
            && !ordered.iter().any(|s| s.text().as_ref() == Some(word))
        {
            ordered.insert(
                0,
                Rc::new(Suggestion::from_values(SuggestionValues {
                    confidence: 1.0,
                    ..SuggestionValues::new(word.clone(), SuggestionKind::Typed)
                })),
            );
            if ordered.len() > max as usize {
                ordered.pop();
            }
        }
        Ok(ordered)
    }
    fn predict(
        &self,
        request: &SuggestionRequest,
        max: i32,
    ) -> SuggestionResult<Vec<Rc<Suggestion>>> {
        let preceding = request
            .preceding_text()
            .ok_or(SuggestionError::NullReference)?;
        let mut end = preceding.len();
        while end > 0 && TextUtils::is_white_space(preceding.units()[end - 1]) {
            end -= 1;
        }
        let trimmed = Utf16Text::from_units(preceding.units()[..end].to_vec());
        let previous = TextUtils::get_current_word(&trimmed);
        let mut result = Vec::new();
        if !previous.is_empty() {
            for (word, count) in self.user_dictionary.predict(&previous, max) {
                result.push(Rc::new(Suggestion::from_values(SuggestionValues {
                    confidence: 1.0f64.min(0.3 + 0.1 * count as f64),
                    is_removable: true,
                    ..SuggestionValues::new(word, SuggestionKind::Prediction)
                })));
            }
        } else if request
            .preceding_text()
            .ok_or(SuggestionError::NullReference)?
            .units()
            .iter()
            .all(|&c| TextUtils::is_white_space(c))
        {
            let defaults = self
                .default_predictions()
                .ok_or(SuggestionError::NullDefaultPredictions)?;
            super::prediction_list::visit_predictions(&defaults, max, |word| {
                result.push(Rc::new(Suggestion::from_values(SuggestionValues {
                    confidence: 0.1,
                    ..SuggestionValues::new_nullable(word, SuggestionKind::Prediction, 0.0, false)
                })));
                Ok(())
            })?;
        }
        Ok(result)
    }
    pub fn learn(&self, word: &Utf16Text, previous_word: Option<&Utf16Text>) {
        if word.len() < 2
            || !word
                .units()
                .iter()
                .any(|&c| properties(c as u32).0 & 8 != 0)
        {
            return;
        }
        self.user_dictionary.learn(word, previous_word);
    }
    pub fn forget(&self, word: &Utf16Text) {
        self.user_dictionary.block(word);
    }
    pub fn learn_nullable(
        &self,
        word: Option<&Utf16Text>,
        previous_word: Option<&Utf16Text>,
    ) -> SuggestionResult<()> {
        self.learn(word.ok_or(SuggestionError::NullReference)?, previous_word);
        Ok(())
    }
    pub fn forget_nullable(&self, word: Option<&Utf16Text>) -> SuggestionResult<()> {
        self.forget(word.ok_or(SuggestionError::ArgumentNull("key"))?);
        Ok(())
    }
}
impl PartialEq for DictionarySuggestionProvider {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl ISuggestionProvider for DictionarySuggestionProvider {
    fn suggest(
        &self,
        request: Option<&SuggestionRequest>,
    ) -> SuggestionResult<Option<SuggestionList>> {
        self.suggest_nullable(request).map(Some)
    }
    fn learn(
        &self,
        word: Option<&Utf16Text>,
        previous_word: Option<&Utf16Text>,
    ) -> SuggestionResult<()> {
        self.learn_nullable(word, previous_word)
    }
    fn forget(&self, word: Option<&Utf16Text>) -> SuggestionResult<()> {
        self.forget_nullable(word)
    }
}
fn double_compare(a: f64, b: f64) -> std::cmp::Ordering {
    a.partial_cmp(&b)
        .unwrap_or_else(|| a.is_nan().cmp(&b.is_nan()).reverse())
}

impl From<crate::layouts::ResourceTextListError> for SuggestionError {
    fn from(value: crate::layouts::ResourceTextListError) -> Self {
        Self::Collection(value)
    }
}
