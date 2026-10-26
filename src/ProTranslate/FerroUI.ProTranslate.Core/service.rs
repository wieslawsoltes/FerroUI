use crate::event::Event;
use crate::*;
use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet, VecDeque},
    rc::Rc,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranslationFallbackOptions {
    pub default_culture: Culture,
    pub fallback_cultures: Vec<Culture>,
    pub use_parent_cultures: bool,
    pub use_default_culture: bool,
    pub return_key_when_missing: bool,
    pub provider_failure_behavior: TranslationProviderFailureBehavior,
    pub format_failure_behavior: TranslationFormatFailureBehavior,
}
impl Default for TranslationFallbackOptions {
    fn default() -> Self {
        Self {
            default_culture: Culture::invariant(),
            fallback_cultures: Vec::new(),
            use_parent_cultures: true,
            use_default_culture: true,
            return_key_when_missing: true,
            provider_failure_behavior: Default::default(),
            format_failure_behavior: Default::default(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranslationCacheOptions {
    pub enabled: bool,
    pub cache_missing_translations: bool,
    pub clear_on_culture_changed: bool,
    /// Zero is unbounded, matching the source policy. Finite caches evict FIFO.
    pub maximum_entries: usize,
}
impl Default for TranslationCacheOptions {
    fn default() -> Self {
        Self {
            enabled: true,
            cache_missing_translations: true,
            clear_on_culture_changed: true,
            maximum_entries: 0,
        }
    }
}
type CacheKey = (Culture, String);
#[derive(Default)]
struct Cache {
    values: HashMap<CacheKey, LocalizedString>,
    order: VecDeque<CacheKey>,
    provider_revision: u64,
}
impl Cache {
    fn clear(&mut self) {
        self.values.clear();
        self.order.clear();
    }
}
struct TranslationServiceInner {
    provider: Rc<dyn TranslationProvider>,
    cultures: CultureService,
    generation: Cell<u64>,
    options: RefCell<TranslationFallbackOptions>,
    cache_options: RefCell<TranslationCacheOptions>,
    cache: RefCell<Cache>,
    changed: Event<CultureChangedEventArgs>,
    diagnostics: Event<ProTranslateDiagnostic>,
    diagnostic_sink: RefCell<Option<Rc<dyn ProTranslateDiagnosticSink>>>,
    _culture_subscription: Subscription,
}
#[derive(Clone)]
pub struct TranslationService(Rc<TranslationServiceInner>);
impl TranslationService {
    pub fn new(provider: Rc<dyn TranslationProvider>, cultures: CultureService) -> Self {
        Self::with_options(provider, cultures, Default::default(), Default::default())
    }
    pub fn with_options(
        provider: Rc<dyn TranslationProvider>,
        cultures: CultureService,
        options: TranslationFallbackOptions,
        cache_options: TranslationCacheOptions,
    ) -> Self {
        Self(Rc::new_cyclic(
            |weak: &std::rc::Weak<TranslationServiceInner>| {
                let weak = weak.clone();
                let subscription = cultures.subscribe(move |event| {
                    if let Some(service) = weak.upgrade() {
                        service
                            .generation
                            .set(service.generation.get().wrapping_add(1));
                        if service.cache_options.borrow().clear_on_culture_changed {
                            service.cache.borrow_mut().clear();
                        }
                        service.changed.emit(event.clone());
                    }
                });
                TranslationServiceInner {
                    provider,
                    cultures,
                    generation: Cell::new(0),
                    options: RefCell::new(options),
                    cache_options: RefCell::new(cache_options),
                    cache: RefCell::new(Cache::default()),
                    changed: Event::default(),
                    diagnostics: Event::default(),
                    diagnostic_sink: RefCell::new(None),
                    _culture_subscription: subscription,
                }
            },
        ))
    }
    pub fn cultures(&self) -> CultureService {
        self.0.cultures.clone()
    }
    pub fn current_culture(&self) -> Culture {
        self.0.cultures.current_culture()
    }
    pub fn current_ui_culture(&self) -> Culture {
        self.0.cultures.current_ui_culture()
    }
    pub fn options(&self) -> TranslationFallbackOptions {
        self.0.options.borrow().clone()
    }
    pub fn set_options(&self, options: TranslationFallbackOptions) {
        if *self.0.options.borrow() == options {
            return;
        }
        *self.0.options.borrow_mut() = options;
        self.refresh();
    }
    pub fn set_cache_options(&self, options: TranslationCacheOptions) {
        *self.0.cache_options.borrow_mut() = options;
        self.clear_cache();
    }
    pub fn set_diagnostic_sink(&self, sink: Option<Rc<dyn ProTranslateDiagnosticSink>>) {
        *self.0.diagnostic_sink.borrow_mut() = sink;
    }
    pub fn subscribe(&self, callback: impl Fn(&CultureChangedEventArgs) + 'static) -> Subscription {
        self.0.changed.subscribe(callback)
    }
    pub fn subscribe_diagnostics(
        &self,
        callback: impl Fn(&ProTranslateDiagnostic) + 'static,
    ) -> Subscription {
        self.0.diagnostics.subscribe(callback)
    }
    pub fn subscriber_count(&self) -> usize {
        self.0.changed.listener_count()
    }
    pub fn clear_cache(&self) {
        self.0.cache.borrow_mut().clear();
    }
    /// Explicitly announces mutable provider data or changed configuration.
    /// This also refreshes observable translations without changing culture.
    pub fn refresh(&self) {
        self.0
            .generation
            .set(self.0.generation.get().wrapping_add(1));
        self.clear_cache();
        let snapshot = self.0.cultures.snapshot();
        self.0.changed.emit(CultureChangedEventArgs {
            old: snapshot.clone(),
            new: snapshot,
        });
    }
    pub fn cached_lookup_count(&self) -> usize {
        self.0.cache.borrow().values.len()
    }
    pub fn remove_cached_string(&self, key: &str, culture: &Culture) -> bool {
        let cache_key = (culture.clone(), key.into());
        let mut cache = self.0.cache.borrow_mut();
        let removed = cache.values.remove(&cache_key).is_some();
        if removed {
            cache.order.retain(|key| key != &cache_key);
        }
        removed
    }
    pub fn fallback_cultures(&self, culture: &Culture) -> Vec<Culture> {
        let options = self.options();
        let mut seen = HashSet::new();
        let mut result = Vec::new();
        let mut append = |culture: &Culture| {
            let mut current = Some(culture.clone());
            while let Some(culture) = current {
                if seen.insert(culture.clone()) {
                    result.push(culture.clone());
                }
                current = if options.use_parent_cultures {
                    culture.parent()
                } else {
                    None
                };
            }
        };
        append(culture);
        for fallback in &options.fallback_cultures {
            append(fallback);
        }
        if options.use_default_culture {
            append(&options.default_culture);
        }
        result
    }
    pub fn get_string(&self, key: &str) -> Result<LocalizedString, TranslationError> {
        self.get_string_for(key, &self.current_ui_culture())
    }
    pub fn get_string_for(
        &self,
        key: &str,
        culture: &Culture,
    ) -> Result<LocalizedString, TranslationError> {
        if key.trim().is_empty() {
            return Err(TranslationError::InvalidKey);
        }
        let cache_options = self.0.cache_options.borrow().clone();
        let cache_key = (culture.clone(), key.into());
        let provider_revision = self.0.provider.revision();
        let generation = self.0.generation.get();
        if cache_options.enabled {
            let mut cache = self.0.cache.borrow_mut();
            if cache.provider_revision != provider_revision {
                cache.clear();
                cache.provider_revision = provider_revision;
            }
            if let Some(value) = cache.values.get(&cache_key) {
                return Ok(value.clone());
            }
        }
        let options = self.options();
        let mut all_diagnostics = Vec::new();
        let mut result = None;
        for candidate in self.fallback_cultures(culture) {
            match self.0.provider.get_string(key, &candidate) {
                Ok(mut localized) => {
                    for diagnostic in &localized.diagnostics {
                        self.report(diagnostic.clone());
                    }
                    all_diagnostics.append(&mut localized.diagnostics);
                    if !localized.resource_not_found {
                        localized.diagnostics = std::mem::take(&mut all_diagnostics);
                        result = Some(localized);
                        break;
                    }
                }
                Err(error) => {
                    let diagnostic = ProTranslateDiagnostic::provider(
                        key,
                        &candidate,
                        self.0.provider.name(),
                        error.clone(),
                    );
                    self.report(diagnostic.clone());
                    all_diagnostics.push(diagnostic);
                    if options.provider_failure_behavior
                        == TranslationProviderFailureBehavior::ReportAndReturnError
                    {
                        return Err(error);
                    }
                }
            }
        }
        let resolved = result.unwrap_or_else(|| {
            let diagnostic = ProTranslateDiagnostic {
                kind: ProTranslateDiagnosticKind::MissingTranslation,
                severity: ProTranslateDiagnosticSeverity::Warning,
                message: format!("Translation '{key}' was not found for culture '{culture}'"),
                key: Some(key.into()),
                culture: Some(culture.clone()),
                provider_name: Some(self.0.provider.name().into()),
                error: None,
            };
            self.report(diagnostic.clone());
            all_diagnostics.push(diagnostic);
            let mut localized =
                LocalizedString::missing(key, culture.clone(), self.0.provider.name());
            if !options.return_key_when_missing {
                localized.value.clear();
            }
            localized.diagnostics = all_diagnostics;
            localized
        });
        if cache_options.enabled
            && (!resolved.resource_not_found || cache_options.cache_missing_translations)
        {
            // A callback may mutate provider data or policy while diagnostics run;
            // do not publish a stale resolution into the new cache generation.
            if self.0.provider.revision() == provider_revision
                && self.options() == options
                && self.0.generation.get() == generation
                && *self.0.cache_options.borrow() == cache_options
            {
                let mut cache = self.0.cache.borrow_mut();
                if cache_options.maximum_entries > 0 {
                    while cache.values.len() >= cache_options.maximum_entries {
                        if let Some(old) = cache.order.pop_front() {
                            cache.values.remove(&old);
                        } else {
                            break;
                        }
                    }
                }
                cache.order.push_back(cache_key.clone());
                cache.values.insert(cache_key, resolved.clone());
            }
        }
        Ok(resolved)
    }
    pub fn format(
        &self,
        key: &str,
        arguments: &[FormatArgument],
    ) -> Result<String, TranslationError> {
        let snapshot = self.0.cultures.snapshot();
        let localized = self.get_string_for(key, &snapshot.ui_culture)?;
        if arguments.is_empty() {
            return Ok(localized.value);
        }
        self.format_localized(localized, arguments, &snapshot.culture)
    }
    pub fn format_for(
        &self,
        key: &str,
        arguments: &[FormatArgument],
        culture: &Culture,
    ) -> Result<String, TranslationError> {
        let localized = self.get_string_for(key, culture)?;
        if arguments.is_empty() {
            return Ok(localized.value);
        }
        self.format_localized(localized, arguments, culture)
    }
    pub fn format_localized(
        &self,
        localized: LocalizedString,
        arguments: &[FormatArgument],
        culture: &Culture,
    ) -> Result<String, TranslationError> {
        match format_composite(&localized.value, arguments, culture) {
            Ok(value) => Ok(value),
            Err(error) => {
                self.report(ProTranslateDiagnostic {
                    kind: ProTranslateDiagnosticKind::FormatFailure,
                    severity: ProTranslateDiagnosticSeverity::Error,
                    message: format!(
                        "Localized format '{}' failed for '{}': {error}",
                        localized.key, culture
                    ),
                    key: Some(localized.key),
                    culture: Some(culture.clone()),
                    provider_name: Some(localized.provider_name),
                    error: Some(error.clone()),
                });
                if self.options().format_failure_behavior
                    == TranslationFormatFailureBehavior::ReportAndReturnError
                {
                    Err(error)
                } else {
                    Ok(localized.value)
                }
            }
        }
    }
    /// Format an adapter-provided fallback or outer format with the configured
    /// failure policy and diagnostic sink.
    pub fn format_value(
        &self,
        key: &str,
        template: &str,
        arguments: &[FormatArgument],
        culture: &Culture,
    ) -> Result<String, TranslationError> {
        self.format_localized(
            LocalizedString::found(key, template, culture.clone(), self.0.provider.name()),
            arguments,
            culture,
        )
    }
    pub fn observe(
        &self,
        key: impl Into<String>,
        arguments: Vec<FormatArgument>,
    ) -> ObservableLocalizedString {
        ObservableLocalizedString::new(self.clone(), key.into(), arguments)
    }
    pub(crate) fn report(&self, diagnostic: ProTranslateDiagnostic) {
        let sink = self.0.diagnostic_sink.borrow().clone();
        if let Some(sink) = sink {
            sink.report(&diagnostic);
        }
        self.0.diagnostics.emit(diagnostic);
    }
}
struct ObservableInner {
    service: TranslationService,
    key: String,
    arguments: RefCell<Vec<FormatArgument>>,
    value: RefCell<Result<String, TranslationError>>,
    changed: Event<Result<String, TranslationError>>,
    _subscription: Subscription,
}
#[derive(Clone)]
pub struct ObservableLocalizedString(Rc<ObservableInner>);
impl ObservableLocalizedString {
    fn new(service: TranslationService, key: String, arguments: Vec<FormatArgument>) -> Self {
        let value = service.format(&key, &arguments);
        Self(Rc::new_cyclic(|weak: &std::rc::Weak<ObservableInner>| {
            let weak = weak.clone();
            let subscription = service.subscribe(move |_| {
                if let Some(inner) = weak.upgrade() {
                    Self(inner).refresh();
                }
            });
            ObservableInner {
                service,
                key,
                arguments: RefCell::new(arguments),
                value: RefCell::new(value),
                changed: Event::default(),
                _subscription: subscription,
            }
        }))
    }
    pub fn key(&self) -> &str {
        &self.0.key
    }
    pub fn value(&self) -> Result<String, TranslationError> {
        self.0.value.borrow().clone()
    }
    pub fn set_arguments(&self, arguments: Vec<FormatArgument>) {
        *self.0.arguments.borrow_mut() = arguments;
        self.refresh();
    }
    pub fn refresh(&self) {
        let arguments = self.0.arguments.borrow().clone();
        let generation = self.0.service.0.generation.get();
        let value = self.0.service.format(&self.0.key, &arguments);
        // Reentrant diagnostics may have requested a refresh or changed culture.
        // That event is already queued; it will compute the newest value.
        if self.0.service.0.generation.get() != generation {
            return;
        }
        let changed = {
            let mut previous = self.0.value.borrow_mut();
            if *previous == value {
                false
            } else {
                *previous = value.clone();
                true
            }
        };
        if changed {
            self.0.changed.emit(value);
        }
    }
    pub fn subscribe(
        &self,
        callback: impl Fn(&Result<String, TranslationError>) + 'static,
    ) -> Subscription {
        self.0.changed.subscribe(callback)
    }
}
