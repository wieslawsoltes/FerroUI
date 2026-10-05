//! The run-time context of a document being built: the context itself is
//! the one of the runtime library ([`XamlIlContext`], used by generated
//! code too, docs/porting/xaml.md 9.12 ruling 7); this module adds what only
//! the interpreter needs: the definition of the context from the
//! transformer configuration, the service adapter with the namespace
//! information and the context value, and the namespace information
//! provider of a parsed document (`IL/NamespaceInfoProvider.cs`).

use std::any::{Any, TypeId};
use std::cell::OnceCell;
use std::collections::HashMap;
use std::rc::Rc;

use ferroui_base::metadata::IServiceProvider;
use ferroui_base::BoxedValue;
use xamlx::ast::XamlDocument;
use xamlx::transform::{NamespaceInfoHelper, TransformerConfiguration};
use xamlx::type_system::IXamlType;

use crate::runtime::type_system::ITypeDescriptorContext;

pub use ferroui_markup_xaml::xaml_il::runtime::{
    IStaticServiceProvider, IXamlIlContextServices, XamlIlContext as RuntimeContext,
    XamlIlContextDefinition as RuntimeContextDefinition, XamlIlContextService as RuntimeContextService,
};

/// What the language configured the context to be: the type mappings of
/// the transformer configuration that are set.
pub fn context_definition(configuration: &TransformerConfiguration) -> RuntimeContextDefinition {
    let mappings = &configuration.type_mappings;
    RuntimeContextDefinition {
        root_object_provider: mappings.root_object_provider.is_some(),
        parent_stack_provider: mappings.parent_stack_provider.is_some(),
        type_descriptor_context: mappings.type_descriptor_context.is_some(),
        provide_value_target: mappings.provide_value_target.is_some(),
        uri_context_provider: mappings.uri_context_provider.is_some(),
        xml_namespace_info_provider: mappings.xml_namespace_info_provider.is_some(),
    }
}

/// The service adapter of the interpreter: the one of the context
/// ([`IXamlIlContextServices`]), plus the namespace information of a parsed
/// document as a service and the context as a value of a type of the
/// transformed tree.
///
/// The service contracts live in the markup runtime crate; the interpreter
/// only knows them through this adapter.
/// [`DefaultRuntimeContextServices`](super::DefaultRuntimeContextServices)
/// maps onto the self-contained contracts of [`services`](super::services).
pub trait IRuntimeContextServices: IXamlIlContextServices {
    /// The namespace information of the document as a service, if
    /// `service_type` is the handle type of the namespace information
    /// provider.
    fn get_namespace_info_service(
        &self,
        provider: &Rc<XmlNamespaceInfoProvider>,
        service_type: TypeId,
    ) -> Option<Rc<dyn Any>>;

    /// The context as a value of the type `type_` (the service provider
    /// type, or the type descriptor context type): what a node that loads
    /// the context evaluates to.
    fn context_value(&self, context: &Rc<RuntimeContext>, type_: &dyn IXamlType) -> BoxedValue {
        if type_.is("System.ComponentModel", "ITypeDescriptorContext") {
            let value: Rc<dyn ITypeDescriptorContext> = context.clone();
            Rc::new(value)
        } else {
            let value: Rc<dyn IServiceProvider> = context.clone();
            Rc::new(value)
        }
    }
}

/// The CLR namespace an XML namespace prefix of a document maps to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct XamlXmlNamespaceInfo {
    pub clr_namespace: String,
    pub clr_assembly_name: Option<String>,
}

/// The namespace information of a document: for each XML namespace prefix
/// the namespaces (and assemblies) it resolves to. Created lazily, once per
/// document.
pub struct XmlNamespaceInfoProvider {
    configuration: Rc<TransformerConfiguration>,
    aliases: Vec<(String, String)>,
    namespaces: OnceCell<Rc<HashMap<String, Vec<XamlXmlNamespaceInfo>>>>,
}

impl XmlNamespaceInfoProvider {
    pub fn new(configuration: Rc<TransformerConfiguration>, document: &XamlDocument) -> Rc<Self> {
        Rc::new(Self {
            configuration,
            aliases: document.namespace_aliases.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            namespaces: OnceCell::new(),
        })
    }

    /// The XML namespace prefixes of the document and what they resolve to.
    pub fn xml_namespaces(&self) -> Rc<HashMap<String, Vec<XamlXmlNamespaceInfo>>> {
        self.namespaces
            .get_or_init(|| {
                let mut namespaces = HashMap::with_capacity(self.aliases.len());
                for (alias, xmlns) in &self.aliases {
                    let resolved = NamespaceInfoHelper::try_resolve(&self.configuration, Some(xmlns.as_str())).unwrap_or_default();
                    let infos = resolved
                        .into_iter()
                        .map(|r| XamlXmlNamespaceInfo {
                            clr_assembly_name: r.assembly_name.or_else(|| r.assembly.map(|a| a.name())),
                            clr_namespace: r.clr_namespace,
                        })
                        .collect();
                    namespaces.insert(alias.clone(), infos);
                }
                Rc::new(namespaces)
            })
            .clone()
    }
}

struct NamespaceInfoStaticProvider {
    provider: Rc<XmlNamespaceInfoProvider>,
    services: Rc<dyn IRuntimeContextServices>,
}

impl IStaticServiceProvider for NamespaceInfoStaticProvider {
    fn get_static_service(&self, service_type: TypeId) -> Option<Rc<dyn Any>> {
        self.services.get_namespace_info_service(&self.provider, service_type)
    }
}

/// The namespace information provider of a document as a static provider.
pub fn namespace_info_static_provider(
    provider: Rc<XmlNamespaceInfoProvider>,
    services: Rc<dyn IRuntimeContextServices>,
) -> Rc<dyn IStaticServiceProvider> {
    Rc::new(NamespaceInfoStaticProvider { provider, services })
}

impl ITypeDescriptorContext for RuntimeContext {}
