use crate::gateway::normalize_culture;
use crate::{CatalogGateway, ImportedCatalog, StudioError};
use ferroui_protranslate_formats::{
    TranslationCatalog, TranslationCatalogEntry, TranslationExportResult, TranslationFileFormat,
    TranslationFormatDiagnostic, TranslationFormatOptions,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub const SESSION_SCHEMA_VERSION: u32 = 1;
pub const DEFAULT_HISTORY_LIMIT: usize = 100;

/// Stable identity independent of a record's key, culture, position or duplicate status.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EntryId(pub u64);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum TranslationReviewState {
    Missing,
    Review,
    Approved,
}

impl TranslationReviewState {
    fn from_entry(entry: &TranslationCatalogEntry) -> Self {
        if entry.value.trim().is_empty() {
            return Self::Missing;
        }
        match entry
            .state
            .as_deref()
            .unwrap_or("")
            .to_ascii_lowercase()
            .as_str()
        {
            "approved" | "final" | "translated" | "signed-off" => Self::Approved,
            "review" | "fuzzy" | "needs-review-translation" | "needs-review-adaptation" => {
                Self::Review
            }
            _ if entry
                .comment
                .as_ref()
                .is_some_and(|comment| comment.to_ascii_lowercase().contains("review")) =>
            {
                Self::Review
            }
            _ => Self::Approved,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryStatus {
    pub id: EntryId,
    pub review: TranslationReviewState,
    /// Changes monotonically even when undo returns to an earlier text value.
    pub revision: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CultureChoice {
    pub culture_name: String,
    pub display_name: String,
}

/// Versioned, complete session payload. Runtime history and credentials are separate.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationSession {
    pub schema_version: u32,
    pub session_id: String,
    pub created_at_millis: i64,
    pub last_modified_at_millis: i64,
    pub file_name: String,
    pub source_format: TranslationFileFormat,
    pub target_culture: String,
    pub catalog: TranslationCatalog,
    /// One status per catalog entry, in the same order; validated when restoring.
    pub entry_status: Vec<EntryStatus>,
    pub import_diagnostics: Vec<TranslationFormatDiagnostic>,
    pub target_cultures: Vec<CultureChoice>,
    pub revision: u64,
    pub next_entry_id: u64,
}

impl TranslationSession {
    pub fn validate(&self) -> Result<(), StudioError> {
        if self.schema_version != SESSION_SCHEMA_VERSION {
            return Err(StudioError::InvalidInput(format!(
                "Unsupported session schema {}",
                self.schema_version
            )));
        }
        validate_session_id(&self.session_id)?;
        normalize_culture(&self.target_culture)?;
        if let Some(source) = &self.catalog.source_culture {
            normalize_culture(source)?;
        }
        if self.catalog.entries.len() != self.entry_status.len() {
            return Err(StudioError::InvalidInput(
                "Catalog and entry identities have different lengths".into(),
            ));
        }
        let mut ids = BTreeSet::new();
        for status in &self.entry_status {
            if status.id.0 == 0
                || status.id.0 >= self.next_entry_id
                || status.revision > self.revision
                || !ids.insert(status.id)
            {
                return Err(StudioError::InvalidInput(
                    "Invalid or duplicate entry identity/revision".into(),
                ));
            }
        }
        let mut cultures = BTreeSet::new();
        for culture in &self.target_cultures {
            if !cultures.insert(normalize_culture(&culture.culture_name)?) {
                return Err(StudioError::InvalidInput("Duplicate target culture".into()));
            }
        }
        if self.next_entry_id == 0 {
            return Err(StudioError::InvalidInput(
                "Entry identity counter overflow".into(),
            ));
        }
        if self
            .catalog
            .validate()
            .iter()
            .any(|d| d.severity == ferroui_protranslate_formats::Severity::Error)
        {
            return Err(StudioError::InvalidInput(
                "Catalog contains invalid entries".into(),
            ));
        }
        Ok(())
    }

    pub fn to_json(&self) -> Result<String, StudioError> {
        self.validate()?;
        Ok(serde_json::to_string_pretty(self)?)
    }
    pub fn from_json(json: &str) -> Result<Self, StudioError> {
        let session: Self = serde_json::from_str(json)?;
        session.validate()?;
        Ok(session)
    }
}

pub(crate) fn validate_session_id(value: &str) -> Result<(), StudioError> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(StudioError::InvalidInput(
            "Session identity must contain 1–128 ASCII letters, numbers, '-' or '_'".into(),
        ));
    }
    Ok(())
}

#[derive(Clone, Debug, Default)]
pub struct EntryFilter {
    pub search: String,
    pub state: Option<TranslationReviewState>,
    pub culture: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ReviewMetrics {
    pub total: usize,
    pub missing: usize,
    pub review: usize,
    pub approved: usize,
}
impl ReviewMetrics {
    pub fn completeness(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            self.approved as f64 / self.total as f64
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TranslationCoverage {
    pub culture_name: String,
    pub display_name: String,
    pub metrics: ReviewMetrics,
}

#[derive(Clone)]
struct HistoryEntry {
    before: TranslationSession,
    after: TranslationSession,
}

/// An owned document; hosts decide how to expose notifications and selection.
pub struct StudioDocument {
    session: TranslationSession,
    saved_revision: Option<u64>,
    undo: VecDeque<HistoryEntry>,
    redo: Vec<HistoryEntry>,
    history_limit: usize,
    /// Never rolled back by undo, preventing identity reuse by pending suggestions.
    next_revision: u64,
    next_entry_id: u64,
    /// Incremented on every restore, preventing suggestions crossing session reloads.
    generation: u64,
}

impl StudioDocument {
    pub fn new(
        session_id: impl Into<String>,
        imported: ImportedCatalog,
        now_millis: i64,
    ) -> Result<Self, StudioError> {
        let statuses: Vec<_> = imported
            .catalog
            .entries
            .iter()
            .enumerate()
            .map(|(index, entry)| EntryStatus {
                id: EntryId(index as u64 + 1),
                review: TranslationReviewState::from_entry(entry),
                revision: 0,
            })
            .collect();
        let next_entry_id = statuses.len() as u64 + 1;
        let mut cultures = imported.catalog.cultures();
        if !cultures
            .iter()
            .any(|culture| culture.eq_ignore_ascii_case(&imported.target_culture))
        {
            cultures.push(imported.target_culture.clone());
        }
        let session = TranslationSession {
            schema_version: SESSION_SCHEMA_VERSION,
            session_id: session_id.into(),
            created_at_millis: now_millis,
            last_modified_at_millis: now_millis,
            file_name: imported.file_name,
            source_format: imported.format,
            target_culture: imported.target_culture,
            catalog: imported.catalog,
            entry_status: statuses,
            import_diagnostics: imported.diagnostics,
            target_cultures: cultures
                .into_iter()
                .map(|name| CultureChoice {
                    culture_name: name.clone(),
                    display_name: name,
                })
                .collect(),
            revision: 0,
            next_entry_id,
        };
        session.validate()?;
        Ok(Self {
            session,
            saved_revision: None,
            undo: VecDeque::new(),
            redo: Vec::new(),
            history_limit: DEFAULT_HISTORY_LIMIT,
            next_revision: 1,
            next_entry_id,
            generation: 0,
        })
    }

    pub fn from_session(session: TranslationSession) -> Result<Self, StudioError> {
        session.validate()?;
        let next_revision = session
            .revision
            .checked_add(1)
            .ok_or_else(|| StudioError::InvalidInput("Session revision exhausted".into()))?;
        Ok(Self {
            saved_revision: Some(session.revision),
            next_entry_id: session.next_entry_id,
            session,
            undo: VecDeque::new(),
            redo: Vec::new(),
            history_limit: DEFAULT_HISTORY_LIMIT,
            next_revision,
            generation: 0,
        })
    }

    pub fn restore(
        &mut self,
        session: TranslationSession,
        recovered: bool,
    ) -> Result<(), StudioError> {
        let mut restored = Self::from_session(session)?;
        restored.generation = self
            .generation
            .checked_add(1)
            .ok_or_else(|| StudioError::InvalidInput("Document generation exhausted".into()))?;
        restored.history_limit = self.history_limit;
        if recovered {
            restored.saved_revision = None;
        }
        *self = restored;
        Ok(())
    }

    pub fn session(&self) -> &TranslationSession {
        &self.session
    }
    pub fn snapshot(&self, now_millis: i64) -> TranslationSession {
        let mut snapshot = self.session.clone();
        snapshot.last_modified_at_millis = now_millis;
        snapshot
    }
    pub fn catalog(&self) -> &TranslationCatalog {
        &self.session.catalog
    }
    pub fn is_dirty(&self) -> bool {
        self.saved_revision != Some(self.session.revision)
    }
    /// Call only after the host successfully persisted this exact snapshot.
    pub fn mark_saved(&mut self, snapshot: &TranslationSession) -> bool {
        let mut current = self.session.clone();
        current.last_modified_at_millis = snapshot.last_modified_at_millis;
        if current != *snapshot {
            return false;
        }
        self.saved_revision = Some(snapshot.revision);
        self.session.last_modified_at_millis = snapshot.last_modified_at_millis;
        true
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }
    pub fn undo_count(&self) -> usize {
        self.undo.len()
    }
    pub fn redo_count(&self) -> usize {
        self.redo.len()
    }
    pub fn set_history_limit(&mut self, limit: usize) {
        self.history_limit = limit;
        while self.undo.len() > limit {
            self.undo.pop_front();
        }
        if self.redo.len() > limit {
            self.redo.drain(..self.redo.len() - limit);
        }
    }
    pub fn entry(&self, id: EntryId) -> Option<(&TranslationCatalogEntry, &EntryStatus)> {
        self.session
            .entry_status
            .iter()
            .position(|status| status.id == id)
            .map(|index| {
                (
                    &self.session.catalog.entries[index],
                    &self.session.entry_status[index],
                )
            })
    }

    pub fn filtered_ids(&self, filter: &EntryFilter) -> Vec<EntryId> {
        let search = filter.search.trim().to_lowercase();
        self.session
            .catalog
            .entries
            .iter()
            .zip(&self.session.entry_status)
            .filter(|(entry, status)| {
                filter.state.is_none_or(|state| state == status.review)
                    && filter
                        .culture
                        .as_deref()
                        .is_none_or(|culture| same_culture(entry.culture.as_deref(), culture))
                    && (search.is_empty()
                        || [
                            &entry.key,
                            &entry.value,
                            entry.source.as_ref().unwrap_or(&entry.key),
                        ]
                        .iter()
                        .any(|value| value.to_lowercase().contains(&search)))
            })
            .map(|(_, status)| status.id)
            .collect()
    }

    pub fn metrics(&self, culture: Option<&str>) -> ReviewMetrics {
        let mut metrics = ReviewMetrics::default();
        for (entry, status) in self
            .session
            .catalog
            .entries
            .iter()
            .zip(&self.session.entry_status)
        {
            if culture.is_some_and(|culture| !same_culture(entry.culture.as_deref(), culture)) {
                continue;
            }
            metrics.total += 1;
            match status.review {
                TranslationReviewState::Missing => metrics.missing += 1,
                TranslationReviewState::Review => metrics.review += 1,
                TranslationReviewState::Approved => metrics.approved += 1,
            }
        }
        metrics
    }

    /// Distinct keys use the converter's last-entry-wins rule. Missing keys count.
    pub fn coverage(&self) -> Vec<TranslationCoverage> {
        let keys: BTreeSet<_> = self
            .session
            .catalog
            .entries
            .iter()
            .map(|e| &e.key)
            .collect();
        self.session
            .target_cultures
            .iter()
            .map(|culture| {
                let states: BTreeMap<_, _> = self
                    .session
                    .catalog
                    .entries
                    .iter()
                    .zip(&self.session.entry_status)
                    .filter(|(entry, _)| {
                        same_culture(entry.culture.as_deref(), &culture.culture_name)
                    })
                    .map(|(entry, status)| (&entry.key, status.review))
                    .collect();
                let mut metrics = ReviewMetrics {
                    total: keys.len(),
                    ..ReviewMetrics::default()
                };
                for key in &keys {
                    match states
                        .get(key)
                        .copied()
                        .unwrap_or(TranslationReviewState::Missing)
                    {
                        TranslationReviewState::Missing => metrics.missing += 1,
                        TranslationReviewState::Review => metrics.review += 1,
                        TranslationReviewState::Approved => metrics.approved += 1,
                    }
                }
                TranslationCoverage {
                    culture_name: culture.culture_name.clone(),
                    display_name: culture.display_name.clone(),
                    metrics,
                }
            })
            .collect()
    }

    pub fn set_target(
        &mut self,
        id: EntryId,
        value: impl Into<String>,
    ) -> Result<bool, StudioError> {
        let value = value.into();
        self.edit(id, |entry, status| {
            if entry.value != value {
                entry.value = value;
                status.review = if entry.value.trim().is_empty() {
                    TranslationReviewState::Missing
                } else {
                    TranslationReviewState::Review
                };
            }
        })
    }
    pub fn set_notes(&mut self, id: EntryId, notes: Option<String>) -> Result<bool, StudioError> {
        self.edit(id, |entry, _| entry.comment = notes)
    }
    pub fn set_review(
        &mut self,
        id: EntryId,
        review: TranslationReviewState,
    ) -> Result<bool, StudioError> {
        if review == TranslationReviewState::Approved
            && self
                .entry(id)
                .is_some_and(|(entry, _)| entry.value.trim().is_empty())
        {
            return Err(StudioError::InvalidInput(
                "An empty translation cannot be approved".into(),
            ));
        }
        self.edit(id, |_, status| status.review = review)
    }
    pub fn replace_entry(
        &mut self,
        id: EntryId,
        value: TranslationCatalogEntry,
        review: TranslationReviewState,
    ) -> Result<bool, StudioError> {
        if value.key.trim().is_empty() {
            return Err(StudioError::InvalidInput("Translation key is empty".into()));
        }
        if value.value.trim().is_empty() && review == TranslationReviewState::Approved {
            return Err(StudioError::InvalidInput(
                "An empty translation cannot be approved".into(),
            ));
        }
        self.edit(id, |entry, status| {
            *entry = value;
            status.review = review;
        })
    }

    pub fn add_entry(&mut self, entry: TranslationCatalogEntry) -> Result<EntryId, StudioError> {
        if entry.key.trim().is_empty() {
            return Err(StudioError::InvalidInput("Translation key is empty".into()));
        }
        if self.session.catalog.entries.iter().any(|existing| {
            existing.key.eq_ignore_ascii_case(&entry.key)
                && existing
                    .culture
                    .as_deref()
                    .unwrap_or("")
                    .eq_ignore_ascii_case(entry.culture.as_deref().unwrap_or(""))
        }) {
            return Err(StudioError::InvalidInput(
                "The culture/key already exists".into(),
            ));
        }
        let before = self.session.clone();
        let id = EntryId(self.next_entry_id);
        self.next_entry_id = self
            .next_entry_id
            .checked_add(1)
            .ok_or_else(|| StudioError::InvalidInput("Entry identities exhausted".into()))?;
        self.session.entry_status.push(EntryStatus {
            id,
            review: TranslationReviewState::from_entry(&entry),
            revision: self.next_revision,
        });
        self.session.catalog.entries.push(entry);
        self.finish_edit(before)?;
        Ok(id)
    }
    pub fn delete_entry(&mut self, id: EntryId) -> Result<(), StudioError> {
        let index = self.index(id)?;
        let before = self.session.clone();
        self.session.catalog.entries.remove(index);
        self.session.entry_status.remove(index);
        self.finish_edit(before)?;
        Ok(())
    }
    pub fn add_target_culture(
        &mut self,
        culture_name: &str,
        display_name: impl Into<String>,
    ) -> Result<bool, StudioError> {
        let culture_name = normalize_culture(culture_name)?;
        if self
            .session
            .target_cultures
            .iter()
            .any(|culture| culture.culture_name.eq_ignore_ascii_case(&culture_name))
        {
            return Ok(false);
        }
        let before = self.session.clone();
        self.session.target_cultures.push(CultureChoice {
            culture_name,
            display_name: display_name.into(),
        });
        self.finish_edit(before)
    }
    pub fn set_target_culture(&mut self, culture: &str) -> Result<bool, StudioError> {
        let culture = normalize_culture(culture)?;
        if self.session.target_culture == culture {
            return Ok(false);
        }
        let before = self.session.clone();
        if !self
            .session
            .target_cultures
            .iter()
            .any(|c| c.culture_name.eq_ignore_ascii_case(&culture))
        {
            self.session.target_cultures.push(CultureChoice {
                culture_name: culture.clone(),
                display_name: culture.clone(),
            });
        }
        self.session.target_culture = culture;
        self.finish_edit(before)
    }

    /// Export the normalized catalog with its original native format states.
    /// For Studio review states, explicitly export `catalog_with_review_states()`
    /// through `CatalogGateway`; its generic states may incur format-loss warnings.
    pub fn export(
        &self,
        format: TranslationFileFormat,
        options: &TranslationFormatOptions,
    ) -> Result<TranslationExportResult, StudioError> {
        CatalogGateway::export(&self.session.catalog, format, options)
    }

    /// Add missing rows for a target culture without replacing existing work.
    /// Source/context/plural metadata is copied from the last normalized key;
    /// new values and original file review states are cleared.
    pub fn materialize_target_culture(
        &mut self,
        culture: &str,
    ) -> Result<Vec<EntryId>, StudioError> {
        let culture = normalize_culture(culture)?;
        let existing: BTreeSet<_> = self
            .session
            .catalog
            .entries
            .iter()
            .filter(|entry| same_culture(entry.culture.as_deref(), &culture))
            .map(|entry| entry.key.clone())
            .collect();
        let sources: BTreeMap<_, _> = self
            .session
            .catalog
            .entries
            .iter()
            .filter(|entry| !existing.contains(&entry.key))
            .map(|entry| (entry.key.clone(), entry.clone()))
            .collect();
        self.transaction(|document| {
            document.add_target_culture(&culture, culture.clone())?;
            let mut added = Vec::new();
            for (_, mut entry) in sources {
                if entry.source.is_none() {
                    entry.source = Some(entry.value.clone());
                }
                entry.culture = Some(culture.clone());
                entry.value.clear();
                entry.state = None;
                added.push(document.add_entry(entry)?);
            }
            Ok(added)
        })
    }

    /// Explicit review-state projection for workflows that want Studio approvals
    /// in the exchange file. The preserved source catalog remains unchanged.
    pub fn catalog_with_review_states(&self) -> TranslationCatalog {
        let mut catalog = self.session.catalog.clone();
        for (entry, status) in catalog.entries.iter_mut().zip(&self.session.entry_status) {
            entry.state = Some(
                match status.review {
                    TranslationReviewState::Missing => "Missing",
                    TranslationReviewState::Review => "Review",
                    TranslationReviewState::Approved => "Approved",
                }
                .into(),
            );
        }
        catalog
    }

    pub fn entry_diagnostics(&self, id: EntryId) -> Result<Vec<String>, StudioError> {
        let (entry, _) = self.entry(id).ok_or(StudioError::EntryNotFound(id.0))?;
        let mut diagnostics = self
            .session
            .import_diagnostics
            .iter()
            .filter(|diagnostic| {
                diagnostic.key.as_deref().is_none_or(|key| key == entry.key)
                    && diagnostic.culture.as_deref().is_none_or(|culture| {
                        entry
                            .culture
                            .as_deref()
                            .is_none_or(|tag| tag.eq_ignore_ascii_case(culture))
                    })
            })
            .map(|diagnostic| diagnostic.message.clone())
            .collect::<Vec<_>>();
        diagnostics.extend(
            crate::validate_placeholders(
                entry.source.as_deref().unwrap_or(&entry.key),
                &entry.value,
                self.session.source_format,
            )
            .into_iter()
            .map(|diagnostic| diagnostic.message),
        );
        Ok(diagnostics)
    }

    /// One history record for a set of edits. Failed edits leave the document intact.
    pub fn transaction<R>(
        &mut self,
        action: impl FnOnce(&mut Self) -> Result<R, StudioError>,
    ) -> Result<R, StudioError> {
        let before = self.session.clone();
        let saved_revision = self.saved_revision;
        let generation = self.generation;
        let history_limit = self.history_limit;
        let next_revision = self.next_revision;
        let next_entry_id = self.next_entry_id;
        let undo = self.undo.clone();
        let redo = self.redo.clone();
        let result = action(self);
        self.next_revision = self.next_revision.max(next_revision);
        self.next_entry_id = self.next_entry_id.max(next_entry_id);
        self.undo = undo;
        self.redo = redo;
        match result {
            Ok(value) => {
                if self.session != before {
                    self.push_history(before);
                }
                Ok(value)
            }
            Err(error) => {
                self.session = before;
                self.saved_revision = saved_revision;
                if self.generation != generation {
                    self.generation = self.generation.max(generation).saturating_add(1);
                }
                self.history_limit = history_limit;
                Err(error)
            }
        }
    }

    pub fn undo(&mut self) -> Result<bool, StudioError> {
        let Some(history) = self.undo.pop_back() else {
            return Ok(false);
        };
        if let Err(error) = self.restore_history(history.before.clone()) {
            self.undo.push_back(history);
            return Err(error);
        }
        self.redo.push(history);
        Ok(true)
    }
    pub fn redo(&mut self) -> Result<bool, StudioError> {
        let Some(history) = self.redo.pop() else {
            return Ok(false);
        };
        if let Err(error) = self.restore_history(history.after.clone()) {
            self.redo.push(history);
            return Err(error);
        }
        self.undo.push_back(history);
        Ok(true)
    }

    fn index(&self, id: EntryId) -> Result<usize, StudioError> {
        self.session
            .entry_status
            .iter()
            .position(|status| status.id == id)
            .ok_or(StudioError::EntryNotFound(id.0))
    }
    fn edit(
        &mut self,
        id: EntryId,
        change: impl FnOnce(&mut TranslationCatalogEntry, &mut EntryStatus),
    ) -> Result<bool, StudioError> {
        let index = self.index(id)?;
        let before = self.session.clone();
        change(
            &mut self.session.catalog.entries[index],
            &mut self.session.entry_status[index],
        );
        if self.session == before {
            return Ok(false);
        }
        self.session.entry_status[index].revision = self.next_revision;
        self.finish_edit(before)
    }
    fn finish_edit(&mut self, before: TranslationSession) -> Result<bool, StudioError> {
        self.session.revision = match self.take_revision() {
            Ok(revision) => revision,
            Err(error) => {
                self.session = before;
                return Err(error);
            }
        };
        self.session.next_entry_id = self.next_entry_id;
        self.push_history(before);
        Ok(true)
    }
    fn push_history(&mut self, before: TranslationSession) {
        self.redo.clear();
        if self.history_limit > 0 {
            self.undo.push_back(HistoryEntry {
                before,
                after: self.session.clone(),
            });
            while self.undo.len() > self.history_limit {
                self.undo.pop_front();
            }
        }
    }
    fn take_revision(&mut self) -> Result<u64, StudioError> {
        let revision = self.next_revision;
        self.next_revision = revision
            .checked_add(1)
            .ok_or_else(|| StudioError::InvalidInput("Document revisions exhausted".into()))?;
        Ok(revision)
    }
    fn restore_history(&mut self, mut session: TranslationSession) -> Result<(), StudioError> {
        let revision = self.take_revision()?;
        for (entry, status) in session
            .catalog
            .entries
            .iter()
            .zip(&mut session.entry_status)
        {
            if self
                .entry(status.id)
                .is_none_or(|(current, current_status)| {
                    current != entry || current_status.review != status.review
                })
            {
                status.revision = revision;
            } else if let Some((_, current_status)) = self.entry(status.id) {
                status.revision = current_status.revision;
            }
        }
        session.revision = revision;
        session.next_entry_id = self.next_entry_id;
        self.session = session;
        Ok(())
    }
}

fn same_culture(entry: Option<&str>, selected: &str) -> bool {
    entry.unwrap_or("").eq_ignore_ascii_case(selected)
}
