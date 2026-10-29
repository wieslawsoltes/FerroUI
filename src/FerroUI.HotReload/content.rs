use crate::{PreparedReload, ReloadError, ReloadTarget};
use ferroui_base::utilities::{Uri, UriKind};
use ferroui_base::{BoxedValue, Ref, WeakRef};
use ferroui_controls::ContentControl;
use ferroui_markup_xaml::{
    source_overlay::Sources, RuntimeXamlLoaderConfiguration, RuntimeXamlLoaderDocument,
};
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use std::{cell::RefCell, rc::Rc};
/// Reloads a classless document inside a stable host. The host and its data context survive.
/// Custom `x:Class` roots need an application adapter that refreshes their cached references.
pub struct ContentReloadTarget {
    host: WeakRef<ContentControl>,
    uri: String,
    dependencies: Vec<String>,
    configuration: RuntimeXamlLoaderConfiguration,
    accepted_source: Rc<RefCell<String>>,
    state_adapters: Vec<Rc<dyn crate::ControlStateAdapter>>,
    restoration: crate::control_state::StateRestoration,
}
impl ContentReloadTarget {
    pub fn new(
        host: &Ref<ContentControl>,
        uri: String,
        dependencies: Vec<String>,
        configuration: RuntimeXamlLoaderConfiguration,
    ) -> Self {
        Self {
            host: host.downgrade(),
            uri: crate::document_key(&uri),
            dependencies: dependencies
                .iter()
                .map(|uri| crate::document_key(uri))
                .collect(),
            configuration,
            accepted_source: Rc::new(RefCell::new(String::new())),
            state_adapters: Vec::new(),
            restoration: Default::default(),
        }
    }
    /// Add application state transfer for uniquely identified, compatible controls.
    /// Restoration runs after a successful revision, alongside built-in state transfer.
    pub fn with_state_adapter(mut self, adapter: Rc<dyn crate::ControlStateAdapter>) -> Self {
        self.state_adapters.push(adapter);
        self
    }
}
struct ContentChange {
    restoration: crate::control_state::StateRestoration,
    host: Ref<ContentControl>,
    before: Option<BoxedValue>,
    after: Option<BoxedValue>,
    state: Rc<crate::state::ViewState>,
    old_source: String,
    new_source: String,
    accepted_source: Rc<RefCell<String>>,
    detached_context: Option<crate::detached_context::DetachedContext>,
}
impl PreparedReload for ContentChange {
    fn commit(&mut self) -> Result<(), ReloadError> {
        self.detached_context = crate::detached_context::DetachedContext::freeze(&self.before);
        self.host.set_content(self.after.clone());
        Ok(())
    }
    fn rollback(&mut self) {
        let _retired = crate::detached_context::DetachedContext::freeze(&self.after);
        self.host.set_content(self.before.clone());
        if let Some(context) = self.detached_context.take() {
            context.restore();
        }
    }
    fn finish(&mut self) {
        *self.accepted_source.borrow_mut() = self.new_source.clone();
        let host = self.host.downgrade();
        let root = self.after.clone();
        let state = self.state.clone();
        let before = self.old_source.clone();
        let after = self.new_source.clone();
        self.restoration.schedule(
            move || {
                if let (Some(host), Some(root)) = (host.upgrade(), root) {
                    if host.content().is_some_and(|current| Rc::ptr_eq(&current, &root)) {
                        host.update_layout();
                    }
                    if host
                        .content()
                        .is_some_and(|current| Rc::ptr_eq(&current, &root))
                    {
                        state.restore(&root, &before, &after);
                    }
                }
            },
        );
    }
}
impl ReloadTarget for ContentReloadTarget {
    fn complete_pending_state(&self) {
        self.restoration.complete();
    }
    fn documents(&self) -> Vec<String> {
        let mut all = self.dependencies.clone();
        all.push(self.uri.clone());
        all
    }
    fn prepare(&self, sources: Rc<Sources>) -> Result<Box<dyn PreparedReload>, ReloadError> {
        let host = self
            .host
            .upgrade()
            .ok_or_else(|| ReloadError::Failed("Reload host has closed".into()))?;
        let source = sources
            .get(&self.uri)
            .ok_or_else(|| ReloadError::Failed(format!("Missing source: {}", self.uri)))?;
        if crate::DocumentContract::parse(source)?.class.is_some() {
            return Err(ReloadError::RestartRequired(
                "A class root requires an explicit reload adapter".into(),
            ));
        }
        let uri = Uri::try_create(&self.uri, UriKind::Absolute)
            .ok_or_else(|| ReloadError::Failed("Document URI must be absolute".into()))?;
        let runtime_source = crate::identity::runtime_source(source)?;
        let document = RuntimeXamlLoaderDocument::with_base_uri(Some(uri), &runtime_source);
        let after =
            FerroRuntimeXamlLoader::load_document(document, Some(self.configuration.clone()))
                .map_err(|error| ReloadError::Failed(error.to_string()))?;
        Ok(Box::new(ContentChange {
            restoration: self.restoration.clone(),
            state: Rc::new(crate::state::ViewState::capture_document(
                host.content().as_ref(), Some(&self.uri),
                &self.accepted_source.borrow(),
                &self.state_adapters,
            )),
            old_source: self.accepted_source.borrow().clone(),
            new_source: source.clone(),
            accepted_source: self.accepted_source.clone(),
            detached_context: None,
            before: host.content(),
            host,
            after: Some(after),
        }))
    }
}
