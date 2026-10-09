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
        let type_system = type_system.clone();
        // The documents, for the diagnostic of a method name no method answers to.
        let documents: Vec<(String, String, Option<String>)> = sources
            .iter()
            .map(|source| (source.name.to_string(), source.xaml.to_string(), source.root_type.as_ref().map(|class| class.full_name())))
            .collect();
        XamlDiagnosticsHandler {
            handle_diagnostic: Some(Box::new(move |diagnostic: &XamlDiagnostic| {
                let described = method_not_found(&type_system, &documents, diagnostic);
                let diagnostic = described.as_ref().unwrap_or(diagnostic);
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
        compiler.transform(&mut parsed).map_err(|error| described_error(&diagnostics, error))?;

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

/// The error of a transform as the diagnostic that describes it, when the handler of the
/// diagnostics described the diagnostic of the error ([`method_not_found`]).
fn described_error(diagnostics: &RefCell<Vec<XamlDiagnostic>>, error: XamlError) -> XamlError {
    let described = diagnostics
        .borrow()
        .iter()
        .rev()
        .find(|diagnostic| {
            diagnostic.title.starts_with(METHOD_NOT_FOUND)
                && diagnostic.line_number == error.line_number()
                && diagnostic.line_position == error.line_position()
        })
        .map(|diagnostic| diagnostic.to_exception());
    described.unwrap_or(error)
}

/// How the diagnostic of a method name that no method of the root object answers to starts
/// ([`method_not_found`]); the name of the method, the member it is named for and the
/// diagnostic of the compiler follow.
pub const METHOD_NOT_FOUND: &str = "No method `";

/// The diagnostic of the compiler for a method name assigned to an event or to a property
/// of a delegate type when no method of the root object fits, with what a build needs to
/// name: the method and the member it is named for (the document and the position are the
/// ones of the diagnostic).
///
/// The compiler reports such an assignment as any assignment no setter takes (`Unable to
/// find suitable setter or adder for property Click ... for argument System.String`,
/// upstream's text, which the run-time loader gives unchanged): the name of the method is
/// a text to it. Here the text is read back from the document at the position of the
/// diagnostic, and the diagnostic is described as the missing method when a setter of the
/// member takes a delegate. `None` for any other diagnostic.
fn method_not_found(
    type_system: &Rc<dyn IXamlTypeSystem>,
    documents: &[(String, String, Option<String>)],
    diagnostic: &XamlDiagnostic,
) -> Option<XamlDiagnostic> {
    use xamlx::ast::{IXamlAstVisitor, IXamlLineInfo, XamlAstNamePropertyReference, XamlAstTextNode, XamlAstXamlPropertyValueNode};

    let rest = diagnostic.title.strip_prefix("Unable to find suitable setter or adder for property ")?;
    let (member, rest) = rest.split_once(" of type ")?;
    let (_, rest) = rest.split_once(" for argument ")?;
    let (argument, lists) = rest.split_once(", available setter parameter lists are:\n")?;
    if !argument.ends_with("System.String") {
        return None;
    }
    // A setter of the member takes one delegate: an event, or a property of a delegate type
    // (a registered one also has the setters of every registered property, which take a
    // binding and the unset value).
    let delegate = type_system.find_type("System.Delegate")?;
    let is_delegate = |name: &str| {
        let definition = name.split('[').next().unwrap_or(name);
        type_system.find_type(definition).is_some_and(|found| delegate.is_assignable_from(&*found))
    };
    let lists: Vec<&str> = lists
        .lines()
        .map(|list| list.split(" Line ").next().unwrap_or(list).trim())
        .filter(|list| !list.is_empty() && !list.contains(", ") && is_delegate(list))
        .collect();
    if lists.is_empty() {
        return None;
    }

    let name = diagnostic.document.as_deref()?;
    let (_, xaml, class) = documents.iter().find(|(document, _, _)| document == name)?;
    let (line, position) = (diagnostic.line_number?, diagnostic.line_position?);

    /// Finds the text assigned to the member at a position.
    struct Assigned<'m> {
        member: &'m str,
        line: i32,
        position: i32,
        found: Option<String>,
    }
    impl IXamlAstVisitor for Assigned<'_> {
        fn visit(&mut self, node: Rc<dyn IXamlAstNode>) -> XamlResult<Rc<dyn IXamlAstNode>> {
            if let Some(assignment) = node.cast::<XamlAstXamlPropertyValueNode>() {
                let named = assignment.property.borrow().cast::<XamlAstNamePropertyReference>().is_some_and(|property| property.name() == self.member);
                if let ([value], true) = (assignment.values.borrow().as_slice(), named) {
                    if let Some(text) = value.cast::<XamlAstTextNode>() {
                        if text.line() == self.line && text.position() == self.position {
                            self.found = Some(text.text());
                        }
                    }
                }
            }
            Ok(node)
        }
        fn push(&mut self, _node: Rc<dyn IXamlAstNode>) {}
        fn pop(&mut self) {}
    }
    let parsed = xamlx::parsers::XDocumentXamlParser::parse(xaml, None).ok()?;
    let mut assigned = Assigned { member, line, position, found: None };
    xamlx::ast::visit_node(&parsed.root().ok()?, &mut assigned).ok()?;
    let method = assigned.found?;

    let owner = match class {
        Some(class) => format!(
            "the class `{class}` of the document has no method of that name that the delegate can call (a method the markup metadata \
             of the class declares, `methods: [fn {method}(..) => ..]`, with the parameters of the delegate or wider ones)"
        ),
        None => "the document has no class (`x:Class`), and the type of its root object has no method of that name that the delegate can call"
            .to_string(),
    };
    // The text of the compiler follows, without the place it ends with: the diagnostic has it.
    let place = format!(" Line {line}, position {position}.");
    let original = diagnostic.title.replace(['\r', '\n'], " ");
    let mut described = diagnostic.clone();
    described.title = format!(
        "{METHOD_NOT_FOUND}{method}` for `{member}` ({}): {owner}. {}",
        lists.join(" or "),
        original.strip_suffix(&place).unwrap_or(&original)
    );
    // The error of the diagnostic is the described one, not the error it was made from.
    described.inner_exception = None;
    Some(described)
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
