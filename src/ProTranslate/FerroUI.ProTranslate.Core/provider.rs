use crate::{Culture, TranslationError};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProTranslateDiagnosticKind {
    MissingTranslation,
    ProviderFailure,
    FormatFailure,
    RegionProfileFailure,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProTranslateDiagnosticSeverity {
    Information,
    Warning,
    Error,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProTranslateDiagnostic {
    pub kind: ProTranslateDiagnosticKind,
    pub severity: ProTranslateDiagnosticSeverity,
    pub message: String,
    pub key: Option<String>,
    pub culture: Option<Culture>,
    pub provider_name: Option<String>,
    pub error: Option<TranslationError>,
}
impl ProTranslateDiagnostic {
    pub(crate) fn provider(
        key: &str,
        culture: &Culture,
        provider: &str,
        error: TranslationError,
    ) -> Self {
        Self {
            kind: ProTranslateDiagnosticKind::ProviderFailure,
            severity: ProTranslateDiagnosticSeverity::Error,
            message: format!("Provider '{provider}' failed for '{key}' in '{culture}': {error}"),
            key: Some(key.into()),
            culture: Some(culture.clone()),
            provider_name: Some(provider.into()),
            error: Some(error),
        }
    }
}
pub trait ProTranslateDiagnosticSink {
    fn report(&self, diagnostic: &ProTranslateDiagnostic);
}
impl<F: Fn(&ProTranslateDiagnostic)> ProTranslateDiagnosticSink for F {
    fn report(&self, diagnostic: &ProTranslateDiagnostic) {
        self(diagnostic);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalizedString {
    pub key: String,
    pub value: String,
    pub culture: Culture,
    pub resource_not_found: bool,
    pub provider_name: String,
    pub diagnostics: Vec<ProTranslateDiagnostic>,
}
impl LocalizedString {
    pub fn found(
        key: impl Into<String>,
        value: impl Into<String>,
        culture: Culture,
        provider_name: impl Into<String>,
    ) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
            culture,
            resource_not_found: false,
            provider_name: provider_name.into(),
            diagnostics: Vec::new(),
        }
    }
    pub fn missing(
        key: impl Into<String>,
        culture: Culture,
        provider_name: impl Into<String>,
    ) -> Self {
        let key = key.into();
        Self {
            value: key.clone(),
            key,
            culture,
            resource_not_found: true,
            provider_name: provider_name.into(),
            diagnostics: Vec::new(),
        }
    }
}
impl std::fmt::Display for LocalizedString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.value)
    }
}
/// A provider performs one exact-culture lookup; the service owns fallback.
/// Empty values are successful translations. Failures are distinct from misses.
pub trait TranslationProvider {
    fn name(&self) -> &str;
    fn get_string(&self, key: &str, culture: &Culture)
        -> Result<LocalizedString, TranslationError>;
    /// Change this token when mutable catalog contents change. The service
    /// invalidates cached hits and misses before its next lookup.
    fn revision(&self) -> u64 {
        0
    }
}
pub struct InMemoryTranslationProvider {
    name: String,
    entries: RefCell<HashMap<(Culture, String), String>>,
    revision: Cell<u64>,
}
impl Default for InMemoryTranslationProvider {
    fn default() -> Self {
        Self {
            name: "InMemory".into(),
            entries: RefCell::default(),
            revision: Cell::new(0),
        }
    }
}
/// Validate a resource key at a provider boundary, including direct lookups that
/// do not pass through a translation service.
pub fn validate_translation_key(key: &str) -> Result<(), TranslationError> {
    if key.trim().is_empty() {
        Err(TranslationError::InvalidKey)
    } else {
        Ok(())
    }
}
fn validate_provider_name(name: &str) -> Result<(), TranslationError> {
    if name.trim().is_empty() {
        Err(TranslationError::InvalidProviderName)
    } else {
        Ok(())
    }
}
impl InMemoryTranslationProvider {
    /// Construct a named catalog. Use `try_new` for names supplied by users.
    ///
    /// # Panics
    /// Panics if the name is empty or whitespace.
    pub fn new(name: impl Into<String>) -> Self {
        Self::try_new(name).expect("a translation provider must have a nonempty name")
    }
    pub fn try_new(name: impl Into<String>) -> Result<Self, TranslationError> {
        let name = name.into();
        validate_provider_name(&name)?;
        Ok(Self {
            name,
            ..Self::default()
        })
    }
    /// Insert a translation. Empty values are intentional translations.
    ///
    /// # Panics
    /// Panics if the key is empty or whitespace; `try_insert` is recoverable.
    pub fn insert(
        &self,
        culture: Culture,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Option<String> {
        self.try_insert(culture, key, value)
            .expect("a translation key must not be empty or whitespace")
    }
    pub fn try_insert(
        &self,
        culture: Culture,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<Option<String>, TranslationError> {
        let key = key.into();
        validate_translation_key(&key)?;
        let result = self
            .entries
            .borrow_mut()
            .insert((culture, key), value.into());
        self.revision.set(self.revision.get().wrapping_add(1));
        Ok(result)
    }
    /// Add a culture's catalog atomically: an invalid key rejects the complete
    /// batch without changing translations or the provider revision.
    pub fn insert_many<K: Into<String>, V: Into<String>>(
        &self,
        culture: Culture,
        translations: impl IntoIterator<Item = (K, V)>,
    ) -> Result<(), TranslationError> {
        let translations = translations
            .into_iter()
            .map(|(key, value)| {
                let key = key.into();
                validate_translation_key(&key)?;
                Ok((key, value.into()))
            })
            .collect::<Result<Vec<_>, TranslationError>>()?;
        if !translations.is_empty() {
            self.entries.borrow_mut().extend(
                translations
                    .into_iter()
                    .map(|(key, value)| ((culture.clone(), key), value)),
            );
            self.revision.set(self.revision.get().wrapping_add(1));
        }
        Ok(())
    }
    pub fn remove(&self, culture: &Culture, key: &str) -> Option<String> {
        let result = self
            .entries
            .borrow_mut()
            .remove(&(culture.clone(), key.into()));
        if result.is_some() {
            self.revision.set(self.revision.get().wrapping_add(1));
        }
        result
    }
    pub fn len(&self) -> usize {
        self.entries.borrow().len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.borrow().is_empty()
    }
    pub fn clear(&self) {
        self.entries.borrow_mut().clear();
        self.revision.set(self.revision.get().wrapping_add(1));
    }
}
impl TranslationProvider for InMemoryTranslationProvider {
    fn name(&self) -> &str {
        &self.name
    }
    fn get_string(
        &self,
        key: &str,
        culture: &Culture,
    ) -> Result<LocalizedString, TranslationError> {
        validate_translation_key(key)?;
        Ok(
            match self.entries.borrow().get(&(culture.clone(), key.into())) {
                Some(value) => LocalizedString::found(key, value, culture.clone(), &self.name),
                None => LocalizedString::missing(key, culture.clone(), &self.name),
            },
        )
    }
    fn revision(&self) -> u64 {
        self.revision.get()
    }
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TranslationProviderFailureBehavior {
    #[default]
    ReportAndContinue,
    ReportAndReturnError,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TranslationFormatFailureBehavior {
    #[default]
    ReportAndReturnUnformatted,
    ReportAndReturnError,
}
/// Ordered, composable providers. Provider failures remain in result diagnostics
/// even if a later provider successfully resolves the value.
pub struct CompositeTranslationProvider {
    providers: Vec<Rc<dyn TranslationProvider>>,
    failure_behavior: TranslationProviderFailureBehavior,
    child_revisions: RefCell<Vec<u64>>,
    revision: Cell<u64>,
}
impl CompositeTranslationProvider {
    pub fn new(providers: Vec<Rc<dyn TranslationProvider>>) -> Self {
        Self {
            providers,
            failure_behavior: Default::default(),
            child_revisions: RefCell::default(),
            revision: Cell::new(0),
        }
    }
    pub fn with_failure_behavior(mut self, behavior: TranslationProviderFailureBehavior) -> Self {
        self.failure_behavior = behavior;
        self
    }
    pub fn providers(&self) -> &[Rc<dyn TranslationProvider>] {
        &self.providers
    }
}
impl TranslationProvider for CompositeTranslationProvider {
    fn name(&self) -> &str {
        "Composite"
    }
    fn get_string(
        &self,
        key: &str,
        culture: &Culture,
    ) -> Result<LocalizedString, TranslationError> {
        validate_translation_key(key)?;
        let mut diagnostics = Vec::new();
        for provider in &self.providers {
            match provider.get_string(key, culture) {
                Ok(mut localized) => {
                    diagnostics.append(&mut localized.diagnostics);
                    if !localized.resource_not_found {
                        localized.diagnostics = diagnostics;
                        return Ok(localized);
                    }
                }
                Err(error) => {
                    if self.failure_behavior
                        == TranslationProviderFailureBehavior::ReportAndReturnError
                    {
                        return Err(error);
                    }
                    diagnostics.push(ProTranslateDiagnostic::provider(
                        key,
                        culture,
                        provider.name(),
                        error,
                    ));
                }
            }
        }
        let mut missing = LocalizedString::missing(key, culture.clone(), self.name());
        missing.diagnostics = diagnostics;
        Ok(missing)
    }
    fn revision(&self) -> u64 {
        // Comparing the actual revision vector avoids collisions from hashing
        // unrelated mutable providers into a single cache token.
        let revisions: Vec<_> = self.providers.iter().map(|p| p.revision()).collect();
        let mut previous = self.child_revisions.borrow_mut();
        if *previous != revisions {
            *previous = revisions;
            self.revision.set(self.revision.get().wrapping_add(1));
        }
        self.revision.get()
    }
}

/// Host bridge for resource managers, localizer services, embedded stores, and
/// other application providers. `Ok(None)` is a miss; `Ok(Some(""))` is a hit.
/// Hosts remain responsible for calling `TranslationService::refresh` after
/// changing external data when already-bound values must refresh immediately.
pub struct CallbackTranslationProvider<F> {
    name: String,
    callback: F,
    revision: Cell<u64>,
}
impl<F> CallbackTranslationProvider<F>
where
    F: Fn(&str, &Culture) -> Result<Option<String>, TranslationError>,
{
    pub fn new(name: impl Into<String>, callback: F) -> Self {
        Self::try_new(name, callback).expect("a translation provider must have a nonempty name")
    }
    pub fn try_new(name: impl Into<String>, callback: F) -> Result<Self, TranslationError> {
        let name = name.into();
        validate_provider_name(&name)?;
        Ok(Self {
            name,
            callback,
            revision: Cell::new(0),
        })
    }
    pub fn invalidate(&self) {
        self.revision.set(self.revision.get().wrapping_add(1));
    }
}
impl<F> TranslationProvider for CallbackTranslationProvider<F>
where
    F: Fn(&str, &Culture) -> Result<Option<String>, TranslationError>,
{
    fn name(&self) -> &str {
        &self.name
    }
    fn get_string(
        &self,
        key: &str,
        culture: &Culture,
    ) -> Result<LocalizedString, TranslationError> {
        validate_translation_key(key)?;
        Ok(match (self.callback)(key, culture)? {
            Some(value) => LocalizedString::found(key, value, culture.clone(), &self.name),
            None => LocalizedString::missing(key, culture.clone(), &self.name),
        })
    }
    fn revision(&self) -> u64 {
        self.revision.get()
    }
}
