//! Port of `FerroXamlIlRuntimeCompiler.cs`: loads documents at run time.
//!
//! The managed original compiles the transformed documents to IL with
//! `System.Reflection.Emit` and runs the generated `Build` / `Populate`
//! methods. Here the same transformed documents are interpreted
//! ([`crate::runtime::interpreter`]) over the run-time type system
//! ([`crate::runtime::type_system`]); everything up to the back end
//! (parsing, root type substitution, the transformer pipeline, the group
//! transformers, the handling of diagnostics) is the upstream sequence.

use std::cell::RefCell;
use std::rc::Rc;

use ferroui_base::metadata::{IServiceProvider, MarkupValue};
use ferroui_base::BoxedValue;
use ferroui_markup_xaml::xaml_il::runtime::XamlIlRuntimeHelpers;
use ferroui_markup_xaml::{
    RuntimeXamlDiagnostic, RuntimeXamlDiagnosticSeverity, RuntimeXamlLoaderConfiguration, RuntimeXamlLoaderDocument,
};
use xamlx::ast::XamlAstExtensions as _;
use xamlx::ast::XamlAstNodeExtensions as _;
use xamlx::ast::{IXamlAstValueNode, XamlDocument};
use xamlx::diagnostics::{throw_exception_if_any_error, XamlDiagnostic, XamlDiagnosticSeverity};
use xamlx::exceptions::{XamlError, XamlResult};
use xamlx::transform::XamlDiagnosticsHandler;
use xamlx::type_system::{IXamlAssembly, IXamlType, IXamlTypeSystem};

use crate::compiler_extensions::{
    FerroXamlDiagnosticCodes, FerroXamlIlCompiler, FerroXamlIlCompilerConfiguration, FerroXamlIlLanguage,
    IXamlDocumentResource, IXamlDocumentTypeBuilderProvider, XamlDocumentResource,
};
use crate::runtime::framework::{self, DocumentBody, RuntimeDocumentTypeBuilderProvider};
use crate::runtime::type_system::RuntimeTypeSystem;

ferroui_base::ferro_thread_local! {
    static TYPE_SYSTEM: RefCell<Option<Rc<RuntimeTypeSystem>>> = const { RefCell::new(None) };
}

/// What a document is transformed from.
struct DocumentSource {
    xaml: String,
    /// The type of the root instance the document populates.
    override_type: Option<Rc<dyn IXamlType>>,
    /// The name of the document in diagnostics.
    name: String,
    base_uri: Option<String>,
}

/// The transformed documents of a group, ready to be built or populated: the counterpart
/// of the builder types the managed original compiles a group into. Interpreting a
/// document does not change it (templates and deferred resources are built from the same
/// nodes any number of times), so a group is interpreted as often as it is loaded.
struct TransformedGroup {
    providers: Vec<Rc<RuntimeDocumentTypeBuilderProvider>>,
    root_types: Vec<Rc<dyn IXamlType>>,
}

/// The environment variable that makes the loader print, for every load, the time it
/// spent parsing, transforming, group-transforming and interpreting (standard error).
const TIMINGS_VARIABLE: &str = "FERROUI_XAML_TIMINGS";

/// Bound edit history without flushing frequently used documents. Source bytes
/// are an admission budget, not a measurement of the transformed AST's heap size.
const MAX_CACHED_GROUPS: usize = 64;
const MAX_CACHED_SOURCE_BYTES: usize = 4 * 1024 * 1024;

#[derive(Default)]
struct GroupCache {
    entries: std::collections::VecDeque<(String, usize, Rc<TransformedGroup>)>,
    source_bytes: usize,
}
impl GroupCache {
    fn get(&mut self, key: &str) -> Option<Rc<TransformedGroup>> {
        let index = self.entries.iter().position(|entry| entry.0 == key)?;
        let entry = self.entries.remove(index)?;
        let group = entry.2.clone();
        self.entries.push_back(entry);
        Some(group)
    }

    fn insert(&mut self, key: String, source_bytes: usize, group: Rc<TransformedGroup>) {
        if source_bytes > MAX_CACHED_SOURCE_BYTES {
            return;
        }
        if let Some(index) = self.entries.iter().position(|entry| entry.0 == key) {
            self.source_bytes -= self.entries.remove(index).unwrap().1;
        }
        while self.entries.len() >= MAX_CACHED_GROUPS
            || self.source_bytes + source_bytes > MAX_CACHED_SOURCE_BYTES
        {
            let (_, bytes, _) = self.entries.pop_front().unwrap();
            self.source_bytes -= bytes;
        }
        self.source_bytes += source_bytes;
        self.entries.push_back((key, source_bytes, group));
    }
}

ferroui_base::ferro_thread_local! {
    /// The transformed groups of this thread by [`group_key`].
    static GROUPS: RefCell<GroupCache> = RefCell::new(GroupCache::default());
}

/// Identifies the result of transforming a group: the parts of the configuration the
/// transform depends on (design mode, the default of compiled bindings, source info, the
/// local assembly) and, for each document in order, its name, base URI, the type of its
/// root instance and its text. The documents of a group are linked to each other by the
/// group transformers, so the key is the key of the whole group. A changed text is
/// another key.
fn group_key(sources: &[DocumentSource], configuration: &RuntimeXamlLoaderConfiguration) -> String {
    use std::fmt::Write as _;
    use std::hash::{Hash as _, Hasher as _};
    let mut key = format!(
        "{}|{}|{}|{}",
        configuration.design_mode,
        configuration.use_compiled_bindings_by_default,
        configuration.create_source_info(),
        configuration.local_assembly.map_or("", |assembly| assembly.name),
    );
    for source in sources {
        // Two independent hashes of the text and its length stand for the text.
        let mut first = std::collections::hash_map::DefaultHasher::new();
        source.xaml.hash(&mut first);
        let mut second = std::collections::hash_map::DefaultHasher::new();
        (source.xaml.len(), "ferro-xaml").hash(&mut second);
        source.xaml.bytes().rev().for_each(|byte| byte.hash(&mut second));
        let _ = write!(
            key,
            "\n{}|{}|{}|{}|{:016x}{:016x}",
            source.name,
            source.base_uri.as_deref().unwrap_or(""),
            source.override_type.as_ref().map(|type_| type_.get_full_name()).unwrap_or_default(),
            source.xaml.len(),
            first.finish(),
            second.finish(),
        );
    }
    key
}

/// Loads documents at run time.
pub struct FerroXamlIlRuntimeCompiler;

impl FerroXamlIlRuntimeCompiler {
    /// The run-time type system of the calling thread: created on first use,
    /// after the type tables of the framework crates have been registered.
    /// Types registered later are found too.
    pub fn type_system() -> Rc<RuntimeTypeSystem> {
        TYPE_SYSTEM.with(|slot| {
            if let Some(system) = &*slot.borrow() {
                return system.clone();
            }
            // The type system only sees what is registered: make sure the framework is.
            ferroui_markup_xaml::register_types();
            let system = RuntimeTypeSystem::new();
            *slot.borrow_mut() = Some(system.clone());
            system
        })
    }

    /// `Load(document, configuration)`.
    pub fn load(
        document: RuntimeXamlLoaderDocument,
        configuration: &RuntimeXamlLoaderConfiguration,
    ) -> XamlResult<BoxedValue> {
        let mut loaded = Self::load_group(vec![document], configuration)?;
        match loaded.pop() {
            Some(Some(object)) if loaded.is_empty() => Ok(object),
            Some(None) if loaded.is_empty() => {
                Err(XamlError::internal("NullReferenceException", "The document built a null root object"))
            }
            _ => Err(XamlError::invalid_operation("Sequence contains more than one element")),
        }
    }

    /// `LoadGroup(documents, configuration)`: the objects the documents
    /// build, in the order of the documents.
    pub fn load_group(
        documents: Vec<RuntimeXamlLoaderDocument>,
        configuration: &RuntimeXamlLoaderConfiguration,
    ) -> XamlResult<Vec<Option<BoxedValue>>> {
        let runtime_type_system = Self::type_system();

        // The text of the documents and the types of their root instances: what, with the
        // configuration, determines the transformed documents.
        let mut sources: Vec<DocumentSource> = Vec::with_capacity(documents.len());
        for (index, document) in documents.iter().enumerate() {
            let xaml = document.read_to_string().map_err(|e| {
                XamlError::internal("IOException", format!("Unable to read the XAML document: {e}"))
            })?;
            let override_type: Option<Rc<dyn IXamlType>> =
                document.root_instance.as_ref().map(|root| runtime_type_system.runtime_type_of(root));
            #[cfg(all(feature = "hot-reload", debug_assertions))]
            let override_type = document.service_provider.as_ref()
                .and_then(|provider| ferroui_markup_xaml::reload_events::code_behind(&**provider, &document.root_instance))
                .map(|root| runtime_type_system.runtime_type_of(&root)).or(override_type);
            sources.push(DocumentSource {
                xaml,
                override_type,
                name: document.document.clone().unwrap_or_else(|| format!("runtimexaml{index}")),
                base_uri: document.base_uri.as_ref().map(|uri| uri.to_string()),
            });
        }

        // A group whose documents were transformed before on this thread is only
        // interpreted again. A diagnostic handler sees the diagnostics of the transform,
        // so a load with one always transforms.
        let cache_key = configuration.diagnostic_handler.is_none().then(|| group_key(&sources, configuration));
        let cached = cache_key.as_ref().and_then(|key| GROUPS.with(|groups| groups.borrow_mut().get(key)));
        let timings = std::env::var_os(TIMINGS_VARIABLE).is_some();
        let was_cached = cached.is_some();
        let interpret_start = std::time::Instant::now();
        let group = match cached {
            Some(group) => group,
            None => {
                let type_system = runtime_type_system.as_type_system();
                let group =
                    Rc::new(Self::transform_group(&runtime_type_system, type_system, &sources, configuration)?);
                if let Some(key) = cache_key {
                    GROUPS.with(|groups| {
                        let source_bytes = sources.iter().map(|source| source.xaml.len()).sum();
                        groups.borrow_mut().insert(key, source_bytes, group.clone());
                    });
                }
                group
            }
        };

        let mut loaded = Vec::with_capacity(documents.len());
        for ((document, provider), root_type) in documents.iter().zip(&group.providers).zip(&group.root_types) {
            let name = document.document.clone();
            let result = Self::load_or_populate(
                provider,
                root_type,
                document.root_instance.clone(),
                document.service_provider.clone(),
            );
            loaded.push(result.map_err(|error| match error.document() {
                Some(_) => error,
                None => error.with_document(name),
            })?);
        }
        if timings {
            eprintln!(
                "[xaml] {} document(s), first '{}': {} + interpret, {:?} in all",
                sources.len(),
                sources.first().map_or("", |source| source.name.as_str()),
                if was_cached { "cached" } else { "transformed (see above)" },
                interpret_start.elapsed(),
            );
        }
        Ok(loaded)
    }

    /// Removes the transformed documents this thread keeps (see [`Self::load_group`]).
    pub fn clear_cache() {
        GROUPS.with(|groups| *groups.borrow_mut() = GroupCache::default());
    }

    /// Parses and transforms the documents of a group and prepares their build and
    /// populate methods: everything of `LoadGroup` up to running them. The transformers
    /// see `type_system`, the run-time type system.
    ///
    /// An include of a document of an assembly without compiled markup may stay a
    /// run-time include (`XamlRuntimeIncludeFallback`, xaml.md decision 22). The emitter
    /// of Rust source transforms with the same sequence and without that fallback
    /// (`rust_emitter::transform_group`).
    fn transform_group(
        runtime_type_system: &Rc<RuntimeTypeSystem>,
        type_system: Rc<dyn IXamlTypeSystem>,
        sources: &[DocumentSource],
        configuration: &RuntimeXamlLoaderConfiguration,
    ) -> XamlResult<TransformedGroup> {
        let runtime_type_system = runtime_type_system.clone();

        let (mut mappings, emit_mappings) = FerroXamlIlLanguage::configure(&type_system)?;
        framework::adapt_type_mappings(&mut mappings);

        let assembly: Option<Rc<dyn IXamlAssembly>> = match configuration.local_assembly {
            Some(local) => type_system.assemblies().into_iter().find(|a| a.name() == local.name),
            None => None,
        };

        let diagnostics: Rc<RefCell<Vec<XamlDiagnostic>>> = Rc::new(RefCell::new(Vec::new()));
        let diagnostics_handler = {
            let diagnostics = diagnostics.clone();
            let handler = configuration.diagnostic_handler.clone();
            XamlDiagnosticsHandler {
                handle_diagnostic: Some(Box::new(move |diagnostic: &XamlDiagnostic| {
                    let mut runtime_diagnostic = RuntimeXamlDiagnostic::new(
                        diagnostic.code.clone(),
                        match diagnostic.severity {
                            XamlDiagnosticSeverity::None => RuntimeXamlDiagnosticSeverity::Info,
                            XamlDiagnosticSeverity::Warning => RuntimeXamlDiagnosticSeverity::Warning,
                            XamlDiagnosticSeverity::Error => RuntimeXamlDiagnosticSeverity::Error,
                            XamlDiagnosticSeverity::Fatal => RuntimeXamlDiagnosticSeverity::Fatal,
                        },
                        diagnostic.title.clone(),
                        diagnostic.line_number,
                        diagnostic.line_position,
                    );
                    runtime_diagnostic.document = diagnostic.document.clone();
                    let new_severity = match handler.as_ref().map(|handler| handler(&runtime_diagnostic)) {
                        Some(RuntimeXamlDiagnosticSeverity::Info) => XamlDiagnosticSeverity::None,
                        Some(RuntimeXamlDiagnosticSeverity::Warning) => XamlDiagnosticSeverity::Warning,
                        Some(RuntimeXamlDiagnosticSeverity::Error) => XamlDiagnosticSeverity::Error,
                        Some(RuntimeXamlDiagnosticSeverity::Fatal) => XamlDiagnosticSeverity::Fatal,
                        None => diagnostic.severity,
                    };
                    let mut diagnostic = diagnostic.clone();
                    diagnostic.severity = new_severity;
                    diagnostics.borrow_mut().push(diagnostic);
                    new_severity
                })),
                code_mappings: Box::new(FerroXamlDiagnosticCodes::xaml_x_diagnostic_code_to_ferro),
                ..XamlDiagnosticsHandler::default()
            }
        };

        let compiler_configuration = FerroXamlIlCompilerConfiguration::new(
            type_system.clone(),
            assembly,
            mappings,
            None,
            Some(FerroXamlIlLanguage::value_converter()),
            None,
            Some(diagnostics_handler),
        )?;
        let transformer_configuration = compiler_configuration.as_transformer_configuration().clone();
        framework::configure_configuration(&transformer_configuration);

        let compiler = FerroXamlIlCompiler::new(transformer_configuration.clone())?;
        compiler.set_default_compile_bindings(configuration.use_compiled_bindings_by_default);
        compiler.set_is_design_mode(configuration.design_mode);
        compiler.set_create_source_info(configuration.create_source_info());

        let interpreter =
            framework::create_interpreter(transformer_configuration.clone(), runtime_type_system.clone(), &emit_mappings)?;
        let service_provider_type = transformer_configuration.type_mappings.service_provider()?;
        let void = transformer_configuration.well_known_types().void.clone();

        let mut parsed_documents: Vec<Rc<dyn IXamlDocumentResource>> = Vec::with_capacity(sources.len());
        let mut providers: Vec<Rc<RuntimeDocumentTypeBuilderProvider>> = Vec::with_capacity(sources.len());
        let mut root_types: Vec<Rc<dyn IXamlType>> = Vec::with_capacity(sources.len());

        let timings = std::env::var_os(TIMINGS_VARIABLE).is_some();
        let (mut parse_time, mut transform_time) = (std::time::Duration::ZERO, std::time::Duration::ZERO);
        for document in sources {
            let start = std::time::Instant::now();
            let mut parsed: XamlDocument = compiler.parse(&document.xaml, document.override_type.clone())?;
            parse_time += start.elapsed();
            parsed.document = Some(document.name.clone());
            let start = std::time::Instant::now();
            compiler.transform(&mut parsed)?;
            transform_time += start.elapsed();

            let root = parsed.root()?;
            let root_type = root
                .cast::<dyn IXamlAstValueNode>()
                .ok_or_else(|| {
                    XamlError::invalid_cast(format!(
                        "Unable to cast object of type '{}' to type 'IXamlAstValueNode'.",
                        root.type_name()
                    ))
                })?
                .type_()
                .get_clr_type()?;
            let xaml_name =
                Self::get_safe_uri_identifier(document.base_uri.clone()).unwrap_or_else(|| root_type.name());

            root_types.push(root_type.clone());
            let provider = RuntimeDocumentTypeBuilderProvider::new(
                &format!("Builder_{xaml_name}"),
                root_type,
                service_provider_type.clone(),
                void.clone(),
                document.override_type.is_none(),
            );
            let factory_provider = provider.clone();
            parsed_documents.push(XamlDocumentResource::new(
                Rc::new(RefCell::new(parsed)),
                document.base_uri.clone(),
                None,
                None,
                true,
                Box::new(move || Ok(factory_provider.clone() as Rc<dyn IXamlDocumentTypeBuilderProvider>)),
            ));
            providers.push(provider);
        }

        let start = std::time::Instant::now();
        compiler.transform_group(&parsed_documents)?;
        let group_transform_time = start.elapsed();

        throw_exception_if_any_error(&diagnostics.borrow())?;

        // The counterpart of compiling each document into its builder type.
        for ((resource, provider), source) in parsed_documents.iter().zip(&providers).zip(sources) {
            let xaml_document = resource.xaml_document();
            let xaml_document = xaml_document.borrow();
            let runtime_document = interpreter.document(&xaml_document, resource.uri().as_deref())?;
            #[cfg(all(feature = "hot-reload", debug_assertions))]
            { *runtime_document.source_text.borrow_mut() = Some(Rc::from(source.xaml.as_str())); }
            #[cfg(not(all(feature = "hot-reload", debug_assertions)))]
            let _ = source;
            provider.set_body(DocumentBody {
                interpreter: interpreter.clone(),
                root: xaml_document.root()?,
                document: runtime_document,
            });
        }

        if timings {
            eprintln!(
                "[xaml] {} document(s): parse {parse_time:?}, transform {transform_time:?}, group transform {group_transform_time:?}",
                sources.len(),
            );
        }
        Ok(TransformedGroup { providers, root_types })
    }

    /// `LoadOrPopulate(created, rootInstance, parentServiceProvider)`.
    ///
    /// A document without a root instance whose root type is a class with markup of its
    /// own (upstream: a type with the populate override field of compiled markup; here:
    /// a class with a registered class document, see [`crate::FerroRuntimeXamlLoader`])
    /// is not built by its `Build` method: the instance is created by the constructor of
    /// the class, with the populate of this document in place of the markup of the class.
    fn load_or_populate(
        provider: &Rc<RuntimeDocumentTypeBuilderProvider>,
        root_type: &Rc<dyn IXamlType>,
        root_instance: Option<BoxedValue>,
        parent_service_provider: Option<Rc<dyn IServiceProvider>>,
    ) -> XamlResult<MarkupValue> {
        let service_provider = XamlIlRuntimeHelpers::create_root_service_provider_v3(parent_service_provider);
        match root_instance {
            None => {
                let class = root_type
                    .as_any()
                    .downcast_ref::<crate::runtime::type_system::RuntimeType>()
                    .and_then(crate::runtime::type_system::RuntimeType::type_info)
                    .filter(|class| crate::FerroRuntimeXamlLoader::has_class_document(class));
                if let Some((class, constructor)) = class.and_then(|class| Some((class, class.default_constructor()?))) {
                    let populate = {
                        let provider = provider.clone();
                        let service_provider = service_provider.clone();
                        Rc::new(move |target: &BoxedValue| {
                            let target = Some(crate::runtime::type_system::normalize_object(target.clone()));
                            provider.populate(Some(service_provider.clone()), &target)
                        })
                    };
                    let (created, populated) =
                        crate::FerroRuntimeXamlLoader::with_populate_override(class, populate, constructor);
                    if let Some(Err(error)) = populated {
                        return Err(error);
                    }
                    return Ok(crate::runtime::type_system::box_object(created));
                }
                provider.build(Some(service_provider))
            }
            Some(root_instance) => {
                let root_instance = Some(crate::runtime::type_system::normalize_object(root_instance));
                provider.populate(Some(service_provider), &root_instance)?;
                Ok(root_instance)
            }
        }
    }

    /// `GetSafeUriIdentifier(uri)`.
    fn get_safe_uri_identifier(uri: Option<String>) -> Option<String> {
        uri.map(|uri| uri.replace([':', '/', '?', '=', '.'], "_"))
    }
}

#[cfg(test)]
mod cache_tests {
    use super::*;
    use ferroui_base::{media::SolidColorBrush, Ref};

    fn group() -> Rc<TransformedGroup> {
        Rc::new(TransformedGroup { providers: Vec::new(), root_types: Vec::new() })
    }

    #[test]
    fn cache_evicts_old_revisions_but_keeps_recently_used_groups() {
        let mut cache = GroupCache::default();
        let old = group();
        let old_weak = Rc::downgrade(&old);
        cache.insert("old".into(), MAX_CACHED_SOURCE_BYTES / 2, old);
        let recent = group();
        cache.insert("recent".into(), MAX_CACHED_SOURCE_BYTES / 2, recent.clone());
        assert!(Rc::ptr_eq(&cache.get("recent").unwrap(), &recent));
        cache.insert("new".into(), 1, group());
        assert!(old_weak.upgrade().is_none());
        assert!(cache.get("recent").is_some());
        assert!(cache.get("new").is_some());
        let oversized = group();
        let oversized_weak = Rc::downgrade(&oversized);
        cache.insert("oversized".into(), MAX_CACHED_SOURCE_BYTES + 1, oversized);
        assert!(oversized_weak.upgrade().is_none());
        assert_eq!(cache.entries.len(), 2);

        let mut cache = GroupCache::default();
        for index in 0..MAX_CACHED_GROUPS {
            cache.insert(index.to_string(), 1, group());
        }
        cache.get("0").unwrap();
        cache.insert("next".into(), 1, group());
        assert_eq!(cache.entries.len(), MAX_CACHED_GROUPS);
        assert!(cache.get("0").is_some());
        assert!(cache.get("1").is_none());
    }

    #[test]
    fn cached_documents_build_fresh_objects_and_invalidate_the_whole_group() {
        FerroXamlIlRuntimeCompiler::clear_cache();
        let load = |color: &str| {
            let documents = ["Red", color].into_iter().map(|color| {
                RuntimeXamlLoaderDocument::from_stream_with_base_uri_and_root_instance(
                    None, None, Box::new(std::io::Cursor::new(format!(
                        "<SolidColorBrush xmlns='https://github.com/ferroui' Color='{color}'/>"
                    ).into_bytes())),
                )
            }).collect();
            FerroXamlIlRuntimeCompiler::load_group(documents, &RuntimeXamlLoaderConfiguration::default())
        };
        let first = load("Green").expect("first transform");
        let brush = ferroui_base::metadata::from_markup_value::<Ref<SolidColorBrush>>(&first[0]).unwrap();
        let weak = brush.downgrade();
        let first_group = GROUPS.with(|cache| cache.borrow().entries.back().unwrap().2.clone());
        let second = load("Green").expect("cached transform");
        let second_brush = ferroui_base::metadata::from_markup_value::<Ref<SolidColorBrush>>(&second[0]).unwrap();
        assert!(!brush.ptr_eq(&second_brush));
        assert!(GROUPS.with(|cache| Rc::ptr_eq(&first_group, &cache.borrow().entries.back().unwrap().2)));
        drop(brush);
        drop(first);
        assert!(weak.upgrade().is_none(), "cached transforms must not retain built objects");
        let changed = load("Blue").expect("changed dependency text");
        let previous_color = ferroui_base::metadata::from_markup_value::<Ref<SolidColorBrush>>(&second[1]).unwrap().color();
        let next_color = ferroui_base::metadata::from_markup_value::<Ref<SolidColorBrush>>(&changed[1]).unwrap().color();
        assert_ne!(previous_color, next_color);
        assert!(GROUPS.with(|cache| !Rc::ptr_eq(&first_group, &cache.borrow().entries.back().unwrap().2)));
        let entries = GROUPS.with(|cache| cache.borrow().entries.len());
        assert!(load("not-a-color").is_err());
        assert_eq!(GROUPS.with(|cache| cache.borrow().entries.len()), entries);
        FerroXamlIlRuntimeCompiler::clear_cache();
    }
}
