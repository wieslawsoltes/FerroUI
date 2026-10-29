//! Development reload transactions. All targets and commits remain on the creating UI thread.
#![cfg(debug_assertions)]
mod component;
mod state_scope;
mod handlers;
#[cfg(test)]
mod handler_tests;
#[cfg(test)]
mod template_state_tests;
mod detached_context;
mod deferred_context;
mod frame_timing;
pub use frame_timing::record_frame_timing;
mod input_pause;
mod include_tracking;
pub use input_pause::InputPause;
mod restart_handoff;
pub use restart_handoff::{RestartHandoff, RestartParticipant, RestartResume};
mod panel;
mod resource_preparation;
mod resources;
pub use resource_preparation::ResourcePreparation;
mod root_properties;
mod root_resources;
mod root_styles;
mod root_preview;
#[cfg(test)]
mod root_binding_tests;
mod styles;
pub use panel::PanelReloadTarget;
pub use resources::ResourceReloadTarget;
pub use styles::StyleReloadTarget;
mod content;
mod control_state;
pub use control_state::{CapturedControlState, ControlStateAdapter, ControlStateRestoreContext};
pub use component::{ComponentRefresh, ComponentReloadTarget};
mod contract;
mod identity;
mod manifest;
mod state;
pub use identity::KEY_NAMESPACE;
mod watcher;
pub use content::ContentReloadTarget;
pub use contract::DocumentContract;
use ferroui_markup_xaml::source_overlay::{with_sources, Sources};
pub use manifest::ReloadManifest;
use std::{
    collections::{BTreeMap, BTreeSet},
    rc::{Rc, Weak},
    time::{Duration, Instant},
};
pub use watcher::{FileBatch, FileChange, FileWatcher, WatchDirectory, WatchRevision};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReloadError {
    Failed(String),
    RestartRequired(String),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReloadStatus {
    Applied,
    Failed,
    RestartRequired,
    Superseded,
}
#[derive(Debug, Clone)]
pub struct ReloadOutcome {
    pub revision: u64,
    pub documents: Vec<String>,
    pub status: ReloadStatus,
    pub diagnostic: Option<String>,
    pub preparation: Duration,
    pub application: Duration,
}
/// An adapter must keep its prior state until `finish`. Rollback must also undo a partially
/// failed commit. Preparation must not change the live tree or run irreversible user effects.
pub trait PreparedReload {
    fn commit(&mut self) -> Result<(), ReloadError>;
    fn rollback(&mut self);
    fn finish(&mut self) {}
}
pub trait ReloadTarget {
    /// Complete state restoration from an earlier accepted revision at the UI
    /// boundary. Runs against accepted sources, before candidate resource previews
    /// or preparation. Custom adapters normally need only the default no-op.
    fn complete_pending_state(&self) {}
    /// Includes transitive document dependencies. A target may refresh this list after each revision.
    fn documents(&self) -> Vec<String>;
    fn prepare(&self, sources: Rc<Sources>) -> Result<Box<dyn PreparedReload>, ReloadError>;
    /// Resource owners provide a lazy preview before ordinary view preparation.
    fn resource_preparation(
        self: Rc<Self>,
        _sources: Rc<Sources>,
    ) -> Result<Option<ResourcePreparation>, ReloadError> {
        Ok(None)
    }
}
/// Sources become authoritative only after every affected live target commits successfully.
/// Session ownership is intentionally `!Send`: callers must pump it on the UI thread.
#[derive(Default)]
pub struct HotReloadSession {
    watch_revision: Option<WatchRevision>,
    accepted: Rc<Sources>,
    published: Rc<std::cell::RefCell<Rc<Sources>>>,
    overrides: Vec<(
        &'static ferroui_base::TypeInfo,
        ferroui_markup_xaml::PopulateOverride,
    )>,
    source_registration: Option<ferroui_markup_xaml::source_overlay::SourceRegistration>,
    contracts: BTreeMap<String, DocumentContract>,
    build_contracts: Option<Rc<contract::BuildContracts>>,
    pending: BTreeMap<String, String>,
    read_errors: BTreeMap<String, String>,
    watch_errors: Vec<String>,
    targets: Vec<Weak<dyn ReloadTarget>>,
    includes: include_tracking::IncludeTracking,
    revision: u64,
    applied_revision: u64,
    attempted_revision: u64,
}
impl HotReloadSession {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn register_document(&mut self, uri: String, source: String) -> Result<(), ReloadError> {
        let uri = document_key(&uri);
        if self.contracts.contains_key(&uri) {
            return Err(ReloadError::Failed(format!(
                "Document already registered: {uri}"
            )));
        }
        let contract = DocumentContract::parse_with_build(&source, self.build_contracts.as_deref())?;
        if self.source_registration.is_none() {
            self.source_registration = Some(
                ferroui_markup_xaml::source_overlay::SourceRegistration::new(&self.published)
                    .map_err(ReloadError::Failed)?,
            );
        }
        self.source_registration
            .as_ref()
            .unwrap()
            .check_document(&uri)
            .map_err(ReloadError::Failed)?;
        self.source_registration.as_ref().unwrap().register_compiled_source(&uri, &source);
        self.contracts.insert(uri.clone(), contract);
        Rc::make_mut(&mut self.accepted).insert(uri, source);
        *self.published.borrow_mut() = self.accepted.clone();
        Ok(())
    }
    /// New instances populate from the last accepted snapshot. Existing instances need
    /// a ReloadTarget adapter; this hook never clears a live root.
    pub fn bind_class(
        &mut self,
        class: &'static ferroui_base::TypeInfo,
        uri: String,
        configuration: ferroui_markup_xaml::RuntimeXamlLoaderConfiguration,
    ) -> Result<(), ReloadError> {
        use ferroui_markup_xaml::{FerroXamlLoader, RuntimeXamlLoaderDocument};
        let uri = document_key(&uri);
        if !self.accepted.contains_key(&uri) {
            return Err(ReloadError::Failed(format!("Unregistered document: {uri}")));
        }
        if FerroXamlLoader::populate_override(class).is_some() {
            return Err(ReloadError::Failed(
                "Class already has a populate override".into(),
            ));
        }
        let base = ferroui_base::utilities::Uri::try_create(
            &uri,
            ferroui_base::utilities::UriKind::Absolute,
        )
        .ok_or_else(|| ReloadError::Failed("Document URI must be absolute".into()))?;
        let sources = self.published.clone();
        let populate: ferroui_markup_xaml::PopulateOverride = Rc::new(move |sp, root| {
            let sources = sources.borrow().clone();
            let text = sources.get(&uri).ok_or_else(|| {
                ferroui_markup_xaml::XamlLoadException::with_message("Reload document was removed")
            })?;
            let prepared = crate::identity::runtime_source(text).map_err(|error| {
                ferroui_markup_xaml::XamlLoadException::with_message(format!("{error:?}"))
            })?;
            let mut document = RuntimeXamlLoaderDocument::with_base_uri_and_root_instance(
                Some(base.clone()),
                Some(root.clone()),
                &prepared,
            );
            document.service_provider = sp.cloned();
            with_sources(sources, || {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load_document(
                    document,
                    Some(configuration.clone()),
                )
                .map(|_| ())
            })
        });
        FerroXamlLoader::set_populate_override(class, Some(populate.clone()));
        self.overrides.push((class, populate));
        Ok(())
    }

    /// The application owns its adapters. Closed components disappear when their adapters are dropped.
    pub fn register_target(&mut self, target: &Rc<dyn ReloadTarget>) {
        self.targets.push(Rc::downgrade(target));
        if !self.pending.is_empty() {
            self.revision += 1;
        }
    }
    /// Track direct resource includes in this container, including later additions
    /// and removals. Only includes whose documents belong to this session are adopted.
    /// Owners are weakly held. Repeated registration keeps the first configuration.
    /// All registered targets conservatively depend on these includes, so static
    /// consumers prepare with the same resource revision. Do not also manually
    /// register adapters for the same contributions.
    pub fn track_resource_includes(
        &mut self,
        owner: &ferroui_base::Ref<ferroui_base::controls::ResourceDictionary>,
        configuration: ferroui_markup_xaml::RuntimeXamlLoaderConfiguration,
    ) {
        self.includes.resources(owner, configuration);
    }
    /// Track direct style includes with the same ownership and dependency rules as
    /// `track_resource_includes`. Arbitrary inline styles need explicit adapters.
    pub fn track_style_includes(
        &mut self,
        owner: &ferroui_base::Ref<ferroui_base::styling::Styles>,
        configuration: ferroui_markup_xaml::RuntimeXamlLoaderConfiguration,
    ) {
        self.includes.styles(owner, configuration);
    }
    pub fn submit(&mut self, uri: String, text: String) -> u64 {
        self.revision += 1;
        let uri = document_key(&uri);
        self.read_errors.remove(&uri);
        self.pending.insert(uri, text);
        self.revision
    }
    /// Reapply an explicitly registered view after its Rust builder changes.
    /// Keep a pending editor draft and any read errors: a builder request must
    /// not silently replace newer text with the accepted source or hide a failure.
    pub fn reapply_document(&mut self, uri: &str) -> Result<u64, ReloadError> {
        let uri = document_key(uri);
        let source = self
            .pending
            .get(&uri)
            .or_else(|| self.accepted.get(&uri))
            .cloned()
            .ok_or_else(|| {
                ReloadError::RestartRequired(format!("Unregistered builder document: {uri}"))
            })?;
        self.revision += 1;
        self.pending.insert(uri, source);
        Ok(self.revision)
    }
    /// Submit editor/supervisor text as one revision, without rebuilding targets
    /// for unchanged accepted documents. Returning to accepted text discards that
    /// document's failed draft; drafts and read errors for other documents remain.
    /// Resubmitting a pending draft explicitly retries its failed transaction.
    /// Use `submit` when a view must be reapplied even with identical markup.
    pub fn submit_texts(&mut self, documents: impl IntoIterator<Item = (String, String)>) -> u64 {
        let mut changed = false;
        for (uri, text) in documents {
            let uri = document_key(&uri);
            changed |= self.read_errors.remove(&uri).is_some();
            if self.accepted.get(&uri) == Some(&text) {
                changed |= self.pending.remove(&uri).is_some();
            } else {
                self.pending.insert(uri, text);
                changed = true;
            }
        }
        if changed {
            self.revision += 1;
        }
        self.revision
    }
    /// Submit a complete watcher snapshot as one revision. Read errors block the
    /// transaction, including older pending edits, until that document recovers.
    /// Editors can recover an unreadable file by submitting its unsaved text.
    pub fn submit_files(&mut self, batch: FileBatch) -> u64 {
        self.revision += 1;
        self.watch_revision = batch.revision;
        self.watch_errors = batch.diagnostics;
        for file in batch.files {
            let uri = document_key(&file.uri);
            match file.contents {
                Ok(text) => {
                    self.read_errors.remove(&uri);
                    if self.accepted.get(&uri) == Some(&text) {
                        self.pending.remove(&uri);
                    } else {
                        self.pending.insert(uri, text);
                    }
                }
                Err(error) => {
                    self.read_errors.insert(uri, error);
                }
            }
        }
        self.revision
    }
    pub fn sources(&self) -> Rc<Sources> {
        self.accepted.clone()
    }
    pub fn applied_revision(&self) -> u64 {
        self.applied_revision
    }
    pub fn has_pending_changes(&self) -> bool {
        !self.pending.is_empty()
            || !self.read_errors.is_empty()
            || !self.watch_errors.is_empty()
            || self
                .watch_revision
                .as_ref()
                .is_some_and(|revision| !revision.is_current())
    }
    pub fn apply_pending(&mut self) -> Option<ReloadOutcome> {
        self.apply_pending_with(|| Ok(()))
    }
    /// Coordinate an external update with the markup transaction. The callback
    /// runs once, after every target commits but before accepted sources publish
    /// and old graphs/subscriptions are released. A callback error rolls back all
    /// target commits. Preparation/commit failure never invokes the callback.
    ///
    /// The callback must leave its own state unchanged on error or panic. After
    /// success, adapter `finish` is infallible cleanup: it must not panic. This
    /// cannot roll back arbitrary external effects or an already installed patch.
    /// A supervisor can install a validated synchronous function patch here and
    /// publish both revisions only after installation succeeds. Observers invoked
    /// synchronously by adapter commit still see the provisional graph; adapters
    /// must honor their existing reversible-side-effect contract.
    pub fn apply_pending_with(
        &mut self,
        finalize: impl FnOnce() -> Result<(), ReloadError>,
    ) -> Option<ReloadOutcome> {
        if self.pending.is_empty() && self.read_errors.is_empty() && self.watch_errors.is_empty() {
            return None;
        }
        let start = Instant::now();
        let discovered = catch_reload(|| self.includes.refresh(&self.accepted));
        if self.attempted_revision == self.revision
            && discovered.as_ref().is_ok_and(|(_, changed)| *changed)
        {
            // A newly attached include can validate a previously rejected draft
            // without requiring another filesystem save.
            self.revision += 1;
        }
        if self.attempted_revision == self.revision {
            return None;
        }
        self.attempted_revision = self.revision;
        // Keep failed drafts: a later fix must retry the entire coherent batch, including
        // edits to other documents whose watcher has already observed their saved contents.
        let pending = self.pending.clone();
        let documents: Vec<_> = pending
            .keys()
            .chain(self.read_errors.keys())
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let mut outcome = ReloadOutcome {
            revision: self.revision,
            documents,
            status: ReloadStatus::Applied,
            diagnostic: None,
            preparation: Duration::ZERO,
            application: Duration::ZERO,
        };
        if self
            .watch_revision
            .as_ref()
            .is_some_and(|revision| !revision.is_current())
        {
            outcome.status = ReloadStatus::Superseded;
            outcome.diagnostic =
                Some("A newer source read superseded this document revision".into());
            return Some(outcome);
        }
        if !self.read_errors.is_empty() || !self.watch_errors.is_empty() {
            outcome.fail(ReloadError::Failed(
                self.read_errors
                    .iter()
                    .map(|(uri, error)| format!("{uri}: {error}"))
                    .chain(self.watch_errors.iter().cloned())
                    .collect::<Vec<_>>()
                    .join("; "),
            ));
            return Some(outcome);
        }
        let completion = with_sources(self.accepted.clone(), || {
            for target in self.targets.iter().filter_map(Weak::upgrade) {
                catch_reload(|| {
                    target.complete_pending_state();
                    Ok(())
                })?;
            }
            Ok::<_, ReloadError>(())
        });
        if let Err(error) = completion {
            outcome.fail(error);
            outcome.preparation = start.elapsed();
            return Some(outcome);
        }
        let mut candidate = (*self.accepted).clone();
        for (uri, text) in &pending {
            if let (Some(contracts), Some(before)) = (&self.build_contracts, self.accepted.get(uri)) {
                let check = contracts.fingerprint(before).and_then(|baseline| {
                    if baseline == contracts.fingerprint(text)? {
                        Ok(())
                    } else {
                        Err(ReloadError::RestartRequired(format!(
                            "Built handler or unresolved type contract changed: {uri}"
                        )))
                    }
                });
                if let Err(error) = check {
                    outcome.fail(error);
                    outcome.preparation = start.elapsed();
                    return Some(outcome);
                }
            }
            let check =
                DocumentContract::parse_with_build(text, self.build_contracts.as_deref()).and_then(|contract| match self.contracts.get(uri) {
                    Some(baseline) if baseline == &contract => Ok(()),
                    Some(_) => Err(ReloadError::RestartRequired(format!(
                        "XAML contract changed: {uri}"
                    ))),
                    None => Err(ReloadError::RestartRequired(format!(
                        "Unregistered document: {uri}"
                    ))),
                });
            if let Err(error) = check {
                outcome.fail(error);
                outcome.preparation = start.elapsed();
                return Some(outcome);
            }
            candidate.insert(uri.clone(), text.clone());
        }
        let candidate = Rc::new(candidate);
        self.targets.retain(|target| target.strong_count() != 0);
        let affected: BTreeSet<_> = pending.keys().cloned().collect();
        let prepared = with_sources(candidate.clone(), || {
            let (include_targets, _) = discovered?;
            let ambient_documents: Vec<_> = include_targets
                .iter()
                .flat_map(|target| target.documents())
                .collect();
            let validation = ferroui_markup_xaml::reload_validation::ValidationScope::new();
            let previews = ferroui_base::controls::resource_preview::ResourcePreviewScope::new();
            let mut prepared = Vec::new();
            let mut resources = Vec::new();
            let mut consumers = Vec::new();
            let mut covered = BTreeSet::new();
            for target in self.targets.iter().filter_map(Weak::upgrade).chain(include_targets) {
                let mut documents = target.documents();
                documents.extend(ambient_documents.iter().cloned());
                let dependencies = dependencies_of(documents, &candidate);
                if !dependencies.is_disjoint(&affected) {
                    if let Some(resource) =
                        catch_reload(|| target.clone().resource_preparation(candidate.clone()))?
                    {
                        resource.register(&previews)?;
                        resources.push(resource);
                    } else {
                        consumers.push(target);
                    }
                    covered.extend(dependencies);
                }
            }
            if !affected.is_subset(&covered) {
                return Err(ReloadError::RestartRequired(
                    "Changed document has no live validation adapter".into(),
                ));
            }
            // All factories are visible before any dictionary or view is built.
            for resource in resources {
                prepared.push(resource.take_change()?);
            }
            for target in consumers {
                prepared.push(catch_reload(|| target.prepare(candidate.clone()))?);
            }
            catch_reload(|| {
                validation
                    .validate()
                    .map_err(|error| ReloadError::Failed(error.to_string()))
            })?;
            Ok::<_, ReloadError>(prepared)
        });
        outcome.preparation = start.elapsed();
        let mut prepared = match prepared {
            Ok(prepared) => prepared,
            Err(error) => {
                outcome.fail(error);
                return Some(outcome);
            }
        };
        if self
            .watch_revision
            .as_ref()
            .is_some_and(|revision| !revision.is_current())
        {
            outcome.status = ReloadStatus::Superseded;
            outcome.diagnostic = Some("Sources changed while preparing the candidate view".into());
            return Some(outcome);
        }
        let start = Instant::now();
        let result = with_sources(candidate.clone(), || {
            for index in 0..prepared.len() {
                if let Err(error) = catch_reload(|| prepared[index].commit()) {
                    for previous in prepared[..=index].iter_mut().rev() {
                        previous.rollback();
                    }
                    return Err(error);
                }
            }
            if let Err(error) = catch_reload(finalize) {
                for change in prepared.iter_mut().rev() {
                    change.rollback();
                }
                return Err(error);
            }
            Ok(())
        });
        outcome.application = start.elapsed();
        match result {
            Err(error) => outcome.fail(error),
            Ok(()) => {
                self.pending.clear();
                self.accepted = candidate;
                *self.published.borrow_mut() = self.accepted.clone();
                self.applied_revision = self.revision;
                for change in &mut prepared {
                    change.finish();
                }
            }
        }
        Some(outcome)
    }
}
impl ReloadOutcome {
    fn fail(&mut self, error: ReloadError) {
        let (status, diagnostic) = match error {
            ReloadError::Failed(message) => (ReloadStatus::Failed, message),
            ReloadError::RestartRequired(message) => (ReloadStatus::RestartRequired, message),
        };
        self.status = status;
        self.diagnostic = Some(diagnostic);
    }
}
#[cfg(test)]
mod tests;

fn catch_reload<T>(action: impl FnOnce() -> Result<T, ReloadError>) -> Result<T, ReloadError> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(action)).unwrap_or_else(|payload| {
        if let Some(error) = payload.downcast_ref::<ReloadError>() {
            return Err(error.clone());
        }
        let message = payload
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| payload.downcast_ref::<&str>().map(|text| text.to_string()))
            .unwrap_or_else(|| "Reload callback panicked".into());
        Err(ReloadError::Failed(message))
    })
}

impl Drop for HotReloadSession {
    fn drop(&mut self) {
        for (class, installed) in &self.overrides {
            ferroui_markup_xaml::FerroXamlLoader::remove_populate_override(class, installed);
        }
    }
}

fn document_key(uri: &str) -> String {
    ferroui_base::utilities::Uri::try_create(uri, ferroui_base::utilities::UriKind::Absolute)
        .map(|uri| uri.absolute_uri().to_string())
        .unwrap_or_else(|| uri.to_string())
}

fn dependencies_of(roots: Vec<String>, sources: &Sources) -> BTreeSet<String> {
    use ferroui_base::utilities::{Uri, UriKind};
    let mut seen = BTreeSet::new();
    let mut pending = roots;
    while let Some(uri) = pending.pop() {
        let key = document_key(&uri);
        if !seen.insert(key.clone()) {
            continue;
        }
        let Some(source) = sources.get(&key) else {
            continue;
        };
        let (Ok(document), Some(base)) = (
            roxmltree::Document::parse(source),
            Uri::try_create(&key, UriKind::Absolute),
        ) else {
            continue;
        };
        for node in document.descendants().filter(|node| node.is_element()) {
            if !matches!(
                node.tag_name().name(),
                "StyleInclude" | "ResourceInclude" | "MergeResourceInclude"
            ) {
                continue;
            }
            if let Some(relative) = node
                .attribute("Source")
                .and_then(|source| Uri::try_create(source, UriKind::RelativeOrAbsolute))
            {
                let absolute = if relative.is_absolute_uri() {
                    relative
                } else {
                    Uri::combine(&base, &relative)
                };
                pending.push(absolute.absolute_uri().to_string());
            }
        }
    }
    seen
}
