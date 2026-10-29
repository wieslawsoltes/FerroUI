//! Ownership-aware replacement of a merged resource-dictionary contribution.
use crate::{PreparedReload, ReloadError, ReloadTarget};
use ferroui_base::{
    controls::{resource_provider_ptr_eq, IResourceProvider, ResourceDictionary},
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

/// Replaces one explicitly owned merged dictionary, preserving its container,
/// direct application resources and all other merged dictionaries. Register
/// static-resource consumers with this document as a dependency so the session
/// prepares their replacements in the same revision. Dynamic-resource consumers
/// receive the resource provider's ordinary owner notifications.
///
/// The replaced provider itself is not a stable application object: applications
/// caching it or subscribing to it directly need an explicit refresh adapter.
pub struct ResourceReloadTarget {
    container: WeakRef<ResourceDictionary>,
    current: Rc<RefCell<Weak<dyn IResourceProvider>>>,
    uri: String,
    dependencies: Vec<String>,
    configuration: RuntimeXamlLoaderConfiguration,
}
impl ResourceReloadTarget {
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

    pub fn merged(
        container: &Ref<ResourceDictionary>,
        index: usize,
        uri: String,
        configuration: RuntimeXamlLoaderConfiguration,
    ) -> Result<Self, ReloadError> {
        let provider = container
            .merged_dictionaries()
            .to_vec()
            .get(index)
            .cloned()
            .ok_or_else(|| ReloadError::Failed("Invalid merged-resource index".into()))?;
        Ok(Self {
            container: container.downgrade(),
            current: Rc::new(RefCell::new(Rc::downgrade(&provider))),
            uri: crate::document_key(&uri),
            dependencies: Vec::new(),
            configuration,
        })
    }
    /// Discovers direct merged include contributions without loading their source.
    /// Keep the returned adapters alive and register their documents as dependencies
    /// of static consumers. Nested includes are covered by source dependency traversal.
    pub fn discover_merged(
        container: &Ref<ResourceDictionary>,
        configuration: RuntimeXamlLoaderConfiguration,
    ) -> Result<Vec<Self>, ReloadError> {
        container
            .merged_dictionaries()
            .to_vec()
            .iter()
            .enumerate()
            .filter_map(|(index, provider)| provider.development_source().map(|uri| (index, uri)))
            .map(|(index, uri)| Self::merged(container, index, uri, configuration.clone()))
            .collect()
    }

    pub fn is_alive(&self) -> bool {
        self.container.upgrade().is_some()
    }
    pub(crate) fn tracks(&self, provider: &Rc<dyn IResourceProvider>) -> bool {
        self.current
            .borrow()
            .upgrade()
            .is_some_and(|current| resource_provider_ptr_eq(&current, provider))
    }
    pub(crate) fn is_attached(&self) -> bool {
        self.container.upgrade().is_some_and(|container| {
            container.merged_dictionaries().to_vec().iter().any(|provider| self.tracks(provider))
        })
    }
}
struct Change {
    container: Ref<ResourceDictionary>,
    index: usize,
    old: Rc<dyn IResourceProvider>,
    new: Rc<dyn IResourceProvider>,
    current: Rc<RefCell<Weak<dyn IResourceProvider>>>,
    started: bool,
}
impl PreparedReload for Change {
    fn commit(&mut self) -> Result<(), ReloadError> {
        let entries = self.container.merged_dictionaries().to_vec();
        if entries
            .get(self.index)
            .is_none_or(|entry| !resource_provider_ptr_eq(entry, &self.old))
        {
            return Err(ReloadError::RestartRequired(
                "Owned resource contribution changed after preparation".into(),
            ));
        }
        self.started = true;
        // The collection transfers ownership through RemoveOwner/AddOwner and
        // the dictionaries raise hosted-resource notifications, including themes.
        self.container
            .merged_dictionaries()
            .set(self.index, self.new.clone());
        Ok(())
    }
    fn rollback(&mut self) {
        if self.started {
            self.container
                .merged_dictionaries()
                .set(self.index, self.old.clone());
        }
    }
    fn finish(&mut self) {
        *self.current.borrow_mut() = Rc::downgrade(&self.new);
    }
}
impl ReloadTarget for ResourceReloadTarget {
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
impl ResourceReloadTarget {
    fn prepare_change(&self, sources: Rc<Sources>) -> Result<Change, ReloadError> {
        let container = self
            .container
            .upgrade()
            .ok_or_else(|| ReloadError::Failed("Resource container closed".into()))?;
        let old = self.current.borrow().upgrade().ok_or_else(|| {
            ReloadError::RestartRequired("Owned resource contribution was removed".into())
        })?;
        let entries = container.merged_dictionaries().to_vec();
        let mut matches = entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| resource_provider_ptr_eq(entry, &old));
        let index = matches.next().map(|(index, _)| index).ok_or_else(|| {
            ReloadError::RestartRequired("Owned resource contribution was replaced".into())
        })?;
        if matches.next().is_some() {
            return Err(ReloadError::RestartRequired(
                "Ambiguous resource contribution ownership".into(),
            ));
        }
        let source = sources
            .get(&self.uri)
            .ok_or_else(|| ReloadError::Failed("Missing resource source".into()))?;
        let xml = roxmltree::Document::parse(source)
            .map_err(|error| ReloadError::Failed(error.to_string()))?;
        let root = xml.root_element();
        if root.tag_name().name() != "ResourceDictionary"
            || root.tag_name().namespace() != Some("https://github.com/ferroui")
            || crate::DocumentContract::parse(source)?.class.is_some()
        {
            return Err(ReloadError::RestartRequired(
                "Custom resource roots require an explicit adapter".into(),
            ));
        }
        let uri = ferroui_base::utilities::Uri::try_create(
            &self.uri,
            ferroui_base::utilities::UriKind::Absolute,
        )
        .ok_or_else(|| ReloadError::Failed("Invalid resource URI".into()))?;
        let candidate = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load_document(
            RuntimeXamlLoaderDocument::with_base_uri(Some(uri), source),
            Some(self.configuration.clone()),
        )
        .map_err(|error| ReloadError::Failed(error.to_string()))?;
        let dictionary = Ref::<ResourceDictionary>::from_xaml_object(&candidate)
            .ok_or_else(|| ReloadError::Failed("Expected resource dictionary".into()))?;
        let new: Rc<dyn IResourceProvider> = dictionary.into();
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
