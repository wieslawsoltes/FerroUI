//! Port of `FerroXamlLoader.cs`: loads markup documents by URI.

use crate::{RuntimeXamlLoaderConfiguration, RuntimeXamlLoaderDocument, XamlLoadException};
use ferroui_base::metadata::{IServiceProvider, MarkupAssembly};
use ferroui_base::platform::IAssetLoader;
use ferroui_base::utilities::Uri;
use ferroui_base::{ferro_markup_type, BoxedValue, FerroLocator, LocatorExtensions};
use std::collections::HashMap;
use std::io::Read;
use std::rc::Rc;
use std::sync::{OnceLock, RwLock};

/// Loads documents from their text at run time. Registered as a service
/// (`FerroLocator`) by the runtime XAML loader crate; without it only
/// compiled documents can be loaded.
pub trait IRuntimeXamlLoader {
    fn load(
        &self,
        document: RuntimeXamlLoaderDocument,
        configuration: RuntimeXamlLoaderConfiguration,
    ) -> Result<BoxedValue, XamlLoadException>;

    /// Populates `instance` (an object whose class names a document with
    /// `x:Class`) from the markup of its class, when no compiled markup is
    /// registered for it. A loader that cannot find markup for a class
    /// keeps the default: the error of the managed original for an object
    /// without compiled markup.
    fn load_object(
        &self,
        instance: &BoxedValue,
        service_provider: Option<&Rc<dyn IServiceProvider>>,
    ) -> Result<(), XamlLoadException> {
        let _ = service_provider;
        Err(FerroXamlLoader::no_precompiled_xaml_for_object(instance))
    }
}

/// The loader of the compiled documents of one assembly (crate): returns
/// the object built from the document with the absolute URI `uri`,
/// `Ok(None)` if the assembly has no such document, and the load error when
/// building the document failed.
///
/// This is what the XAML compiler generates per crate; it stands in for
/// the generated `TryLoad(IServiceProvider, string)` method the managed
/// original finds by reflection (whose `null` is `Ok(None)` and whose
/// exception is the error).
pub type CompiledXamlLoader =
    fn(service_provider: Option<&Rc<dyn IServiceProvider>>, uri: &str) -> Result<Option<BoxedValue>, XamlLoadException>;

fn compiled_loaders() -> &'static RwLock<HashMap<String, CompiledXamlLoader>> {
    static LOADERS: OnceLock<RwLock<HashMap<String, CompiledXamlLoader>>> = OnceLock::new();
    LOADERS.get_or_init(Default::default)
}

const NO_ASSET_LOADER: &str = "Could not create IAssetLoader : maybe Application.RegisterServices() wasn't called?";
const RELATIVE_WITHOUT_BASE: &str = "Cannot load relative Uri when BaseUri is null";

/// Loads XAML for a FerroUI application.
pub struct FerroXamlLoader;

impl FerroXamlLoader {
    /// Registers the loader of the compiled documents of the assembly
    /// (crate) named `assembly_name`, replacing a previous registration.
    /// Called by the generated startup code of a crate with compiled XAML.
    ///
    /// The assembly of a URI is the one the asset loader reports for it
    /// (`IAssetLoader::get_assembly`).
    pub fn register_compiled_xaml(assembly_name: &str, try_load: CompiledXamlLoader) {
        let mut loaders = compiled_loaders().write().unwrap_or_else(|e| e.into_inner());
        loaders.insert(assembly_name.to_string(), try_load);
    }

    /// Removes the registration of an assembly. Returns whether there was
    /// one.
    pub fn unregister_compiled_xaml(assembly_name: &str) -> bool {
        let mut loaders = compiled_loaders().write().unwrap_or_else(|e| e.into_inner());
        loaders.remove(assembly_name).is_some()
    }

    fn compiled_loader(assembly_name: &str) -> Option<CompiledXamlLoader> {
        compiled_loaders().read().unwrap_or_else(|e| e.into_inner()).get(assembly_name).copied()
    }

    /// Loads the XAML into a FerroUI component.
    ///
    /// This is the hook the XAML compiler replaces: in a class with
    /// compiled markup the call is rewritten to the generated populate
    /// method of the class. Reaching it at run time means that no compiled
    /// markup exists for the type of `obj`, which is the error it reports.
    pub fn load_object(obj: &BoxedValue) -> Result<(), XamlLoadException> {
        Self::load_object_with_service_provider(None, obj)
    }

    /// [`load_object`](Self::load_object) with the service provider of the
    /// place the component is loaded into.
    pub fn load_object_with_service_provider(
        sp: Option<&Rc<dyn IServiceProvider>>,
        obj: &BoxedValue,
    ) -> Result<(), XamlLoadException> {
        // Without compiled markup for the class the registered run-time loader
        // populates the object, as it loads documents by URI.
        match FerroLocator::current().get_service::<dyn IRuntimeXamlLoader>() {
            Some(runtime_loader) => runtime_loader.load_object(obj, sp),
            None => Err(Self::no_precompiled_xaml_for_object(obj)),
        }
    }

    pub(crate) fn no_precompiled_xaml_for_object(obj: &BoxedValue) -> XamlLoadException {
        let type_name = match crate::object_casts::as_object(obj) {
            Some(object) => object.get_type().full_name(),
            None => (**obj).type_name().to_string(),
        };
        XamlLoadException::with_message(format!(
            "No precompiled XAML found for {type_name}, make sure to specify x:Class and include your XAML file as FerroResource"
        ))
    }

    /// Loads XAML from a URI.
    ///
    /// `base_uri` is the URI `uri` is relative to; it is required when
    /// `uri` is relative.
    pub fn load(uri: &Uri, base_uri: Option<&Uri>) -> Result<BoxedValue, XamlLoadException> {
        Self::load_with_service_provider(None, uri, base_uri)
    }

    /// Loads XAML from a URI, giving the root of the document `sp` as its
    /// parent service provider.
    ///
    /// # Panics
    /// Panics if no asset loader is registered, or if `uri` is relative and
    /// `base_uri` is `None`.
    pub fn load_with_service_provider(
        sp: Option<&Rc<dyn IServiceProvider>>,
        uri: &Uri,
        base_uri: Option<&Uri>,
    ) -> Result<BoxedValue, XamlLoadException> {
        if FerroLocator::current().get_service::<dyn IAssetLoader>().is_none() {
            panic!("{NO_ASSET_LOADER}");
        }
        if !uri.is_absolute_uri() && base_uri.is_none() {
            panic!("{RELATIVE_WITHOUT_BASE}");
        }
        Self::try_load_with_service_provider(sp, uri, base_uri)
    }

    /// [`load_with_service_provider`](Self::load_with_service_provider)
    /// without panics: a missing asset loader and a relative URI without a
    /// base URI are errors too. This is what untyped (metadata) callers
    /// use.
    pub fn try_load_with_service_provider(
        sp: Option<&Rc<dyn IServiceProvider>>,
        uri: &Uri,
        base_uri: Option<&Uri>,
    ) -> Result<BoxedValue, XamlLoadException> {
        let Some(asset_locator) = FerroLocator::current().get_service::<dyn IAssetLoader>() else {
            return Err(XamlLoadException::with_message(NO_ASSET_LOADER));
        };

        let absolute_uri = if uri.is_absolute_uri() {
            uri.clone()
        } else {
            match base_uri {
                Some(base_uri) => Uri::combine(base_uri, uri),
                None => return Err(XamlLoadException::with_message(RELATIVE_WITHOUT_BASE)),
            }
        };

        let compiled_loader =
            asset_locator.get_assembly(uri, base_uri).and_then(|assembly| Self::compiled_loader(assembly.name()));
        if let Some(compiled_loader) = compiled_loader {
            if let Some(compiled_result) = compiled_loader(sp, absolute_uri.absolute_uri())? {
                return Ok(compiled_result);
            }
        }

        // This is intended for unit-tests only
        if let Some(runtime_loader) = FerroLocator::current().get_service::<dyn IRuntimeXamlLoader>() {
            let (stream, assembly) = asset_locator.open_and_get_assembly(uri, base_uri).map_err(|e| {
                XamlLoadException::with_inner(format!("Could not open the XAML document {uri} (baseUri: {}): {e}", display(base_uri)), e)
            })?;
            let stream: Box<dyn Read> = Box::new(stream);
            let mut document = RuntimeXamlLoaderDocument::from_stream_with_base_uri(Some(absolute_uri), stream);
            document.service_provider = sp.cloned();
            let mut configuration = RuntimeXamlLoaderConfiguration::default();
            configuration.local_assembly = MarkupAssembly::find(assembly.name());
            return runtime_loader.load(document, configuration);
        }

        Err(XamlLoadException::with_message(format!(
            "No precompiled XAML found for {uri} (baseUri: {}), make sure to specify x:Class and include your XAML file as FerroResource",
            display(base_uri)
        )))
    }
}

fn null_checked(obj: Option<BoxedValue>) -> Result<BoxedValue, XamlLoadException> {
    obj.ok_or_else(|| XamlLoadException::with_message("Value cannot be null. (Parameter 'obj')"))
}

fn display(uri: Option<&Uri>) -> String {
    uri.map(Uri::to_string).unwrap_or_default()
}

ferro_markup_type!(static FerroXamlLoader {
    methods: [
        static try fn Load(Option<BoxedValue>) => |obj: Option<BoxedValue>| {
            FerroXamlLoader::load_object(&null_checked(obj)?)
        },
        static try fn Load(Option<Rc<dyn IServiceProvider>>, Option<BoxedValue>) =>
            |sp: Option<Rc<dyn IServiceProvider>>, obj: Option<BoxedValue>| {
                FerroXamlLoader::load_object_with_service_provider(sp.as_ref(), &null_checked(obj)?)
            },
        static try fn Load(Uri, Option<Uri>) -> BoxedValue => |uri: Uri, base_uri: Option<Uri>| {
            FerroXamlLoader::try_load_with_service_provider(None, &uri, base_uri.as_ref())
        },
        static try fn Load(Option<Rc<dyn IServiceProvider>>, Uri, Option<Uri>) -> BoxedValue =>
            |sp: Option<Rc<dyn IServiceProvider>>, uri: Uri, base_uri: Option<Uri>| {
                FerroXamlLoader::try_load_with_service_provider(sp.as_ref(), &uri, base_uri.as_ref())
            },
    ],
});
