//! The end-to-end harness of the interpreter tests and the port of the
//! run-time tests of the compiler library (`tests/XamlParserTests/*.cs`):
//! XAML text is parsed, transformed by the standard pipeline over the
//! run-time type system and interpreted; assertions are made on the live
//! objects.
//!
//! The test classes ([`classes`]) are the counterparts of the upstream test
//! classes, declared with the metadata macros. The type names are unique in
//! the crate: the tables of known types are process-wide.

pub(crate) mod classes;
mod compiler_tests;
mod dynamic_setters_tests;
mod markup_extension_tests;

use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use ferroui_base::metadata::{from_markup_value, IServiceProvider, MarkupValue};
use ferroui_base::BoxedValue;
use xamlx::ast::IXamlAstNode;
use xamlx::compiler::XamlImperativeCompiler;
use xamlx::diagnostics::{throw_exception_if_any_error, XamlDiagnostic};
use xamlx::emit::{IXamlEmitResult, XamlLanguageEmitMappings};
use xamlx::exceptions::{XamlError, XamlResult};
use xamlx::parsers::XDocumentXamlParser;
use xamlx::transform::{
    IXamlAstTransformer, TransformerConfiguration, XamlDiagnosticsHandler, XamlLanguageTypeMappings,
};
use xamlx::type_system::{IXamlType, IXamlTypeSystem};

use crate::runtime::type_system::RuntimeTypeSystem;

use super::services::SERVICES_NAMESPACE;
use super::{Interpreter, RuntimeDocument};

pub(crate) const NS: &str = "RtXamlParserTests";
pub(crate) const X: &str = "http://schemas.microsoft.com/winfx/2006/xaml";

struct NoEmitter;
struct NoEmitResult;

impl IXamlEmitResult for NoEmitResult {
    fn return_type(&self) -> Option<Rc<dyn IXamlType>> {
        None
    }
    fn valid(&self) -> bool {
        true
    }
}

/// How a [`TestHost`] differs from the upstream `CompilerTestBase`.
#[derive(Default)]
pub(crate) struct HostOptions {
    /// The name of the static method of `DeferredContentTests` that
    /// customises deferred content.
    pub deferred_customization: Option<&'static str>,
    /// Maps the inner service provider factory of `ServiceProviderTests`.
    pub inner_provider_factory: bool,
    /// A transformer inserted after the property reference resolver.
    pub transformer: Option<fn() -> Box<dyn IXamlAstTransformer>>,
}

/// The counterpart of the upstream `CompilerTestBase`: the test language
/// over the run-time type system, a compiler and an interpreter.
pub(crate) struct TestHost {
    #[allow(dead_code)]
    pub ts: Rc<RuntimeTypeSystem>,
    pub configuration: Rc<TransformerConfiguration>,
    pub interpreter: Rc<Interpreter>,
    diagnostics: Rc<RefCell<Vec<XamlDiagnostic>>>,
    transformer: Option<fn() -> Box<dyn IXamlAstTransformer>>,
}

impl TestHost {
    pub fn new() -> Self {
        Self::with_options(HostOptions::default())
    }

    pub fn with_options(options: HostOptions) -> Self {
        classes::register();
        let ts = RuntimeTypeSystem::new();
        let t = |name: &str| ts.find_type(&format!("{NS}.{name}")).unwrap_or_else(|| panic!("type {name}"));
        let service = |name: &str| ts.find_type(&format!("{SERVICES_NAMESPACE}.{name}")).expect("service type");
        let mut mappings = XamlLanguageTypeMappings::new(&*ts).expect("default mappings");
        mappings.xmlns_attributes.push(ts.attribute_type("XmlnsDefinition"));
        mappings.content_attributes.push(ts.attribute_type("Content"));
        mappings.whitespace_significant_collection_attributes.push(ts.attribute_type("WhitespaceSignificantCollection"));
        mappings.trim_surrounding_whitespace_attributes.push(ts.attribute_type("TrimSurroundingWhitespace"));
        mappings.usable_during_initialization_attributes.push(ts.attribute_type("UsableDuringInitialization"));
        mappings.deferred_content_property_attributes.push(ts.attribute_type("DeferredContent"));
        mappings.root_object_provider = Some(service("IRootObjectProvider"));
        mappings.uri_context_provider = Some(service("IUriContext"));
        mappings.provide_value_target = Some(service("IProvideValueTarget"));
        mappings.parent_stack_provider = Some(service("IXamlParentStackProvider"));
        mappings.xml_namespace_info_provider = Some(service("IXamlXmlNamespaceInfoProvider"));
        mappings.support_initialize = Some(t("ISupportInitialize"));
        mappings.i_add_child = Some(t("IAddChild"));
        mappings.i_add_child_of_t = Some(t("IAddChild`1"));
        if let Some(name) = options.deferred_customization {
            let method = t("DeferredContentTests").get_method(|m| m.name() == name).expect("customizer");
            mappings.deferred_content_executor_customization = Some(method);
        }
        if options.inner_provider_factory {
            let method = t("ServiceProviderTests").find_method(|m| m.name() == "InnerProviderFactory");
            mappings.inner_service_provider_factory_method = method;
        }

        let diagnostics: Rc<RefCell<Vec<XamlDiagnostic>>> = Rc::new(RefCell::new(Vec::new()));
        let collected = diagnostics.clone();
        let handler = XamlDiagnosticsHandler {
            handle_diagnostic: Some(Box::new(move |diagnostic| {
                collected.borrow_mut().push(diagnostic.clone());
                diagnostic.severity
            })),
            ..XamlDiagnosticsHandler::default()
        };
        let type_system = ts.as_type_system();
        let configuration = Rc::new(
            TransformerConfiguration::new(
                type_system.clone(),
                type_system.find_assembly(NS),
                mappings,
                None,
                None,
                None,
                Some(handler),
            )
            .expect("configuration"),
        );
        let interpreter = Rc::new(Interpreter::new(configuration.clone(), ts.clone()));
        Self { ts, configuration, interpreter, diagnostics, transformer: options.transformer }
    }

    /// The upstream `Compile`: parses and transforms a document.
    pub fn compile(&self, xaml: &str) -> XamlResult<(Rc<dyn IXamlAstNode>, Rc<RuntimeDocument>)> {
        let mut document = XDocumentXamlParser::parse(xaml, None)?;
        let mut compiler: XamlImperativeCompiler<NoEmitter, NoEmitResult> =
            XamlImperativeCompiler::new(self.configuration.clone(), Rc::new(XamlLanguageEmitMappings::new()), true);
        if let Some(transformer) = self.transformer {
            // After the property reference resolver.
            compiler.base.transformers.insert(7, transformer());
        }
        compiler.transform(&mut document)?;
        throw_exception_if_any_error(&self.diagnostics.borrow())?;
        let runtime_document = self.interpreter.document(&document, Some("http://example.com/"))?;
        Ok((document.root()?, runtime_document))
    }

    /// The upstream `CompileAndRun`: `Build(prov)`.
    pub fn build(&self, xaml: &str, provider: Option<Rc<dyn IServiceProvider>>) -> XamlResult<MarkupValue> {
        let (root, document) = self.compile(xaml)?;
        self.interpreter.build(&root, provider, &document)
    }

    pub fn run<T: Clone + 'static>(&self, xaml: &str) -> T {
        self.run_with(xaml, None)
    }

    pub fn run_with<T: Clone + 'static>(&self, xaml: &str, provider: Option<Rc<dyn IServiceProvider>>) -> T {
        match self.build(xaml, provider) {
            Ok(value) => get::<T>(&value),
            Err(e) => panic!("{}: {}", e.type_name(), e.message()),
        }
    }

    /// The upstream `CompileAndPopulate`: `Populate(prov, instance)`.
    pub fn populate(
        &self,
        xaml: &str,
        provider: Option<Rc<dyn IServiceProvider>>,
        instance: &MarkupValue,
    ) -> XamlResult<()> {
        let (root, document) = self.compile(xaml)?;
        self.interpreter.populate(&root, provider, instance, &document)
    }

    /// The error building `xaml` fails with.
    pub fn error(&self, xaml: &str, provider: Option<Rc<dyn IServiceProvider>>) -> XamlError {
        match self.build(xaml, provider) {
            Ok(_) => panic!("the document was expected to fail"),
            Err(e) => e,
        }
    }
}

/// The value as a `T`.
pub(crate) fn get<T: Clone + 'static>(value: &MarkupValue) -> T {
    from_markup_value::<T>(value).unwrap_or_else(|| {
        panic!(
            "expected a {}, got {}",
            std::any::type_name::<T>(),
            value.as_ref().map(|v| v.type_name()).unwrap_or("null")
        )
    })
}

pub(crate) fn boxed<T: PartialEq + 'static>(value: T) -> MarkupValue {
    let value: BoxedValue = Rc::new(value);
    Some(value)
}

/// The counterpart of the upstream `DictionaryServiceProvider`.
#[derive(Default)]
pub(crate) struct TestServiceProvider {
    services: RefCell<HashMap<TypeId, Rc<dyn Any>>>,
    parent: RefCell<Option<Rc<dyn IServiceProvider>>>,
}

impl TestServiceProvider {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// Adds the service used through the handle type `T`.
    pub fn with<T: 'static>(self: Rc<Self>, service: T) -> Rc<Self> {
        self.services.borrow_mut().insert(TypeId::of::<T>(), Rc::new(service));
        self
    }

    pub fn with_parent(self: Rc<Self>, parent: Option<Rc<dyn IServiceProvider>>) -> Rc<Self> {
        *self.parent.borrow_mut() = parent;
        self
    }
}

impl IServiceProvider for TestServiceProvider {
    fn get_service(&self, service_type: TypeId) -> Option<Rc<dyn Any>> {
        if let Some(service) = self.services.borrow().get(&service_type) {
            return Some(service.clone());
        }
        let parent = self.parent.borrow().clone();
        parent?.get_service(service_type)
    }
}
