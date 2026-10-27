//! Application-owned design hosts, real class instances and native top-level previews.
#[cfg(debug_assertions)]
use ferroui_base::WeakRef;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::utilities::Uri;
use ferroui_base::{BoxedValue, FerroObject, Ref};
#[cfg(debug_assertions)]
use ferroui_controls::ContentControl;
use ferroui_controls::{Control, Design, PreviewTarget, Window};
use ferroui_designer_core::Document;
use ferroui_markup_xaml::{
    FromXamlObject, RuntimeXamlLoaderConfiguration, RuntimeXamlLoaderDocument,
};
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use std::collections::BTreeMap;
use std::rc::Rc;

pub type PreviewFactory = Rc<dyn Fn(&PreviewRequest) -> Result<BoxedValue, String>>;
pub type ResourcePreviewFactory = Rc<dyn Fn(BoxedValue) -> Result<Ref<Control>, String>>;

/// Preview construction is scoped inside an ordinary host application. Nested loads
/// and failing factories restore the surrounding framework design-mode flag.
pub(crate) struct DesignModeGuard(bool);
impl DesignModeGuard {
    pub(crate) fn new(enabled: bool) -> Self {
        let previous = Design::is_design_mode();
        Design::set_is_design_mode(enabled);
        Self(previous)
    }
}
impl Drop for DesignModeGuard {
    fn drop(&mut self) {
        Design::set_is_design_mode(self.0);
    }
}

/// A factory receives the actual authored document and base URI. It may instantiate a real
/// registered code-behind class and call `populate` to apply XAML before wiring its handlers.
pub struct PreviewRequest {
    pub source: String,
    /// Designer-only loading source. Factories continue to inspect the exact authored
    /// `source`; `populate` loads this projection when design-time overrides are present.
    pub preview_source: Option<String>,
    pub uri: String,
    pub configuration: RuntimeXamlLoaderConfiguration,
}
impl PreviewRequest {
    pub fn populate(&self, root: Option<BoxedValue>) -> Result<BoxedValue, String> {
        let _design_mode = DesignModeGuard::new(self.configuration.design_mode);
        let mut source = self.preview_source.as_ref().unwrap_or(&self.source).clone();
        if root.is_some() {
            // The supplied root IS the code-behind class. x:Class is a compiler directive,
            // so leave its source coordinates intact while the runtime populates that instance.
            let document = Document::new(&source).map_err(|e| e.to_string())?;
            let element = document
                .ast()
                .node(document.ast().root())
                .ok_or("No root")?;
            let spans = element
                .attributes
                .iter()
                .filter(|a| {
                    a.namespace_uri.as_deref()
                        == Some("http://schemas.microsoft.com/winfx/2006/xaml")
                        && a.name.rsplit(':').next() == Some("Class")
                })
                .map(|a| a.span)
                .collect::<Vec<_>>();
            for span in spans.into_iter().rev() {
                let blank = source[span.start..span.end]
                    .chars()
                    .map(|c| {
                        if matches!(c, '\r' | '\n' | '\t') {
                            c
                        } else {
                            ' '
                        }
                    })
                    .collect::<String>();
                source.replace_range(span.start..span.end, &blank);
            }
        }
        let mut input = RuntimeXamlLoaderDocument::with_base_uri_and_root_instance(
            Some(Uri::absolute(&self.uri).map_err(|e| e.to_string())?),
            root,
            &source,
        );
        input.document = Some(self.uri.clone());
        FerroRuntimeXamlLoader::load_document(input, Some(self.configuration.clone()))
            .map_err(|e| format!("{}: {e}", self.uri))
    }
}

#[derive(Clone)]
pub struct PreviewHostOptions {
    /// Runtime metadata registration (custom controls, converters, attached properties).
    pub register_types: Vec<Rc<dyn Fn()>>,
    /// Keys may be x:Class names or namespace-qualified root type names.
    pub root_factories: BTreeMap<String, PreviewFactory>,
    /// Resource dictionaries, application resources, styles and custom native Window roots
    /// can use a host-chosen artboard view. The actual Window remains available in native preview.
    pub resource_factory: Option<ResourcePreviewFactory>,
    pub configuration: RuntimeXamlLoaderConfiguration,
    /// Show authored d: values and sample data in this designer host. This option is
    /// independent from the normal runtime loader and never changes saved XAML.
    pub design_time_data: bool,
}
impl Default for PreviewHostOptions {
    fn default() -> Self {
        let mut configuration = RuntimeXamlLoaderConfiguration::new();
        configuration.design_mode = true;
        configuration.set_create_source_info(true);
        Self {
            register_types: vec![],
            root_factories: BTreeMap::new(),
            resource_factory: None,
            configuration,
            design_time_data: true,
        }
    }
}
impl PreviewHostOptions {
    pub fn register_root(&mut self, key: impl Into<String>, factory: PreviewFactory) {
        self.root_factories.insert(key.into(), factory);
    }
    pub(crate) fn factory(&self, document: &Document) -> Option<&PreviewFactory> {
        let root = document.ast().node(document.ast().root())?;
        root.attributes
            .iter()
            .find(|a| {
                a.namespace_uri.as_deref() == Some("http://schemas.microsoft.com/winfx/2006/xaml")
                    && a.name.rsplit(':').next() == Some("Class")
            })
            .and_then(|a| self.root_factories.get(&a.value))
            .or_else(|| self.root_factories.get(&root.name))
            .or_else(|| {
                self.root_factories.get(&format!(
                    "{{{}}}{}",
                    root.namespace_uri.as_deref().unwrap_or(""),
                    root.local_name()
                ))
            })
    }
    pub(crate) fn load(&self, source: &str, uri: &str) -> Result<BoxedValue, String> {
        let _design_mode = DesignModeGuard::new(self.configuration.design_mode);
        let document = Document::new(source).map_err(|e| e.to_string())?;
        let request = PreviewRequest {
            source: source.into(),
            preview_source: crate::design_time::project(
                &document,
                source,
                self.design_time_data && self.configuration.design_mode,
            )?
            .map(|projection| projection.document.source().to_owned()),
            uri: uri.into(),
            configuration: self.configuration.clone(),
        };
        if let Some(factory) = self.factory(&document) {
            factory(&request)
        } else {
            if let Some(class) = document.ast().node(document.ast().root()).and_then(|root| {
                root.attributes.iter().find(|attribute| {
                    attribute.namespace_uri.as_deref()
                        == Some("http://schemas.microsoft.com/winfx/2006/xaml")
                        && attribute.name.rsplit(':').next() == Some("Class")
                })
            }) {
                return Err(format!(
                    "{}: register a preview root factory for code-behind class {}",
                    uri, class.value
                ));
            }
            request.populate(None)
        }
    }
    pub(crate) fn control(&self, source: &str, uri: &str) -> Result<Ref<Control>, String> {
        let loaded = self.load(source, uri)?;
        if let Some(control) = Ref::<Control>::from_xaml_object(&loaded) {
            if control.cast::<Window>().is_some() {
                if let Some(factory) = &self.resource_factory {
                    return factory(loaded);
                }
                return Err("A Window requires the native preview host".into());
            }
            return Ok(control);
        }
        if let Some(factory) = &self.resource_factory {
            return factory(loaded);
        }
        let value = Some(loaded.clone());
        let target = if let Some(object) = from_markup_value::<Ref<FerroObject>>(&value) {
            PreviewTarget::Object(object)
        } else if let Some(style) =
            from_markup_value::<Rc<dyn ferroui_base::styling::IStyle>>(&value)
        {
            PreviewTarget::Style(style)
        } else if let Some(template) =
            from_markup_value::<Rc<dyn ferroui_controls::templates::IDataTemplate>>(&value)
        {
            PreviewTarget::DataTemplate(template)
        } else {
            PreviewTarget::Other
        };
        Design::create_preview_with_control(&target)
            .ok_or_else(|| "Configure a resource preview factory for this document".into())
    }
}

impl crate::DesignerSession {
    pub fn configure_preview(&mut self, options: PreviewHostOptions) -> Result<(), String> {
        self.animation.native.stop();
        self.preview.clear_animation();
        self.animation.player.pause();
        self.preview.configure(options, &self.document)?;
        self.forget_interaction_preview();
        self.import_animations();
        self.generation += 1;
        self.refresh_layer_visuals();
        Ok(())
    }
    /// Opens the actual platform Window, including native decorations and registered class
    /// behavior. A non-Window document is hosted inside a normal platform Window.
    pub fn open_native_preview(&mut self) -> Result<Ref<Window>, String> {
        let source = self.document().valid_source().to_owned();
        self.preview.open_native(source)
    }
    pub fn close_native_preview(&mut self) {
        self.preview.close_native();
    }
}

#[cfg(debug_assertions)]
pub(crate) struct FactoryReloadTarget {
    pub host: WeakRef<ContentControl>,
    pub uri: String,
    pub use_factory: Rc<std::cell::Cell<bool>>,
    configuration: std::cell::RefCell<FactoryTargetConfiguration>,
}
#[cfg(debug_assertions)]
pub(crate) struct FactoryTargetConfiguration {
    options: PreviewHostOptions,
    fallback: Rc<dyn ferroui_hot_reload::ReloadTarget>,
}
#[cfg(debug_assertions)]
impl FactoryReloadTarget {
    pub(crate) fn new(
        host: &Ref<ContentControl>,
        uri: &str,
        options: PreviewHostOptions,
        use_factory: Rc<std::cell::Cell<bool>>,
    ) -> Result<Self, String> {
        let uri = Uri::absolute(uri)
            .map_err(|error| error.to_string())?
            .absolute_uri()
            .to_owned();
        let configuration = Self::configuration(host, &uri, options);
        Ok(Self {
            host: host.downgrade(),
            uri,
            use_factory,
            configuration: std::cell::RefCell::new(configuration),
        })
    }
    fn configuration(
        host: &Ref<ContentControl>,
        uri: &str,
        options: PreviewHostOptions,
    ) -> FactoryTargetConfiguration {
        let fallback = Rc::new(ferroui_hot_reload::ContentReloadTarget::new(
            host,
            uri.into(),
            Vec::new(),
            options.configuration.clone(),
        ));
        FactoryTargetConfiguration { options, fallback }
    }
    pub(crate) fn replace_configuration(
        &self,
        host: &Ref<ContentControl>,
        options: PreviewHostOptions,
    ) -> FactoryTargetConfiguration {
        self.configuration
            .replace(Self::configuration(host, &self.uri, options))
    }
    pub(crate) fn restore_configuration(&self, configuration: FactoryTargetConfiguration) {
        self.configuration.replace(configuration);
    }
}
#[cfg(debug_assertions)]
impl ferroui_hot_reload::ReloadTarget for FactoryReloadTarget {
    fn complete_pending_state(&self) {
        self.configuration
            .borrow()
            .fallback
            .complete_pending_state();
    }
    fn documents(&self) -> Vec<String> {
        vec![self.uri.clone()]
    }
    fn prepare(
        &self,
        sources: Rc<ferroui_markup_xaml::source_overlay::Sources>,
    ) -> Result<Box<dyn ferroui_hot_reload::PreparedReload>, ferroui_hot_reload::ReloadError> {
        use ferroui_hot_reload::ReloadError;
        let configuration = self.configuration.borrow();
        let fallback = configuration.fallback.clone();
        let options = configuration.options.clone();
        drop(configuration);
        if !self.use_factory.get() {
            return fallback.prepare(sources);
        }
        let host = self
            .host
            .upgrade()
            .ok_or_else(|| ReloadError::Failed("Preview host closed".into()))?;
        let source = sources
            .get(&self.uri)
            .ok_or_else(|| ReloadError::Failed("Missing preview document".into()))?;
        let candidate = options
            .control(source, &self.uri)
            .map_err(ReloadError::Failed)?;
        Ok(Box::new(FactoryChange {
            before: host.content(),
            host,
            after: Control::boxed(candidate),
        }))
    }
}
#[cfg(debug_assertions)]
struct FactoryChange {
    host: Ref<ContentControl>,
    before: Option<BoxedValue>,
    after: BoxedValue,
}
#[cfg(debug_assertions)]
impl ferroui_hot_reload::PreparedReload for FactoryChange {
    fn commit(&mut self) -> Result<(), ferroui_hot_reload::ReloadError> {
        self.host.set_content(Some(self.after.clone()));
        Ok(())
    }
    fn rollback(&mut self) {
        self.host.set_content(self.before.clone());
    }
}
