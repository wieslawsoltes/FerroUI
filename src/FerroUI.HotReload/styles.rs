//! Ownership-aware replacement of an explicitly registered style contribution.
use crate::{PreparedReload, ReloadError, ReloadTarget};
use ferroui_base::{
    styling::{style_ptr_eq, IStyle, Styles},
    Ref, WeakRef,
};
use ferroui_markup_xaml::{
    source_overlay::Sources, FromXamlObject, RuntimeXamlLoaderConfiguration,
    RuntimeXamlLoaderDocument,
};
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

/// Replaces one style or nested Styles contribution, retaining its container and
/// application-owned siblings. Native style removal detaches the previous frames;
/// addition invalidates styling for the owner and its logical descendants.
/// Register static-resource consumers as dependencies of this document.
/// Applications caching the replaced style need an explicit refresh adapter.
pub struct StyleReloadTarget {
    container: WeakRef<Styles>,
    current: Rc<RefCell<Weak<dyn IStyle>>>,
    uri: String,
    dependencies: Vec<String>,
    configuration: RuntimeXamlLoaderConfiguration,
}
impl StyleReloadTarget {
    /// Declares resources/styles outside the document's inline includes, such as
    /// application resources consumed through StaticResource. Their revisions are
    /// prepared and committed in the same transaction as this target.
    pub fn with_dependencies(mut self, dependencies: impl IntoIterator<Item = String>) -> Self {
        self.dependencies = dependencies
            .into_iter()
            .map(|uri| crate::document_key(&uri))
            .collect();
        self
    }

    pub fn contribution(
        container: &Ref<Styles>,
        index: usize,
        uri: String,
        configuration: RuntimeXamlLoaderConfiguration,
    ) -> Result<Self, ReloadError> {
        let provider = container
            .snapshot()
            .get(index)
            .cloned()
            .ok_or_else(|| ReloadError::Failed("Invalid style contribution index".into()))?;
        Ok(Self {
            container: container.downgrade(),
            current: Rc::new(RefCell::new(Rc::downgrade(&provider))),
            uri: crate::document_key(&uri),
            dependencies: Vec::new(),
            configuration,
        })
    }
    /// Discovers direct style includes without evaluating their markup.
    pub fn discover(
        container: &Ref<Styles>,
        configuration: RuntimeXamlLoaderConfiguration,
    ) -> Result<Vec<Self>, ReloadError> {
        container
            .snapshot()
            .iter()
            .enumerate()
            .filter_map(|(index, style)| {
                style
                    .as_resource_provider()?
                    .development_source()
                    .map(|uri| (index, uri))
            })
            .map(|(index, uri)| Self::contribution(container, index, uri, configuration.clone()))
            .collect()
    }

    pub fn is_alive(&self) -> bool {
        self.container.upgrade().is_some()
    }
    pub(crate) fn tracks(&self, style: &Rc<dyn IStyle>) -> bool {
        self.current
            .borrow()
            .upgrade()
            .is_some_and(|current| style_ptr_eq(&current, style))
    }
    pub(crate) fn is_attached(&self) -> bool {
        self.container.upgrade().is_some_and(|container| {
            container.snapshot().iter().any(|style| self.tracks(style))
        })
    }
}
struct Change {
    container: Ref<Styles>,
    index: usize,
    old: Rc<dyn IStyle>,
    new: Rc<dyn IStyle>,
    current: Rc<RefCell<Weak<dyn IStyle>>>,
    started: bool,
}
impl PreparedReload for Change {
    fn commit(&mut self) -> Result<(), ReloadError> {
        let entries = self.container.snapshot();
        if entries
            .get(self.index)
            .is_none_or(|entry| !style_ptr_eq(entry, &self.old))
        {
            return Err(ReloadError::RestartRequired(
                "Owned style contribution changed after preparation".into(),
            ));
        }
        self.started = true;
        // Replace transfers resource ownership and detaches/invalidates style frames.
        self.container.set(self.index, self.new.clone());
        Ok(())
    }
    fn rollback(&mut self) {
        if self.started {
            // Styles::set removes before inserting. An observer may panic after
            // removal, so rollback must not overwrite the next application style.
            let entries = self.container.snapshot();
            if let Some(index) = entries
                .iter()
                .position(|entry| style_ptr_eq(entry, &self.new))
            {
                self.container.set(index, self.old.clone());
            } else if !entries.iter().any(|entry| style_ptr_eq(entry, &self.old)) {
                self.container
                    .insert(self.index.min(entries.len()), self.old.clone());
            }
        }
    }
    fn finish(&mut self) {
        *self.current.borrow_mut() = Rc::downgrade(&self.new);
    }
}
impl ReloadTarget for StyleReloadTarget {
    fn documents(&self) -> Vec<String> {
        let mut documents = self.dependencies.clone();
        documents.push(self.uri.clone());
        documents
    }
    fn prepare(&self, sources: Rc<Sources>) -> Result<Box<dyn PreparedReload>, ReloadError> {
        self.prepare_change(sources)
            .map(|change| Box::new(change) as Box<dyn PreparedReload>)
    }
    fn resource_preparation(
        self: Rc<Self>,
        sources: Rc<Sources>,
    ) -> Result<Option<crate::ResourcePreparation>, ReloadError> {
        let old = self.current.borrow().upgrade().ok_or_else(|| {
            ReloadError::RestartRequired("Owned resource/style contribution was removed".into())
        })?;
        let original: Rc<dyn ferroui_base::controls::IResourceNode> = old;
        Ok(Some(crate::ResourcePreparation::new(original, move || {
            let change = self.prepare_change(sources.clone())?;
            let replacement: Rc<dyn ferroui_base::controls::IResourceNode> = change.new.clone();
            Ok((replacement, Box::new(change)))
        })))
    }
}
impl StyleReloadTarget {
    fn prepare_change(&self, sources: Rc<Sources>) -> Result<Change, ReloadError> {
        let container = self
            .container
            .upgrade()
            .ok_or_else(|| ReloadError::Failed("Style container closed".into()))?;
        let old = self.current.borrow().upgrade().ok_or_else(|| {
            ReloadError::RestartRequired("Owned style contribution was removed".into())
        })?;
        let entries = container.snapshot();
        let mut matches = entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| style_ptr_eq(entry, &old));
        let index = matches.next().map(|(index, _)| index).ok_or_else(|| {
            ReloadError::RestartRequired("Owned style contribution was replaced".into())
        })?;
        if matches.next().is_some() {
            return Err(ReloadError::RestartRequired(
                "Ambiguous style contribution ownership".into(),
            ));
        }
        let source = sources
            .get(&self.uri)
            .ok_or_else(|| ReloadError::Failed("Missing style source".into()))?;
        let xml = roxmltree::Document::parse(source)
            .map_err(|error| ReloadError::Failed(error.to_string()))?;
        let root = xml.root_element();
        if !matches!(root.tag_name().name(), "Styles" | "Style")
            || root.tag_name().namespace() != Some("https://github.com/ferroui")
            || crate::DocumentContract::parse(source)?.class.is_some()
        {
            return Err(ReloadError::RestartRequired(
                "Custom style roots require an explicit adapter".into(),
            ));
        }
        let uri = ferroui_base::utilities::Uri::try_create(
            &self.uri,
            ferroui_base::utilities::UriKind::Absolute,
        )
        .ok_or_else(|| ReloadError::Failed("Invalid style URI".into()))?;
        let candidate = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load_document(
            RuntimeXamlLoaderDocument::with_base_uri(Some(uri), source),
            Some(self.configuration.clone()),
        )
        .map_err(|error| ReloadError::Failed(error.to_string()))?;
        let new = Rc::<dyn IStyle>::from_xaml_object(&candidate)
            .ok_or_else(|| ReloadError::Failed("Expected style".into()))?;
        Ok(Change {
            container,
            index,
            old,
            new,
            current: self.current.clone(),
            started: false,
        })
    }
}
