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
#[derive(Default)]
pub struct InMemoryTranslationProvider {
    name: String,
    entries: RefCell<HashMap<(Culture, String), String>>,
    revision: Cell<u64>,
}
impl InMemoryTranslationProvider {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Self::default()
        }
    }
    pub fn insert(
        &self,
        culture: Culture,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Option<String> {
        let result = self
            .entries
            .borrow_mut()
            .insert((culture, key.into()), value.into());
        self.revision.set(self.revision.get().wrapping_add(1));
        result
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
}
impl CompositeTranslationProvider {
    pub fn new(providers: Vec<Rc<dyn TranslationProvider>>) -> Self {
        Self {
            providers,
            failure_behavior: Default::default(),
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
        self.providers.iter().fold(0xcbf29ce484222325, |v, p| {
            (v ^ p.revision()).wrapping_mul(0x100000001b3)
        })
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
        Self {
            name: name.into(),
            callback,
            revision: Cell::new(0),
        }
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
        Ok(match (self.callback)(key, culture)? {
            Some(value) => LocalizedString::found(key, value, culture.clone(), &self.name),
            None => LocalizedString::missing(key, culture.clone(), &self.name),
        })
    }
    fn revision(&self) -> u64 {
        self.revision.get()
    }
}
