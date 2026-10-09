//! Port of `FerroRuntimeXamlLoader.cs`: the public API of the run-time XAML
//! loader.
//!
//! # Classes with markup (`x:Class`) without compiled markup
//!
//! In the managed original the constructor of a class with markup calls
//! `FerroXamlLoader.Load(this)`, which the XAML compiler rewrites to the
//! generated populate method of the class. Until markup is compiled here
//! (the Rust emitter), `FerroXamlLoader::load_object(instance)` delegates to
//! the registered run-time loader ([`FerroRuntimeXamlLoader::register`]),
//! which needs to know the document of the class:
//!
//! * a crate states it once, from its `register_types()`:
//!   `FerroRuntimeXamlLoader::register_class_document(MyControl::TYPE, "ferres://MyCrate/MyControl.xaml")`
//!   (the document is an asset of the crate, `ferroui_base::platform::register_assets`);
//! * `load_object` looks up the document of the class of the instance (or of the
//!   nearest base class that has one: the constructor of a base class with markup
//!   runs for instances of derived classes too), opens it through the asset loader
//!   and loads it with the instance as the root instance (`Populate`), the URI of
//!   the document as its base URI and the assembly of the asset as the local
//!   assembly. Includes of the document are linked at run time (the run-time
//!   include fallback of the include group transformer).
//!
//! A class without a registered document reports the error of the managed
//! original ("No precompiled XAML found for ..").

use std::collections::HashMap;
use std::io::{Cursor, Read};
use std::rc::Rc;
use std::sync::{OnceLock, RwLock};

use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::{from_markup_value, IServiceProvider, MarkupAssembly};
use ferroui_base::platform::IAssetLoader;
use ferroui_base::utilities::{Uri, UriKind};
use ferroui_base::{BoxedValue, FerroLocator, LocatorExtensions, TypeInfo};
use ferroui_markup_xaml::{
    IRuntimeXamlLoader, RuntimeXamlLoaderConfiguration, RuntimeXamlLoaderDocument, XamlLoadException,
};
use crate::runtime::framework::load_exception;

use crate::ferro_xaml_il_runtime_compiler::FerroXamlIlRuntimeCompiler;

/// Loads XAML at run time.
///
/// Every failure of a load (parsing, transforming, interpreting the
/// document, a member or markup extension that fails) is a
/// [`XamlLoadException`]: its message carries the line and position, its
/// inner error is the error of the compiler (`xamlx::exceptions::XamlError`).
pub struct FerroRuntimeXamlLoader;

impl FerroRuntimeXamlLoader {
    /// Loads XAML from a string.
    ///
    /// * `local_assembly`: the default assembly for `clr-namespace:`.
    /// * `root_instance`: the optional instance into which the XAML should
    ///   be loaded.
    /// * `uri`: the URI of the XAML being loaded.
    /// * `design_mode`: whether the XAML is being loaded in design mode.
    pub fn load(
        xaml: &str,
        local_assembly: Option<&'static MarkupAssembly>,
        root_instance: Option<BoxedValue>,
        uri: Option<Uri>,
        design_mode: bool,
    ) -> Result<BoxedValue, XamlLoadException> {
        let stream: Box<dyn Read> = Box::new(Cursor::new(xaml.as_bytes().to_vec()));
        Self::load_stream(stream, local_assembly, root_instance, uri, design_mode)
    }

    /// Loads XAML from a stream of UTF-8 text.
    pub fn load_stream(
        stream: Box<dyn Read>,
        local_assembly: Option<&'static MarkupAssembly>,
        root_instance: Option<BoxedValue>,
        uri: Option<Uri>,
        design_mode: bool,
    ) -> Result<BoxedValue, XamlLoadException> {
        let document = RuntimeXamlLoaderDocument::from_stream_with_base_uri_and_root_instance(uri, root_instance, stream);
        let mut configuration = RuntimeXamlLoaderConfiguration::new();
        configuration.design_mode = design_mode;
        configuration.local_assembly = local_assembly;
        Self::load_document(document, Some(configuration))
    }

    /// Loads a document. Without a configuration the default one is used.
    pub fn load_document(
        document: RuntimeXamlLoaderDocument,
        configuration: Option<RuntimeXamlLoaderConfiguration>,
    ) -> Result<BoxedValue, XamlLoadException> {
        let configuration = configuration.unwrap_or_default();
        // A document with an absolute URI is loaded as a group with the documents it
        // includes (directly or not) that the asset loader has: the group transformers
        // then link them as the compiler links the documents of a project (a style or
        // resource include becomes the included object, merged resource includes are
        // merged into the including dictionary).
        let uri = document.base_uri.clone().filter(|uri| uri.is_absolute_uri());
        let text = document
            .read_to_string()
            .map_err(|e| XamlLoadException::with_inner(format!("Unable to read the XAML document: {e}"), e))?;
        let mut included = Vec::new();
        let mut visited: Vec<String> = uri.iter().map(|uri| uri.absolute_uri().to_string()).collect();
        collect_included_documents(&*asset_loader(), uri.as_ref(), &text, &mut visited, &|_| false, &mut included);

        // The text was read: the document is given again, with the same properties.
        let stream: Box<dyn Read> = Box::new(Cursor::new(text.into_bytes()));
        let mut root = RuntimeXamlLoaderDocument::from_stream_with_base_uri_and_root_instance(
            document.base_uri.clone(),
            document.root_instance.clone(),
            stream,
        );
        root.service_provider = document.service_provider.clone();
        root.document = document.document.clone();
        if included.is_empty() {
            return FerroXamlIlRuntimeCompiler::load(root, &configuration).map_err(load_exception);
        }
        let mut documents = vec![root];
        documents.extend(included.into_iter().map(|(uri, text)| included_document(uri, text)));
        let loaded = FerroXamlIlRuntimeCompiler::load_group(documents, &configuration).map_err(load_exception)?;
        match loaded.into_iter().next() {
            Some(Some(root)) => Ok(root),
            _ => Err(XamlLoadException::with_message("The document built a null root object".to_string())),
        }
    }

    /// Loads a group of documents that may refer to each other. Returns the
    /// loaded object of each input document, in order.
    pub fn load_group(
        documents: Vec<RuntimeXamlLoaderDocument>,
        configuration: Option<RuntimeXamlLoaderConfiguration>,
    ) -> Result<Vec<Option<BoxedValue>>, XamlLoadException> {
        FerroXamlIlRuntimeCompiler::load_group(documents, &configuration.unwrap_or_default()).map_err(load_exception)
    }

    /// Parses XAML from a string.
    pub fn parse(xaml: &str, local_assembly: Option<&'static MarkupAssembly>) -> Result<BoxedValue, XamlLoadException> {
        Self::load(xaml, local_assembly, None, None, false)
    }

    /// Parses XAML from a string to an object of type `T` (the handle type
    /// of the object: `Ref<Border>`, `Rc<Style>`, ...). The cast of the
    /// managed original: an object of another type is an invalid cast.
    pub fn parse_as<T: Clone + 'static>(xaml: &str, local_assembly: Option<&'static MarkupAssembly>) -> Result<T, XamlLoadException> {
        let loaded = Self::parse(xaml, local_assembly)?;
        let type_name = loaded.type_name();
        from_markup_value::<T>(&Some(loaded)).ok_or_else(|| {
            XamlLoadException::with_message(format!(
                "Unable to cast object of type '{type_name}' to type '{}'.",
                std::any::type_name::<T>()
            ))
        })
    }

    /// States that the markup of `class` (the class its root element names with
    /// `x:Class`) is the document with the absolute URI `uri`, an asset of the crate.
    /// Process-wide; a later registration for the same class replaces the earlier one.
    /// See the module documentation.
    pub fn register_class_document(class: &'static TypeInfo, uri: &str) {
        let mut documents = class_documents().write().unwrap_or_else(|e| e.into_inner());
        documents.insert(class as *const TypeInfo as usize, uri.to_string());
    }

    /// The URI of the document registered for `class` or, failing that, for the
    /// nearest of its base classes that has one.
    pub fn class_document(class: &'static TypeInfo) -> Option<String> {
        let documents = class_documents().read().unwrap_or_else(|e| e.into_inner());
        let mut current = Some(class);
        while let Some(class) = current {
            if let Some(uri) = documents.get(&(class as *const TypeInfo as usize)) {
                return Some(uri.clone());
            }
            current = class.base_type();
        }
        None
    }

    /// Whether a document is registered for exactly `class`.
    pub(crate) fn has_class_document(class: &'static TypeInfo) -> bool {
        let documents = class_documents().read().unwrap_or_else(|e| e.into_inner());
        documents.contains_key(&(class as *const TypeInfo as usize))
    }

    /// Runs `create` (the constructor of `class`) with `populate` in place of the
    /// document registered for the class: the counterpart of the populate override
    /// field the run-time compiler of the managed original sets while it creates an
    /// instance of a class with compiled markup, so that the constructor populates the
    /// instance from the document being loaded instead. Returns what `create` returns
    /// and the result of the populate, if the constructor asked for it.
    pub(crate) fn with_populate_override<T>(
        class: &'static TypeInfo,
        populate: Rc<dyn Fn(&BoxedValue) -> xamlx::exceptions::XamlResult<()>>,
        create: impl FnOnce() -> T,
    ) -> (T, Option<xamlx::exceptions::XamlResult<()>>) {
        let key = class as *const TypeInfo as usize;
        let previous = POPULATE_OVERRIDES
            .with(|overrides| overrides.borrow_mut().insert(key, PopulateOverride { populate, result: None }));
        let created = create();
        let current = POPULATE_OVERRIDES.with(|overrides| {
            let mut overrides = overrides.borrow_mut();
            match previous {
                Some(previous) => overrides.insert(key, previous),
                None => overrides.remove(&key),
            }
        });
        (created, current.and_then(|current| current.result))
    }

    /// Populates `instance` from the document registered for its class
    /// ([`register_class_document`](Self::register_class_document)): what
    /// `FerroXamlLoader::load_object` does for a class without compiled markup.
    pub fn load_object(
        instance: &BoxedValue,
        service_provider: Option<&Rc<dyn IServiceProvider>>,
    ) -> Result<(), XamlLoadException> {
        let object = ValueTypes::as_object(&**instance);
        // The document being loaded by the run-time loader replaces the registered one
        // (`with_populate_override`). Its failure is reported by the load, not here: the
        // constructor that asked for the populate has nothing to do with it.
        if let Some(object) = &object {
            let key = object.get_type() as *const TypeInfo as usize;
            let populate = POPULATE_OVERRIDES
                .with(|overrides| overrides.borrow().get(&key).map(|current| current.populate.clone()));
            if let Some(populate) = populate {
                let result = populate(instance);
                POPULATE_OVERRIDES.with(|overrides| {
                    if let Some(current) = overrides.borrow_mut().get_mut(&key) {
                        current.result = Some(result);
                    }
                });
                return Ok(());
            }
        }
        let type_name = match &object {
            Some(object) => object.get_type().full_name(),
            None => (**instance).type_name().to_string(),
        };
        let Some(uri_text) = object.as_ref().and_then(|object| Self::class_document(object.get_type())) else {
            return Err(XamlLoadException::with_message(format!(
                "No precompiled XAML found for {type_name}, make sure to specify x:Class and include your XAML file as FerroResource"
            )));
        };
        let group = Self::document_group(&uri_text, &type_name)?;
        let (uri, text, included, assembly) = (group.uri, group.text, group.included, group.assembly);

        let stream: Box<dyn Read> = Box::new(Cursor::new(text.into_bytes()));
        let mut document =
            RuntimeXamlLoaderDocument::from_stream_with_base_uri_and_root_instance(Some(uri), Some(instance.clone()), stream);
        document.service_provider = service_provider.cloned();
        let mut documents = vec![document];
        documents.extend(included.into_iter().map(|(uri, text)| included_document(uri, text)));
        let mut configuration = RuntimeXamlLoaderConfiguration::new();
        configuration.local_assembly = assembly;
        Self::load_group(documents, Some(configuration)).map(|_| ())
    }

    /// The document at `uri_text` (of the class named `type_name`, for the
    /// errors) with every document it includes, directly or not, that the
    /// asset loader has, and the assembly of the document: the group the
    /// document of a class is loaded (and compiled) as.
    pub fn document_group(uri_text: &str, type_name: &str) -> Result<DocumentGroup, XamlLoadException> {
        Self::document_group_without(uri_text, type_name, &|_| false)
    }

    /// [`Self::document_group`] without the documents `skip` names, and without the
    /// documents only they include: the compiler of a crate leaves the documents of
    /// another crate with compiled markup to that crate (an include of one calls it
    /// there).
    #[cfg_attr(not(any(feature = "emitter", test)), allow(dead_code))]
    pub(crate) fn document_group_without(
        uri_text: &str,
        type_name: &str,
        skip: &dyn Fn(&Uri) -> bool,
    ) -> Result<DocumentGroup, XamlLoadException> {
        let uri = Uri::new(uri_text, UriKind::Absolute).map_err(|e| {
            XamlLoadException::with_message(format!("The URI '{uri_text}' of the XAML document of {type_name} is invalid: {e}"))
        })?;
        // The document of a class is found wherever the class is created, as compiled
        // markup is: without an asset loader service the registered assets are read
        // directly.
        let assets = asset_loader();
        let (mut stream, assembly) = assets.open_and_get_assembly(&uri, None).map_err(|e| {
            XamlLoadException::with_inner(format!("Could not open the XAML document {uri} of {type_name}: {e}"), e)
        })?;
        let mut text = String::new();
        stream.read_to_string(&mut text).map_err(|e| {
            XamlLoadException::with_inner(format!("Could not read the XAML document {uri} of {type_name}: {e}"), e)
        })?;

        // The document of a class is loaded as a group with every document it includes,
        // directly or not: the group transformers then link the documents to each other
        // as the compiler links the documents of a project (merged resource includes are
        // merged into the including dictionary, style and resource includes become the
        // included objects), so that the populated instance is the one compiled markup
        // builds.
        let mut included = Vec::new();
        collect_included_documents(&*assets, Some(&uri), &text, &mut vec![uri.absolute_uri().to_string()], skip, &mut included);
        Ok(DocumentGroup { uri, text, included, assembly: MarkupAssembly::find(assembly.name()) })
    }

    /// Makes the run-time loader the loader `FerroXamlLoader` falls back to
    /// for documents without compiled markup (the `IRuntimeXamlLoader`
    /// service). An application or a test host calls this once per
    /// locator scope.
    pub fn register() {
        if FerroLocator::current().get_service::<dyn IRuntimeXamlLoader>().is_none() {
            let loader: Rc<dyn IRuntimeXamlLoader> = Rc::new(RuntimeXamlLoaderService);
            FerroLocator::current_mutable().bind::<dyn IRuntimeXamlLoader>().to_constant(loader);
        }
    }
}

/// The document of a class with the documents it includes
/// ([`FerroRuntimeXamlLoader::document_group`]).
pub struct DocumentGroup {
    /// The URI and the text of the document.
    pub uri: Uri,
    pub text: String,
    /// The documents it includes, directly or not, in the order they are
    /// found, each with its text.
    pub included: Vec<(Uri, String)>,
    /// The assembly of the document, if it is registered.
    pub assembly: Option<&'static MarkupAssembly>,
}

/// The run-time loader as the service of the XAML runtime library.
struct RuntimeXamlLoaderService;

impl IRuntimeXamlLoader for RuntimeXamlLoaderService {
    fn load(
        &self,
        document: RuntimeXamlLoaderDocument,
        configuration: RuntimeXamlLoaderConfiguration,
    ) -> Result<BoxedValue, XamlLoadException> {
        FerroRuntimeXamlLoader::load_document(document, Some(configuration))
    }

    fn load_object(
        &self,
        instance: &BoxedValue,
        service_provider: Option<&Rc<dyn IServiceProvider>>,
    ) -> Result<(), XamlLoadException> {
        FerroRuntimeXamlLoader::load_object(instance, service_provider)
    }
}

/// The populate of the document being loaded for a class, and its result once the
/// constructor of the class has asked for it.
struct PopulateOverride {
    populate: Rc<dyn Fn(&BoxedValue) -> xamlx::exceptions::XamlResult<()>>,
    result: Option<xamlx::exceptions::XamlResult<()>>,
}

thread_local! {
    static POPULATE_OVERRIDES: std::cell::RefCell<HashMap<usize, PopulateOverride>> =
        std::cell::RefCell::new(HashMap::new());
}

/// The documents of the classes with markup, by class.
fn class_documents() -> &'static RwLock<HashMap<usize, String>> {
    static DOCUMENTS: OnceLock<RwLock<HashMap<usize, String>>> = OnceLock::new();
    DOCUMENTS.get_or_init(Default::default)
}

/// The asset loader service, or the loader of the registered assets when there is none:
/// the documents of a group are found wherever they are loaded, as compiled markup is.
fn asset_loader() -> Rc<dyn IAssetLoader> {
    match FerroLocator::current().get_service::<dyn IAssetLoader>() {
        Some(assets) => assets,
        None => Rc::new(ferroui_base::platform::StandardAssetLoader::new(None)),
    }
}

/// An included document of a group, named by its path as the compiler names it.
fn included_document(uri: Uri, text: String) -> RuntimeXamlLoaderDocument {
    let name = uri.absolute_path().trim_start_matches('/').to_string();
    let stream: Box<dyn Read> = Box::new(Cursor::new(text.into_bytes()));
    let mut document = RuntimeXamlLoaderDocument::from_stream_with_base_uri(Some(uri), stream);
    document.document = Some(name);
    document
}

/// The element names of the includes the group transformers link.
const INCLUDE_ELEMENTS: [&str; 3] = ["MergeResourceInclude", "ResourceInclude", "StyleInclude"];

/// The `Source` values of the include elements of a document, in document order. The
/// text is scanned without parsing it: a document that is not well formed fails when it
/// is loaded, with the position of the error. Sources given by a markup extension are
/// left to the run time.
pub(crate) fn include_sources(xaml: &str) -> Vec<String> {
    let mut sources = Vec::new();
    let mut rest = xaml;
    while let Some(start) = rest.find('<') {
        rest = &rest[start + 1..];
        if let Some(comment) = rest.strip_prefix("!--") {
            rest = comment.find("-->").map_or("", |end| &comment[end + 3..]);
            continue;
        }
        let name_end = rest.find(|c: char| c.is_whitespace() || c == '>' || c == '/').unwrap_or(rest.len());
        let name = &rest[..name_end];
        let local_name = name.rsplit(':').next().unwrap_or(name);
        if !INCLUDE_ELEMENTS.contains(&local_name) {
            continue;
        }
        let tag = &rest[name_end..rest[name_end..].find('>').map_or(rest.len(), |end| name_end + end)];
        let mut attributes = tag;
        while let Some(at) = attributes.find("Source") {
            let before_is_boundary = attributes[..at].chars().next_back().is_none_or(char::is_whitespace);
            let after = attributes[at + "Source".len()..].trim_start();
            attributes = &attributes[at + "Source".len()..];
            let Some(value) = after.strip_prefix('=') else { continue };
            if !before_is_boundary {
                continue;
            }
            let value = value.trim_start();
            let Some(quote) = value.chars().next().filter(|c| *c == '"' || *c == '\'') else { continue };
            if let Some(end) = value[1..].find(quote) {
                let source = &value[1..1 + end];
                if !source.is_empty() && !source.starts_with('{') {
                    sources.push(source.to_string());
                }
            }
            break;
        }
    }
    sources
}

/// Adds to `documents` the documents `xaml` (at `uri`, if it has one) includes, directly or not, that
/// the asset loader has and that are not in `visited` yet, each with its text.
fn collect_included_documents(
    assets: &dyn IAssetLoader,
    uri: Option<&Uri>,
    xaml: &str,
    visited: &mut Vec<String>,
    skip: &dyn Fn(&Uri) -> bool,
    documents: &mut Vec<(Uri, String)>,
) {
    for source in include_sources(xaml) {
        let Some(relative) = Uri::try_create(&source, UriKind::RelativeOrAbsolute) else { continue };
        // A relative source needs the URI of the including document.
        let included = match (relative.is_absolute_uri(), uri) {
            (true, _) => relative,
            (false, Some(uri)) => Uri::combine(uri, &relative),
            (false, None) => continue,
        };
        let key = included.absolute_uri().to_string();
        if visited.contains(&key) || skip(&included) || !assets.exists(&included, None) {
            continue;
        }
        visited.push(key);
        let Ok(mut stream) = assets.open(&included, None) else { continue };
        let mut text = String::new();
        if stream.read_to_string(&mut text).is_err() {
            continue;
        }
        let at = documents.len();
        documents.push((included.clone(), String::new()));
        collect_included_documents(assets, Some(&included), &text, visited, skip, documents);
        documents[at].1 = text;
    }
}

#[cfg(test)]
mod include_group_tests {
    use super::include_sources;

    #[test]
    fn include_sources_are_found_in_document_order() {
        let xaml = r#"<Styles xmlns="https://github.com/ferroui" xmlns:x="x">
  <!-- <StyleInclude Source="/Commented.xaml" /> -->
  <Styles.Resources>
    <ResourceDictionary>
      <ResourceDictionary.MergedDictionaries>
        <MergeResourceInclude Source="/Accents/Base.xaml" />
        <ResourceInclude x:Key="k" Source='Controls/Button.xaml'/>
        <ResourceInclude Source="{Binding X}" />
      </ResourceDictionary.MergedDictionaries>
    </ResourceDictionary>
  </Styles.Resources>
  <StyleInclude
      Source = "ferres://Other/Styles.xaml" />
  <Border Tag="Source=x" DataSource="y" />
  <local:StyleInclude Source="/Local.xaml" />
</Styles>"#;
        assert_eq!(
            include_sources(xaml),
            ["/Accents/Base.xaml", "Controls/Button.xaml", "ferres://Other/Styles.xaml", "/Local.xaml"]
        );
    }
}
