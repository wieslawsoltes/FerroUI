//! The transform of a group of documents for the emitter of Rust source:
//! parse, the transformer pipeline and the group transformers, exactly the
//! sequence of the run-time loader (`FerroXamlIlRuntimeCompiler::load_group`,
//! upstream's `LoadGroup`) up to the back end, against ANY type system.
//!
//! Nothing is interpreted and nothing of the process is asked: the types are
//! the ones `type_system` has, so the same function transforms against the
//! run-time type system (a host that links the framework) and against the
//! build-time type system over the models of the crates (`ferroui-build`,
//! `ModelTypeSystem`; docs/porting/xaml.md, 9.5).
//!
//! What the run-time loader adds to a configuration for its back end and the
//! emitter does not take over: the include of a document of an assembly
//! without compiled markup never stays a run-time include (xaml.md decision
//! 22), so that the output does not depend on what the process that
//! generates it can load.

use std::cell::RefCell;
use std::rc::Rc;

use ferroui_base::utilities::Uri;
use xamlx::ast::XamlAstExtensions as _;
use xamlx::ast::XamlAstNodeExtensions as _;
use xamlx::ast::{IXamlAstNode, IXamlAstValueNode, XamlDocument};
use xamlx::diagnostics::{throw_exception_if_any_error, XamlDiagnostic, XamlDiagnosticSeverity};
use xamlx::exceptions::{XamlError, XamlResult};
use xamlx::transform::{TransformerConfiguration, XamlDiagnosticsHandler};
use xamlx::type_system::{IXamlAssembly, IXamlMethod, IXamlType, IXamlTypeSystem};

use crate::compiler_extensions::group_transformers::XamlRuntimeIncludeFallback;
use crate::compiler_extensions::{
    FerroXamlDiagnosticCodes, FerroXamlIlCompiler, FerroXamlIlCompilerConfiguration, FerroXamlIlLanguage, IXamlCompileTimeValueParser,
    IXamlDocumentResource, IXamlDocumentTypeBuilderProvider, XamlCompileTimeValueParsers, XamlDocumentResource,
};
use crate::back_end::{adapt_type_mappings, DocumentTypeBuilderProvider, XmlNamespaceInfoProvider};

use super::source::rust_string_literal;

/// A document of a group to transform.
pub struct DocumentSource<'s> {
    /// The name of the document: its path below the root URI of the assembly, and its
    /// name in diagnostics.
    pub name: &'s str,
    pub xaml: &'s str,
    /// The URI the document is loaded by.
    pub base_uri: Option<String>,
    /// The type of the root instance the document populates (`x:Class`); a document
    /// without one builds its root object.
    pub root_type: Option<Rc<dyn IXamlType>>,
}

/// Receives a diagnostic of a transform and returns the severity it has from then on.
pub type DiagnosticHandler = Rc<dyn Fn(&XamlDiagnostic) -> XamlDiagnosticSeverity>;

/// What a transform depends on beside the documents and the type system.
#[derive(Clone, Default)]
pub struct TransformOptions {
    /// The name of the assembly the documents belong to: names without an assembly
    /// resolve against it first.
    pub local_assembly: Option<String>,
    /// Whether `{Binding}` is a compiled binding unless the document says otherwise.
    pub use_compiled_bindings_by_default: bool,
    pub design_mode: bool,
    /// Whether source information is attached to the objects the documents create.
    pub create_source_info: bool,
    /// Receives the diagnostics of the transform and may change their severity.
    pub diagnostic_handler: Option<DiagnosticHandler>,
}

#[cfg(any(feature = "emitter", all(test, feature = "runtime")))]
impl TransformOptions {
    /// The options of the configuration of the run-time loader. The diagnostic handler of
    /// the configuration sees a diagnostic as the run-time loader gives it to the handler.
    pub fn of(configuration: &ferroui_markup_xaml::RuntimeXamlLoaderConfiguration) -> Self {
        use ferroui_markup_xaml::{RuntimeXamlDiagnostic, RuntimeXamlDiagnosticSeverity};
        let diagnostic_handler = configuration.diagnostic_handler.clone().map(|handler| {
            Rc::new(move |diagnostic: &XamlDiagnostic| {
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
                match handler(&runtime_diagnostic) {
                    RuntimeXamlDiagnosticSeverity::Info => XamlDiagnosticSeverity::None,
                    RuntimeXamlDiagnosticSeverity::Warning => XamlDiagnosticSeverity::Warning,
                    RuntimeXamlDiagnosticSeverity::Error => XamlDiagnosticSeverity::Error,
                    RuntimeXamlDiagnosticSeverity::Fatal => XamlDiagnosticSeverity::Fatal,
                }
            }) as DiagnosticHandler
        });
        Self {
            local_assembly: configuration.local_assembly.map(|assembly| assembly.name.to_string()),
            use_compiled_bindings_by_default: configuration.use_compiled_bindings_by_default,
            design_mode: configuration.design_mode,
            create_source_info: configuration.create_source_info(),
            diagnostic_handler,
        }
    }
}

/// A transformed document: the input of the emitter.
pub struct TransformedDocument {
    /// The transformed root node.
    pub root: Rc<dyn IXamlAstNode>,
    /// The configuration the document was transformed with.
    pub configuration: Rc<TransformerConfiguration>,
    /// The base URI the contexts of the document get, as it was given.
    pub base_uri: Option<String>,
    /// The namespace information the contexts of the document get, as the value of an
    /// `rt::XmlNamespaceTable` ([`namespace_table`]); nothing when the language maps no
    /// namespace information provider.
    pub namespaces: Option<String>,
    /// The build method of the document in its group (what a call from another
    /// document of the group calls).
    pub build: Option<Rc<dyn IXamlMethod>>,
}

/// Parses and transforms `sources` as ONE group against `type_system` (the group
/// transformers see every document: the includes between them are resolved as upstream's
/// build resolves the documents of an assembly) and returns each transformed document, in
/// order. `parsers` are the compile-time value parsers of the host.
pub fn transform_group(
    type_system: Rc<dyn IXamlTypeSystem>,
    sources: &[DocumentSource<'_>],
    options: &TransformOptions,
    parsers: &[Rc<dyn IXamlCompileTimeValueParser>],
) -> XamlResult<Vec<TransformedDocument>> {
    let (mut mappings, emit_mappings) = FerroXamlIlLanguage::configure(&type_system)?;
    adapt_type_mappings(&mut mappings);

    let assembly: Option<Rc<dyn IXamlAssembly>> = match &options.local_assembly {
        Some(local) => type_system.assemblies().into_iter().find(|a| a.name() == *local),
        None => None,
    };

    let diagnostics: Rc<RefCell<Vec<XamlDiagnostic>>> = Rc::new(RefCell::new(Vec::new()));
    let diagnostics_handler = {
        let diagnostics = diagnostics.clone();
        let handler = options.diagnostic_handler.clone();
        XamlDiagnosticsHandler {
            handle_diagnostic: Some(Box::new(move |diagnostic: &XamlDiagnostic| {
                let new_severity = handler.as_ref().map_or(diagnostic.severity, |handler| handler(diagnostic));
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
    let value_parsers = transformer_configuration.get_or_create_extra::<XamlCompileTimeValueParsers>();
    for parser in parsers {
        value_parsers.add(parser.clone());
    }
    transformer_configuration.get_or_create_extra::<XamlRuntimeIncludeFallback>().set(None);

    let compiler = FerroXamlIlCompiler::new(transformer_configuration.clone())?;
    compiler.set_default_compile_bindings(options.use_compiled_bindings_by_default);
    compiler.set_is_design_mode(options.design_mode);
    compiler.set_create_source_info(options.create_source_info);

    // The members the language adds to the context must be describable, as for the back
    // end that defines them.
    emit_mappings.context_type_builder_callback(&transformer_configuration.type_mappings)?;
    let service_provider_type = transformer_configuration.type_mappings.service_provider()?;
    let void = transformer_configuration.well_known_types().void.clone();

    let mut parsed_documents: Vec<Rc<dyn IXamlDocumentResource>> = Vec::with_capacity(sources.len());
    let mut providers: Vec<Rc<DocumentTypeBuilderProvider>> = Vec::with_capacity(sources.len());
    for document in sources {
        let mut parsed: XamlDocument = compiler.parse(document.xaml, document.root_type.clone())?;
        parsed.document = Some(document.name.to_string());
        compiler.transform(&mut parsed)?;

        let root = parsed.root()?;
        let root_type = root
            .cast::<dyn IXamlAstValueNode>()
            .ok_or_else(|| {
                XamlError::invalid_cast(format!("Unable to cast object of type '{}' to type 'IXamlAstValueNode'.", root.type_name()))
            })?
            .type_()
            .get_clr_type()?;
        let xaml_name = document.base_uri.as_ref().map(|uri| uri.replace([':', '/', '?', '=', '.'], "_")).unwrap_or_else(|| root_type.name());
        let provider = DocumentTypeBuilderProvider::new(
            &format!("Builder_{xaml_name}"),
            root_type,
            service_provider_type.clone(),
            void.clone(),
            document.root_type.is_none(),
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

    compiler.transform_group(&parsed_documents)?;
    throw_exception_if_any_error(&diagnostics.borrow())?;

    let has_namespace_information = transformer_configuration.type_mappings.xml_namespace_info_provider.is_some();
    let mut transformed = Vec::with_capacity(sources.len());
    for (resource, provider) in parsed_documents.iter().zip(&providers) {
        let xaml_document = resource.xaml_document();
        let xaml_document = xaml_document.borrow();
        let base_uri = resource.uri();
        if let Some(base_uri) = base_uri.as_deref() {
            Uri::absolute(base_uri).map_err(|e| XamlError::internal("UriFormatException", format!("Invalid base URI '{base_uri}': {e}")))?;
        }
        transformed.push(TransformedDocument {
            root: xaml_document.root()?,
            configuration: transformer_configuration.clone(),
            base_uri,
            namespaces: has_namespace_information.then(|| namespace_table(&transformer_configuration, &xaml_document)),
            build: provider.build_method(),
        });
    }
    Ok(transformed)
}

/// The namespace information of a document as the value of an `rt::XmlNamespaceTable`
/// (the argument of `rt::create_context`): what the namespace information provider of
/// the document gives the contexts of the run-time loader, prefixes in order, one per
/// line.
pub fn namespace_table(configuration: &Rc<TransformerConfiguration>, document: &XamlDocument) -> String {
    let namespaces = XmlNamespaceInfoProvider::new(configuration.clone(), document).xml_namespaces();
    let mut prefixes: Vec<&String> = namespaces.keys().collect();
    prefixes.sort();
    let entries: Vec<String> = prefixes
        .into_iter()
        .map(|prefix| {
            let infos: Vec<String> = namespaces[prefix]
                .iter()
                .map(|info| {
                    format!(
                        "({}, {})",
                        rust_string_literal(&info.clr_namespace),
                        rust_string_literal(info.clr_assembly_name.as_deref().unwrap_or(""))
                    )
                })
                .collect();
            format!("    ({}, &[{}]),\n", rust_string_literal(prefix), infos.join(", "))
        })
        .collect();
    format!("&[\n{}]", entries.concat())
}
