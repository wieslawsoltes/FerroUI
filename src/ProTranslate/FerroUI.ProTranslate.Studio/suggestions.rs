use crate::{
    AiProvider, AiTranslationRequest, AiTranslationRequestEntry, AiTranslationResult, EntryId,
    EntryStatus, StudioDocument, StudioError, TranslationReviewState,
};
use ferroui_protranslate_formats::TranslationCatalogEntry;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug)]
struct EntryBaseline {
    session_id: String,
    generation: u64,
    target_culture: String,
    entry: TranslationCatalogEntry,
    status: EntryStatus,
}

/// Captured before awaiting transport. Wire keys are stable IDs, so duplicate
/// catalog keys and punctuation are unambiguous in provider responses.
#[derive(Clone, Debug)]
pub struct TranslationPlan {
    request: AiTranslationRequest,
    baselines: BTreeMap<String, EntryBaseline>,
}
impl TranslationPlan {
    pub fn request(&self) -> &AiTranslationRequest {
        &self.request
    }
    pub fn entry_count(&self) -> usize {
        self.baselines.len()
    }
    /// A result may cover one batch of this plan. Provider response validation is
    /// repeated here for host-created results before any document mutation.
    pub fn suggestions(
        &self,
        result: &AiTranslationResult,
    ) -> Result<Vec<TranslationSuggestion>, StudioError> {
        let mut seen = BTreeSet::new();
        let mut suggestions = Vec::new();
        for entry in &result.entries {
            let baseline = self.baselines.get(&entry.key).ok_or_else(|| {
                StudioError::Provider("Response contains an unknown entry identity".into())
            })?;
            if !seen.insert(&entry.key)
                || entry.translated_text.trim().is_empty()
                || !entry.confidence.is_finite()
                || !(0.0..=1.0).contains(&entry.confidence)
            {
                return Err(StudioError::Provider(
                    "Response contains a duplicate, empty or invalid suggestion".into(),
                ));
            }
            suggestions.push(TranslationSuggestion {
                baseline: baseline.clone(),
                translated_text: entry.translated_text.clone(),
                provider: result.provider,
                model_used: result.model_used.clone(),
                confidence: entry.confidence,
                tokens_used: result.tokens_used,
                duration_ms: result.duration_ms,
            });
        }
        Ok(suggestions)
    }
}

#[derive(Clone, Debug)]
pub struct TranslationSuggestion {
    baseline: EntryBaseline,
    pub translated_text: String,
    pub provider: AiProvider,
    pub model_used: String,
    pub confidence: f64,
    /// Request/batch totals, not falsely attributed per-entry usage.
    pub tokens_used: u64,
    pub duration_ms: u64,
}
impl TranslationSuggestion {
    pub fn entry_id(&self) -> EntryId {
        self.baseline.status.id
    }
    pub fn key(&self) -> &str {
        &self.baseline.entry.key
    }
    /// Rejection consumes only the suggestion; it never modifies a document.
    pub fn reject(self) {}
    pub fn is_current(&self, document: &StudioDocument) -> bool {
        document.session().session_id == self.baseline.session_id
            && document.generation() == self.baseline.generation
            && document.session().target_culture == self.baseline.target_culture
            && document
                .entry(self.entry_id())
                .is_some_and(|(entry, status)| {
                    entry == &self.baseline.entry && status == &self.baseline.status
                })
    }
}

impl StudioDocument {
    pub fn prepare_translation(
        &self,
        ids: &[EntryId],
        context: impl Into<String>,
    ) -> Result<TranslationPlan, StudioError> {
        let mut entries = Vec::new();
        let mut baselines = BTreeMap::new();
        let mut seen = BTreeSet::new();
        for id in ids {
            if !seen.insert(*id) {
                return Err(StudioError::InvalidInput(
                    "A translation plan cannot repeat an entry".into(),
                ));
            }
            let (entry, status) = self.entry(*id).ok_or(StudioError::EntryNotFound(id.0))?;
            if entry.culture.as_deref().is_some_and(|culture| {
                !culture.eq_ignore_ascii_case(&self.session().target_culture)
            }) {
                return Err(StudioError::InvalidInput(
                    "Translation plan entries must belong to the selected target culture".into(),
                ));
            }
            let wire_key = id.0.to_string();
            entries.push(AiTranslationRequestEntry {
                key: wire_key.clone(),
                source_text: entry.source.as_ref().unwrap_or(&entry.key).clone(),
            });
            baselines.insert(
                wire_key,
                EntryBaseline {
                    session_id: self.session().session_id.clone(),
                    generation: self.generation(),
                    target_culture: self.session().target_culture.clone(),
                    entry: entry.clone(),
                    status: status.clone(),
                },
            );
        }
        let request = AiTranslationRequest {
            source_language: self
                .session()
                .catalog
                .source_culture
                .as_deref()
                .unwrap_or("en-US")
                .into(),
            target_language: self.session().target_culture.clone(),
            entries,
            context: context.into(),
        };
        request.validate()?;
        Ok(TranslationPlan { request, baselines })
    }
    pub fn prepare_missing_translations(
        &self,
        context: impl Into<String>,
    ) -> Result<TranslationPlan, StudioError> {
        let ids = self
            .session()
            .catalog
            .entries
            .iter()
            .zip(&self.session().entry_status)
            .filter(|(entry, status)| {
                entry.culture.as_deref().is_none_or(|culture| {
                    culture.eq_ignore_ascii_case(&self.session().target_culture)
                }) && (status.review == TranslationReviewState::Missing
                    || entry.value.trim().is_empty())
            })
            .map(|(_, status)| status.id)
            .collect::<Vec<_>>();
        self.prepare_translation(&ids, context)
    }
    pub fn accept_suggestion(
        &mut self,
        suggestion: &TranslationSuggestion,
    ) -> Result<bool, StudioError> {
        if !suggestion.is_current(self) {
            return Err(StudioError::StaleSuggestion);
        }
        if suggestion.translated_text.trim().is_empty() {
            return Err(StudioError::InvalidInput(
                "Cannot accept an empty translation suggestion".into(),
            ));
        }
        let source = suggestion
            .baseline
            .entry
            .source
            .as_deref()
            .unwrap_or(&suggestion.baseline.entry.key);
        let diagnostics = crate::validate_placeholders(
            source,
            &suggestion.translated_text,
            self.session().source_format,
        );
        if !diagnostics.is_empty() {
            return Err(StudioError::InvalidInput(
                diagnostics
                    .iter()
                    .map(|diagnostic| diagnostic.message.as_str())
                    .collect::<Vec<_>>()
                    .join("; "),
            ));
        }
        self.transaction(|document| {
            let value_changed =
                document.set_target(suggestion.entry_id(), suggestion.translated_text.clone())?;
            let review_changed =
                document.set_review(suggestion.entry_id(), TranslationReviewState::Review)?;
            Ok(value_changed || review_changed)
        })
    }
    /// The whole accepted batch is one undo operation. Any stale suggestion
    /// rejects the batch before writing, so a later edit can never be lost.
    pub fn accept_suggestions(
        &mut self,
        suggestions: &[TranslationSuggestion],
    ) -> Result<usize, StudioError> {
        let mut seen = BTreeSet::new();
        for suggestion in suggestions {
            if !seen.insert(suggestion.entry_id()) {
                return Err(StudioError::InvalidInput(
                    "Duplicate suggestion identity".into(),
                ));
            }
            if !suggestion.is_current(self) {
                return Err(StudioError::StaleSuggestion);
            }
        }
        self.transaction(|document| {
            let mut count = 0;
            for suggestion in suggestions {
                if document.accept_suggestion(suggestion)? {
                    count += 1;
                }
            }
            Ok(count)
        })
    }
}
