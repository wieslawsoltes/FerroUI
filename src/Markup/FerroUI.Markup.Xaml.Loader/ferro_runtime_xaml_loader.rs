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

use std::io::{Cursor, Read};
use std::rc::Rc;

use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::{from_markup_value, IServiceProvider, MarkupAssembly};
use ferroui_base::platform::IAssetLoader;
use ferroui_base::utilities::{Uri, UriKind};
use ferroui_base::{BoxedValue, FerroLocator, LocatorExtensions, TypeInfo};
use ferroui_markup_xaml::{
    IRuntimeXamlLoader, RuntimeXamlLoaderConfiguration, RuntimeXamlLoaderDocument, XamlLoadError,
};
use crate::runtime_document_input::DocumentInput;
use crate::runtime::framework::load_exception;

use crate::ferro_xaml_il_runtime_compiler::FerroXamlIlRuntimeCompiler;

/// Loads XAML at run time.
///
/// Every failure of a load (parsing, transforming, interpreting the
/// document, a member or markup extension that fails) is a
/// [`XamlLoadError`]: its message carries the line and position, its
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
    ) -> Result<BoxedValue, XamlLoadError> {
        let stream: Box<dyn Read> = Box::new(Cursor::new(xaml.as_bytes().to_vec()));
        Self::load_stream(stream, local_assembly, root_instance, uri, design_mode)
    }

    /// Loads a UTF-8 XAML stream or, with `portable-xaml`, an FXD syntax document.
    pub fn load_stream(
        stream: Box<dyn Read>,
        local_assembly: Option<&'static MarkupAssembly>,
        root_instance: Option<BoxedValue>,
        uri: Option<Uri>,
        design_mode: bool,
    ) -> Result<BoxedValue, XamlLoadError> {
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
    ) -> Result<BoxedValue, XamlLoadError> {
        let configuration = configuration.unwrap_or_default();
        // A document with an absolute URI is loaded as a group with the documents it
        // includes (directly or not) that the asset loader has: the group transformers
        // then link them as the compiler links the documents of a project (a style or
        // resource include becomes the included object, merged resource includes are
        // merged into the including dictionary).
        let input = DocumentInput::read(&mut **document.xaml_stream())
            .map_err(|e| XamlLoadError::with_inner(format!("Unable to read the XAML document: {e}"), e))?;
        let assets = asset_loader();
        let inputs = Self::collect_inputs(document, input, &|uri| read_included_asset(&*assets, uri))?;
        Self::build_root(inputs, &configuration)
    }

    /// Loads one document and its transitive includes from an FXP archive. References
    /// outside its assembly still use the application's asset loader. An existing root
    /// and a parent service provider have the same semantics as a normal class populate.
    #[cfg(feature = "portable-xaml")]
    pub fn load_packed_document(
        pack: &crate::binary_document::pack::ThemePack<'_>,
        uri: Uri,
        root: Option<BoxedValue>,
        parent: Option<Rc<dyn IServiceProvider>>,
        configuration: RuntimeXamlLoaderConfiguration,
    ) -> Result<BoxedValue, XamlLoadError> {
        let inputs = Self::packed_inputs(pack, uri, root, parent)?;
        Self::build_root(inputs, &configuration)
    }

    /// Resolve a complete include closure once. The returned plan constructs a fresh
    /// root on each call, retaining the resolved program independently of the source
    /// archive and the process-wide registrations used to resolve its members.
    #[cfg(feature = "portable-xaml")]
    pub fn prepare_packed_document(
        pack: &crate::binary_document::pack::ThemePack<'_>,
        uri: Uri,
        configuration: RuntimeXamlLoaderConfiguration,
    ) -> Result<crate::PreparedPackedDocument, XamlLoadError> {
        let inputs = Self::packed_inputs(pack, uri, None, None)?;
        crate::PreparedPackedDocument::prepare(inputs, &configuration).map_err(load_exception)
    }

    #[cfg(feature = "portable-xaml")]
    fn packed_inputs(
        pack: &crate::binary_document::pack::ThemePack<'_>, uri: Uri,
        root: Option<BoxedValue>, parent: Option<Rc<dyn IServiceProvider>>,
    ) -> Result<Vec<(RuntimeXamlLoaderDocument, DocumentInput)>, XamlLoadError> {
        use ferroui_base::utilities::UriExtensions;
        if !uri.is_absolute_uri() || !UriExtensions::is_asset(&uri) || UriExtensions::authority(&uri).is_empty() {
            return Err(XamlLoadError::with_message("A packed document requires an absolute ferres asset URI"));
        }
        let authority = UriExtensions::authority(&uri).to_owned();
        let path = UriExtensions::get_unescape_absolute_path(&uri);
        let path = path.trim_start_matches('/');
        let mut document = RuntimeXamlLoaderDocument::from_stream_with_base_uri_and_root_instance(
            Some(uri), root, Box::new(std::io::empty()),
        );
        document.document = Some(path.to_string());
        document.service_provider = parent;
        let input = |path: &str| pack.document(path).map(DocumentInput::from_portable).map_err(load_exception);
        let assets = asset_loader();
        Self::collect_inputs(document, input(path)?, &|uri| {
            if UriExtensions::is_asset(uri) && UriExtensions::authority(uri).eq_ignore_ascii_case(&authority) {
                let name = UriExtensions::get_unescape_absolute_path(uri);
                let name = name.trim_start_matches('/');
                if pack.contains(name) {
                    return input(name).map(Some);
                }
            }
            read_included_asset(&*assets, uri)
        })
    }

    fn collect_inputs(
        document: RuntimeXamlLoaderDocument,
        mut input: DocumentInput,
        open: &OpenInclude<'_>,
    ) -> Result<Vec<(RuntimeXamlLoaderDocument, DocumentInput)>, XamlLoadError> {
        let uri = document.base_uri.clone().filter(|uri| uri.is_absolute_uri());
        let mut included = Vec::new();
        let mut visited: Vec<String> = uri.iter().map(|uri| uri.absolute_uri().to_string()).collect();
        collect_included_documents(open, uri.as_ref(), input.include_sources().map_err(load_exception)?, &mut visited, &|_| false, &mut included)?;
        let mut documents = vec![(document, input)];
        documents.extend(included.into_iter().map(|(uri, input)| {
            let name = uri.absolute_path().trim_start_matches('/').to_string();
            let mut included = RuntimeXamlLoaderDocument::from_stream_with_base_uri(Some(uri), Box::new(std::io::empty()));
            included.document = Some(name);
            (included, input)
        }));
        Ok(documents)
    }

    fn build_root(
        documents: Vec<(RuntimeXamlLoaderDocument, DocumentInput)>,
        configuration: &RuntimeXamlLoaderConfiguration,
    ) -> Result<BoxedValue, XamlLoadError> {
        // A root load creates a single object. Included documents participate in
        // group transforms and are built by their referring sites, with the parent
        // services and ambient resources of that site.
        let loaded = FerroXamlIlRuntimeCompiler::load_inputs(documents, configuration, 1).map_err(load_exception)?;
        loaded.into_iter().next().flatten().ok_or_else(|| XamlLoadError::with_message("The document built a null root object".to_string()))
    }

    /// Loads a group of documents that may refer to each other. Returns the
    /// loaded object of each input document, in order.
    pub fn load_group(
        documents: Vec<RuntimeXamlLoaderDocument>,
        configuration: Option<RuntimeXamlLoaderConfiguration>,
    ) -> Result<Vec<Option<BoxedValue>>, XamlLoadError> {
        FerroXamlIlRuntimeCompiler::load_group(documents, &configuration.unwrap_or_default()).map_err(load_exception)
    }

    /// Parses XAML from a string.
    pub fn parse(xaml: &str, local_assembly: Option<&'static MarkupAssembly>) -> Result<BoxedValue, XamlLoadError> {
        Self::load(xaml, local_assembly, None, None, false)
    }

    /// Parses XAML from a string to an object of type `T` (the handle type
    /// of the object: `Ref<Border>`, `Rc<Style>`, ...). The cast of the
    /// managed original: an object of another type is an invalid cast.
    pub fn parse_as<T: Clone + 'static>(xaml: &str, local_assembly: Option<&'static MarkupAssembly>) -> Result<T, XamlLoadError> {
        let loaded = Self::parse(xaml, local_assembly)?;
        let type_name = loaded.type_name();
        from_markup_value::<T>(&Some(loaded)).ok_or_else(|| {
            XamlLoadError::with_message(format!(
                "Unable to cast object of type '{type_name}' to type '{}'.",
                std::any::type_name::<T>()
            ))
        })
    }

    /// Registers the class with the XAML runtime shared by compiled markup and the interpreter.
    pub fn register_class_document(class: &'static TypeInfo, uri: &str) {
        ferroui_markup_xaml::FerroXamlLoader::register_class_document(class, uri);
    }

    /// The registered document for this class or its nearest registered base class.
    pub fn class_document(class: &'static TypeInfo) -> Option<String> {
        ferroui_markup_xaml::FerroXamlLoader::class_document(class)
    }

    pub(crate) fn has_class_document(class: &'static TypeInfo) -> bool {
        ferroui_markup_xaml::FerroXamlLoader::has_class_document(class)
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
        let result = Rc::new(std::cell::RefCell::new(None));
        let captured = result.clone();
        let _guard = ferroui_markup_xaml::FerroXamlLoader::scoped_populate_override(class, Rc::new(move |_, target| {
            *captured.borrow_mut() = Some(populate(target));
            Ok(())
        }));
        let created = create();
        let populated = result.borrow_mut().take();
        (created, populated)
    }

    /// Populates `instance` from the document registered for its class
    /// ([`register_class_document`](Self::register_class_document)): what
    /// `FerroXamlLoader::load_object` does for a class without compiled markup.
    pub fn load_object(
        instance: &BoxedValue,
        service_provider: Option<&Rc<dyn IServiceProvider>>,
    ) -> Result<(), XamlLoadError> {
        let object = ValueTypes::as_object(&**instance);
        if let Some(object) = &object {
            if let Some(populate) = ferroui_markup_xaml::FerroXamlLoader::populate_override(object.get_type()) {
                return populate(service_provider, instance);
            }
        }
        let type_name = match &object {
            Some(object) => object.get_type().full_name(),
            None => (**instance).type_name().to_string(),
        };
        let Some(uri_text) = object.as_ref().and_then(|object| Self::class_document(object.get_type())) else {
            return Err(XamlLoadError::with_message(format!(
                "No precompiled XAML found for {type_name}, make sure to specify x:Class and include your XAML file as FerroResource"
            )));
        };
        let (uri, stream, assembly) = open_class_asset(&*asset_loader(), &uri_text, &type_name)?;
        let mut document = RuntimeXamlLoaderDocument::from_stream_with_base_uri_and_root_instance(
            Some(uri), Some(instance.clone()), stream,
        );
        document.service_provider = service_provider.cloned();
        let mut configuration = RuntimeXamlLoaderConfiguration::new();
        configuration.local_assembly = assembly;
        Self::load_document(document, Some(configuration)).map(|_| ())
    }

    /// The document at `uri_text` (of the class named `type_name`, for the
    /// errors) with every document it includes, directly or not, that the
    /// asset loader has, and the assembly of the document: the group the
    /// document of a class is loaded (and compiled) as.
    pub fn document_group(uri_text: &str, type_name: &str) -> Result<DocumentGroup, XamlLoadError> {
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
    ) -> Result<DocumentGroup, XamlLoadError> {
        let assets = asset_loader();
        let (uri, mut stream, assembly) = open_class_asset(&*assets, uri_text, type_name)?;
        let read_error = |e| XamlLoadError::with_inner(format!("Could not read the XAML source group {uri} of {type_name}: {e}"), e);
        let mut input = DocumentInput::read(&mut *stream).map_err(read_error)?;
        let mut included = Vec::new();
        collect_included_documents(&|uri| read_included_asset(&*assets, uri), Some(&uri), input.include_sources().map_err(load_exception)?, &mut vec![uri.absolute_uri().to_string()], skip, &mut included)?;
        let text = input.into_text().map_err(read_error)?;
        let included = included.into_iter().map(|(uri, doc)| doc.into_text().map(|text| (uri, text))).collect::<std::io::Result<_>>().map_err(read_error)?;
        Ok(DocumentGroup { uri, text, included, assembly })
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
    ) -> Result<BoxedValue, XamlLoadError> {
        FerroRuntimeXamlLoader::load_document(document, Some(configuration))
    }

    fn load_object(
        &self,
        instance: &BoxedValue,
        service_provider: Option<&Rc<dyn IServiceProvider>>,
    ) -> Result<(), XamlLoadError> {
        FerroRuntimeXamlLoader::load_object(instance, service_provider)
    }
}

/// The asset loader service, or the loader of the registered assets when there is none:
/// the documents of a group are found wherever they are loaded, as compiled markup is.
fn asset_loader() -> Rc<dyn IAssetLoader> {
    match FerroLocator::current().get_service::<dyn IAssetLoader>() {
        Some(assets) => assets,
        None => Rc::new(ferroui_base::platform::StandardAssetLoader::new(None)),
    }
}

/// Open the registered asset as a stream so runtime class population accepts both encodings.
fn open_class_asset(
    assets: &dyn IAssetLoader, uri_text: &str, type_name: &str,
) -> Result<(Uri, Box<dyn Read>, Option<&'static MarkupAssembly>), XamlLoadError> {
    let uri = Uri::new(uri_text, UriKind::Absolute).map_err(|e| {
        XamlLoadError::with_message(format!("The URI '{uri_text}' of the XAML document of {type_name} is invalid: {e}"))
    })?;
    let (stream, assembly) = assets.open_and_get_assembly(&uri, None).map_err(|e| {
        XamlLoadError::with_inner(format!("Could not open the XAML document {uri} of {type_name}: {e}"), e)
    })?;
    Ok((uri, stream, MarkupAssembly::find(assembly.name())))
}

type OpenInclude<'a> = dyn Fn(&Uri) -> Result<Option<DocumentInput>, XamlLoadError> + 'a;

fn read_included_asset(assets: &dyn IAssetLoader, uri: &Uri) -> Result<Option<DocumentInput>, XamlLoadError> {
    #[cfg(all(feature = "hot-reload", debug_assertions))]
    if let Some(text) = ferroui_markup_xaml::source_overlay::source(uri.absolute_uri()) {
        return Ok(Some(DocumentInput::xml(text)));
    }
    let Ok(mut stream) = assets.open(uri, None) else { return Ok(None) };
    DocumentInput::read(&mut *stream).map(Some).map_err(|error|
        XamlLoadError::with_inner(format!("Could not read included XAML document {uri}: {error}"), error)
    )
}

/// Adds to `documents` the documents `xaml` (at `uri`, if it has one) includes, directly or not, that
/// the asset loader has and that are not in `visited` yet, each with its text.
fn collect_included_documents(
    open: &OpenInclude<'_>,
    uri: Option<&Uri>,
    sources: Vec<String>,
    visited: &mut Vec<String>,
    skip: &dyn Fn(&Uri) -> bool,
    documents: &mut Vec<(Uri, DocumentInput)>,
) -> Result<(), XamlLoadError> {
    for source in sources {
        let Some(relative) = Uri::try_create(&source, UriKind::RelativeOrAbsolute) else { continue };
        // A relative source needs the URI of the including document.
        let included = match (relative.is_absolute_uri(), uri) {
            (true, _) => relative,
            (false, Some(uri)) => Uri::combine(uri, &relative),
            (false, None) => continue,
        };
        let key = included.absolute_uri().to_string();
        if visited.contains(&key) || skip(&included) {
            continue;
        }
        visited.push(key);
        let Some(mut input) = open(&included)? else { continue };
        let sources = input.include_sources().map_err(load_exception)?;
        documents.push((included.clone(), input));
        collect_included_documents(open, Some(&included), sources, visited, skip, documents)?;
    }
    Ok(())
}

#[cfg(all(test, feature = "portable-xaml"))]
mod portable_tests {
    use super::*;
    use crate::binary_document;
    use ferroui_base::{Ref, controls::ResourceDictionary, media::{Colors, SolidColorBrush}};
    use std::sync::OnceLock;

    #[test]
    fn mixed_xml_and_fxd_asset_groups_follow_nested_relative_includes() {
        const LEAF: &str = r#"<ResourceDictionary xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><x:String x:Key="Message">from the leaf</x:String></ResourceDictionary>"#;
        fn include(path: &str) -> String {
            format!(r#"<ResourceDictionary xmlns="https://github.com/ferroui"><ResourceDictionary.MergedDictionaries><MergeResourceInclude Source="{path}"/></ResourceDictionary.MergedDictionaries></ResourceDictionary>"#)
        }
        static CHILD: OnceLock<Vec<u8>> = OnceLock::new();
        ferroui_base::platform::register_assets("fxd-includes", &[
            ("Child.xaml", CHILD.get_or_init(|| binary_document::compile(&include("Dir/A&amp;B.xaml")).unwrap())),
            ("Dir/A&B.xaml", LEAF.as_bytes()),
        ]);
        // An XML property element and an entity-escaped filename must discover the
        // same assets through both encodings, including the FXD child's dependency.
        let source = include("Child.xaml").replace(
            "<MergeResourceInclude Source=\"Child.xaml\"/>",
            "<MergeResourceInclude><MergeResourceInclude.Source>Child.xaml</MergeResourceInclude.Source></MergeResourceInclude>",
        );
        for bytes in [source.as_bytes().to_vec(), binary_document::compile(&source).unwrap()] {
            let root = FerroRuntimeXamlLoader::load_stream(Box::new(Cursor::new(bytes)), None, None,
                Some(Uri::absolute("ferres://fxd-includes/Root.xaml").unwrap()), false).unwrap();
            let dictionary = from_markup_value::<Ref<ResourceDictionary>>(&Some(root)).unwrap();
            assert_eq!(from_markup_value::<String>(&dictionary.get(&"Message".into())).as_deref(), Some("from the leaf"));
        }
    }

    #[test]
    fn prepared_document_keeps_its_includes_but_builds_fresh_instances() {
        use crate::binary_document::pack::{compile_documents, ThemePack};
        let prepare = |text: &str| {
            let root = r#"<ResourceDictionary xmlns="https://github.com/ferroui"><ResourceDictionary.MergedDictionaries><MergeResourceInclude Source="Child.xaml"/></ResourceDictionary.MergedDictionaries></ResourceDictionary>"#;
            let child = format!(r#"<ResourceDictionary xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><x:String x:Key="Label">{text}</x:String></ResourceDictionary>"#);
            let bytes = compile_documents(&[("Root.xaml", root), ("Child.xaml", &child)]).unwrap();
            let pack = ThemePack::open(&bytes).unwrap();
            FerroRuntimeXamlLoader::prepare_packed_document(
                &pack, Uri::absolute("ferres://prepared-test/Root.xaml").unwrap(), Default::default(),
            ).unwrap()
        };
        let first = prepare("one");
        let second = prepare("two");
        let build = |plan: &crate::PreparedPackedDocument| {
            from_markup_value::<Ref<ResourceDictionary>>(&Some(plan.build(None).unwrap())).unwrap()
        };
        let label = |dict: &Ref<ResourceDictionary>| from_markup_value::<String>(&dict.get(&"Label".into())).unwrap();
        let a = build(&first);
        let b = build(&second);
        assert_eq!(label(&b), "two");
        a.remove(&"Label".into());
        let c = build(&first);
        assert!(!c.ptr_eq(&a));
        assert_eq!(label(&c), "one");
    }

    #[test]
    fn archive_load_preserves_root_identity_and_nested_include_locations() {
        use crate::binary_document::pack::{compile_documents, ThemePack};
        let leaf = r#"<ResourceDictionary xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><x:String x:Key="Message">packed</x:String></ResourceDictionary>"#;
        let include = |source: &str| format!(r#"<ResourceDictionary xmlns="https://github.com/ferroui"><ResourceDictionary.MergedDictionaries><MergeResourceInclude Source="{source}"/></ResourceDictionary.MergedDictionaries></ResourceDictionary>"#);
        let bytes = compile_documents(&[
            ("View/Root.xaml", &include("Child/Middle.xaml")),
            ("View/Child/Middle.xaml", &include("../Leaf.xaml")),
            ("View/Leaf.xaml", leaf),
            // A sibling that was never referenced must not be transformed or constructed.
            ("Unused.xaml", "<Missing xmlns='using:Not.Registered'/>"),
        ]).unwrap();
        let pack = ThemePack::open(&bytes).unwrap();
        let target = ResourceDictionary::new();
        let value = FerroRuntimeXamlLoader::load_packed_document(
            &pack, Uri::absolute("ferres://test/View/Root.xaml").unwrap(), Some(Rc::new(target.clone())), None,
            RuntimeXamlLoaderConfiguration::new(),
        ).unwrap();
        assert!(from_markup_value::<Ref<ResourceDictionary>>(&Some(value)).unwrap().ptr_eq(&target));
        assert_eq!(from_markup_value::<String>(&target.get(&"Message".into())).as_deref(), Some("packed"));
    }

    #[test]
    fn portable_stream_populates_existing_root_and_preserves_diagnostics() {
        let markup = "<SolidColorBrush xmlns='https://github.com/ferroui' Color='Red'/>";
        let brush = SolidColorBrush::new();
        let bytes = binary_document::compile(markup).unwrap();
        let root = FerroRuntimeXamlLoader::load_stream(Box::new(Cursor::new(bytes)), None,
            Some(Rc::new(brush.clone())), None, false).unwrap();
        assert!(from_markup_value::<Ref<SolidColorBrush>>(&Some(root)).unwrap().ptr_eq(&brush));
        assert_eq!(brush.color(), Colors::RED);
        let invalid = "<SolidColorBrush xmlns='https://github.com/ferroui'\n Color='not-a-color'/>";
        let load_error = |bytes| FerroRuntimeXamlLoader::load_stream(Box::new(Cursor::new(bytes)), None, None, None, false)
            .err().expect("invalid color").message().to_owned();
        assert_eq!(load_error(invalid.as_bytes().to_vec()), load_error(binary_document::compile(invalid).unwrap()));
        assert!(load_error(b"FXD\0\xff".to_vec()).contains("FXD"));
    }
}
