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

pub(crate) struct StyleSlot {
    container: WeakRef<Styles>,
    current: Rc<RefCell<Weak<dyn IStyle>>>,
}
impl StyleSlot {
    pub(crate) fn new(container: &Ref<Styles>, index: usize) -> Result<Self, ReloadError> {
        let provider = container
            .snapshot()
            .get(index)
            .cloned()
            .ok_or_else(|| ReloadError::Failed("Invalid style contribution index".into()))?;
        Ok(Self {
            container: container.downgrade(),
            current: Rc::new(RefCell::new(Rc::downgrade(&provider))),
        })
    }
}
/// Replaces one style or nested Styles contribution, retaining its container and
/// application-owned siblings. Native style removal detaches the previous frames;
/// addition invalidates styling for the owner and its logical descendants.
/// Register static-resource consumers as dependencies of this document.
/// Applications caching the replaced style need an explicit refresh adapter.
pub struct StyleReloadTarget {
    slot: StyleSlot,
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
        Ok(Self {
            slot: StyleSlot::new(container, index)?,
            uri: crate::document_key(&uri),
            dependencies: Vec::new(),
            configuration,
        })
    }
    /// Replace this mounted contribution with a complete FXD document on the UI thread.
    /// Editors may deliver bytes after an atomic file rename or a transport message.
    /// Resolve includes against the registered URI and commit only a fully built style.
    #[cfg(feature = "portable-xaml")]
    pub fn replace_portable(&self, bytes: Vec<u8>) -> Result<(), ReloadError> {
        if !ferroui_markup_xaml_loader::binary_document::is_binary(&bytes) {
            return Err(ReloadError::Failed("Expected an FXD document".into()));
        }
        self.slot.replace_if_current(|| false, || {
            let uri = ferroui_base::utilities::Uri::absolute(&self.uri)
                .map_err(|error| ReloadError::Failed(error.to_string()))?;
            let document = RuntimeXamlLoaderDocument::from_stream_with_base_uri(Some(uri), Box::new(std::io::Cursor::new(bytes)));
            let candidate = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load_document(document, Some(self.configuration.clone()))
                .map_err(|error| ReloadError::Failed(error.to_string()))?;
            Rc::<dyn IStyle>::from_xaml_object(&candidate).ok_or_else(|| ReloadError::Failed("Expected style".into()))
        })?;
        Ok(())
    }

    /// Replace this contribution from a complete FXP revision. All member graphs
    /// must validate, including any outside this style's include closure. Relative
    /// includes resolve in the candidate archive during preparation.
    #[cfg(feature = "portable-xaml")]
    pub fn replace_packed(&self, bytes: &[u8]) -> Result<(), ReloadError> {
        use ferroui_markup_xaml_loader::{binary_document::pack::ThemePack, FerroRuntimeXamlLoader};
        self.slot.replace_if_current(|| false, || {
            let archive = ThemePack::open(bytes)
                .map_err(|error| ReloadError::Failed(error.to_string()))?;
            archive.validate().map_err(|error| ReloadError::Failed(error.to_string()))?;
            let uri = ferroui_base::utilities::Uri::absolute(&self.uri)
                .map_err(|error| ReloadError::Failed(error.to_string()))?;
            let candidate = FerroRuntimeXamlLoader::load_packed_document(
                &archive, uri, None, None, self.configuration.clone(),
            ).map_err(|error| ReloadError::Failed(error.to_string()))?;
            Rc::<dyn IStyle>::from_xaml_object(&candidate)
                .ok_or_else(|| ReloadError::Failed("Expected style".into()))
        })?;
        Ok(())
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
        self.slot.container.upgrade().is_some()
    }
    pub(crate) fn tracks(&self, style: &Rc<dyn IStyle>) -> bool {
        self.slot
            .current
            .borrow()
            .upgrade()
            .is_some_and(|current| style_ptr_eq(&current, style))
    }
    pub(crate) fn is_attached(&self) -> bool {
        self.slot
            .container
            .upgrade()
            .is_some_and(|container| container.snapshot().iter().any(|style| self.tracks(style)))
    }
}
pub(crate) struct Change {
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
        let old = self.slot.current.borrow().upgrade().ok_or_else(|| {
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
        self.slot.prepare(|| self.load(sources))
    }
    fn load(&self, sources: Rc<Sources>) -> Result<Rc<dyn IStyle>, ReloadError> {
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
        Rc::<dyn IStyle>::from_xaml_object(&candidate)
            .ok_or_else(|| ReloadError::Failed("Expected style".into()))
    }
}
impl StyleSlot {
    #[cfg(any(feature = "binary-resources", feature = "portable-xaml"))]
    pub(crate) fn replace_if_current(
        &self,
        stale: impl Fn() -> bool,
        build: impl FnOnce() -> Result<Rc<dyn IStyle>, ReloadError>,
    ) -> Result<bool, ReloadError> {
        let mut change = crate::catch_reload(|| self.prepare(build))?;
        if stale() {
            return Ok(false);
        }
        if let Err(err) = crate::catch_reload(|| change.commit()) {
            crate::catch_reload(|| {
                change.rollback();
                Ok(())
            })?;
            return Err(err);
        }
        if stale() {
            crate::catch_reload(|| {
                change.rollback();
                Ok(())
            })?;
            return Ok(false);
        }
        change.finish();
        Ok(true)
    }

    pub(crate) fn prepare(
        &self,
        build: impl FnOnce() -> Result<Rc<dyn IStyle>, ReloadError>,
    ) -> Result<Change, ReloadError> {
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
        let new = build()?;
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

#[cfg(all(test, feature = "portable-xaml"))]
mod portable_tests {
    use super::*;
    use ferroui_base::threading::Dispatcher;
    use ferroui_markup_xaml_loader::binary_document;

    #[test]
    fn changing_the_style_graph_replaces_one_slot_and_an_invalid_image_keeps_it() {
        let _scope = Dispatcher::unit_test_scope();
        ferroui_markup_xaml::register_types();
        let container = Styles::new();
        let initial: Rc<dyn IStyle> = Styles::new().into();
        let sibling: Rc<dyn IStyle> = Styles::new().into();
        container.add(initial.clone());
        container.add(sibling.clone());
        let target = StyleReloadTarget::contribution(&container, 0, "ferres://Editor/Main.xaml".into(), Default::default()).unwrap();
        let revision = binary_document::compile(r#"<Styles xmlns="https://github.com/ferroui"><Style Selector="Button"/></Styles>"#).unwrap();
        target.replace_portable(revision).unwrap();
        let applied = container.get(0);
        assert!(!style_ptr_eq(&applied, &initial));
        assert!(target.replace_portable(b"FXD\0bad".to_vec()).is_err());
        assert!(style_ptr_eq(&applied, &container.get(0)));
        assert!(style_ptr_eq(&sibling, &container.get(1)));
    }

    #[test]
    fn packed_style_reloads_transitive_includes_and_rejects_an_invalid_unrelated_member() {
        let _scope = Dispatcher::unit_test_scope();
        ferroui_markup_xaml::register_types();
        let container = Styles::new();
        let initial: Rc<dyn IStyle> = Styles::new().into();
        let sibling: Rc<dyn IStyle> = Styles::new().into();
        container.add(initial.clone());
        container.add(sibling.clone());
        let target = StyleReloadTarget::contribution(
            &container, 0, "ferres://Editor/Views/Main.xaml".into(), Default::default(),
        ).unwrap();
        let revision = binary_document::pack::compile_documents(&[
            ("Views/Main.xaml", r#"<Styles xmlns="https://github.com/ferroui"><StyleInclude Source="../Parts/Child.xaml"/></Styles>"#),
            ("Parts/Child.xaml", r#"<Styles xmlns="https://github.com/ferroui"><Style Selector="Button"/></Styles>"#),
            ("Z/Unrelated.xaml", r#"<Styles xmlns="https://github.com/ferroui"/>"#),
        ]).unwrap();
        // An editor may still have XML for the preceding generation in scope.
        // Members present in the new pack must win as a coherent revision.
        let stale = [(crate::document_key("ferres://Editor/Parts/Child.xaml"), "invalid xml".to_owned())]
            .into_iter().collect();
        ferroui_markup_xaml::source_overlay::with_sources(Rc::new(stale), || {
            target.replace_packed(&revision).unwrap();
        });
        let accepted = container.get(0);
        assert!(!style_ptr_eq(&initial, &accepted));
        let mut corrupt = revision;
        *corrupt.last_mut().unwrap() = 0xff;
        assert!(target.replace_packed(&corrupt).is_err());
        assert!(style_ptr_eq(&accepted, &container.get(0)));
        assert!(style_ptr_eq(&sibling, &container.get(1)));
    }
}
