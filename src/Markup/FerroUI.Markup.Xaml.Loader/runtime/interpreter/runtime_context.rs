//! The run-time context of a document being built: the equivalent of the
//! context class the IL back end of the managed original generates
//! (`IL/RuntimeContext.cs`) and of its namespace information provider
//! (`IL/NamespaceInfoProvider.cs`).
//!
//! The context is the service provider markup extensions, type converters
//! and constructors receive. It resolves its services in the order of the
//! generated `GetService`: the inner service provider, its own services
//! (root object, parent stack, type descriptor context, provide-value
//! target, URI context), the static providers of the document, and last the
//! parent service provider.

use std::any::{Any, TypeId};
use std::cell::{OnceCell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};

use ferroui_base::metadata::{IServiceProvider, MarkupValue};
use ferroui_base::utilities::Uri;
use ferroui_base::BoxedValue;
use xamlx::ast::XamlDocument;
use xamlx::transform::{NamespaceInfoHelper, TransformerConfiguration};
use xamlx::type_system::IXamlType;

use crate::runtime::type_system::ITypeDescriptorContext;

/// The services a context provides itself, in the order they are checked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeContextService {
    RootObjectProvider,
    ParentStackProvider,
    TypeDescriptorContext,
    ProvideValueTarget,
    UriContext,
}

/// What the language configured the context to be: which services it
/// implements (the type mappings of the transformer configuration that
/// are set).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RuntimeContextDefinition {
    /// `RootObjectProvider` is mapped.
    pub root_object_provider: bool,
    /// `ParentStackProvider` is mapped: the context keeps the parent stack.
    pub parent_stack_provider: bool,
    /// `TypeDescriptorContext` is mapped.
    pub type_descriptor_context: bool,
    /// `ProvideValueTarget` is mapped: the context records the target object
    /// and target property.
    pub provide_value_target: bool,
    /// `UriContextProvider` is mapped.
    pub uri_context_provider: bool,
    /// `XmlNamespaceInfoProvider` is mapped: a document has a namespace
    /// information provider among its static providers.
    pub xml_namespace_info_provider: bool,
}

impl RuntimeContextDefinition {
    pub fn new(configuration: &TransformerConfiguration) -> Self {
        let mappings = &configuration.type_mappings;
        Self {
            root_object_provider: mappings.root_object_provider.is_some(),
            parent_stack_provider: mappings.parent_stack_provider.is_some(),
            type_descriptor_context: mappings.type_descriptor_context.is_some(),
            provide_value_target: mappings.provide_value_target.is_some(),
            uri_context_provider: mappings.uri_context_provider.is_some(),
            xml_namespace_info_provider: mappings.xml_namespace_info_provider.is_some(),
        }
    }

    fn own_services(&self) -> impl Iterator<Item = RuntimeContextService> {
        [
            (self.root_object_provider, RuntimeContextService::RootObjectProvider),
            (self.parent_stack_provider, RuntimeContextService::ParentStackProvider),
            (self.type_descriptor_context, RuntimeContextService::TypeDescriptorContext),
            (self.provide_value_target, RuntimeContextService::ProvideValueTarget),
            (self.uri_context_provider, RuntimeContextService::UriContext),
        ]
        .into_iter()
        .filter_map(|(enabled, service)| enabled.then_some(service))
    }
}

/// Maps the context onto the service contracts of the markup runtime: which
/// handle type identifies each service, and how the services of a parent
/// service provider are read.
///
/// The service contracts live in the markup runtime crate; the interpreter
/// only knows them through this adapter.
/// [`DefaultRuntimeContextServices`](super::DefaultRuntimeContextServices)
/// maps onto the self-contained contracts of [`services`](super::services).
pub trait IRuntimeContextServices: 'static {
    /// The context as the service `service`, boxed for
    /// [`IServiceProvider::get_service`], if `service_type` is the handle
    /// type of that service.
    fn get_own_service(
        &self,
        context: &Rc<RuntimeContext>,
        service: RuntimeContextService,
        service_type: TypeId,
    ) -> Option<Rc<dyn Any>>;

    /// The namespace information of the document as a service, if
    /// `service_type` is the handle type of the namespace information
    /// provider.
    fn get_namespace_info_service(
        &self,
        provider: &Rc<XmlNamespaceInfoProvider>,
        service_type: TypeId,
    ) -> Option<Rc<dyn Any>>;

    /// The root object of the root object provider of a parent service
    /// provider. `None` if the parent has no root object provider.
    fn get_parent_root_object(&self, parent: &Rc<dyn IServiceProvider>) -> Option<MarkupValue>;

    /// The parents of the parent stack provider of a parent service
    /// provider, nearest first. `None` if the parent has no parent stack
    /// provider.
    fn get_parent_stack(&self, parent: &Rc<dyn IServiceProvider>) -> Option<Vec<MarkupValue>>;

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

/// A provider that belongs to a document rather than to a context: the
/// namespace information provider, and whatever a language adds.
pub trait IStaticServiceProvider: 'static {
    /// The provider as the service `service_type`, if it is one (the
    /// requested type is assignable from the provider).
    fn get_static_service(&self, service_type: TypeId) -> Option<Rc<dyn Any>>;
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

/// The run-time context: the state of one `Build`, `Populate` or deferred
/// content build, and the service provider handed to user code during it.
pub struct RuntimeContext {
    this: Weak<RuntimeContext>,
    definition: RuntimeContextDefinition,
    services: Rc<dyn IRuntimeContextServices>,
    root_object: RefCell<MarkupValue>,
    intermediate_root: RefCell<MarkupValue>,
    parents: RefCell<Vec<MarkupValue>>,
    parent_service_provider: Option<Rc<dyn IServiceProvider>>,
    inner_service_provider: RefCell<Option<Rc<dyn IServiceProvider>>>,
    static_providers: Rc<[Rc<dyn IStaticServiceProvider>]>,
    target_object: RefCell<MarkupValue>,
    target_property: RefCell<MarkupValue>,
    base_uri: RefCell<Option<Uri>>,
    extensions: RefCell<HashMap<TypeId, Rc<dyn Any>>>,
}

impl RuntimeContext {
    /// The constructor of the context class: stores the parent service
    /// provider, the static providers and the base URI.
    pub fn new(
        definition: RuntimeContextDefinition,
        services: Rc<dyn IRuntimeContextServices>,
        parent_service_provider: Option<Rc<dyn IServiceProvider>>,
        static_providers: Rc<[Rc<dyn IStaticServiceProvider>]>,
        base_uri: Option<Uri>,
    ) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            definition,
            services,
            root_object: RefCell::new(None),
            intermediate_root: RefCell::new(None),
            parents: RefCell::new(Vec::new()),
            parent_service_provider,
            inner_service_provider: RefCell::new(None),
            static_providers,
            target_object: RefCell::new(None),
            target_property: RefCell::new(None),
            base_uri: RefCell::new(if definition.uri_context_provider { base_uri } else { None }),
            extensions: RefCell::new(HashMap::new()),
        })
    }

    pub fn definition(&self) -> RuntimeContextDefinition {
        self.definition
    }

    /// The `RootObject` field.
    pub fn root_object_field(&self) -> MarkupValue {
        self.root_object.borrow().clone()
    }

    pub fn set_root_object(&self, value: MarkupValue) {
        *self.root_object.borrow_mut() = value;
    }

    /// The root object, as the root object provider reports it: the root
    /// object of this context, or else the one of the root object provider
    /// of the parent service provider.
    pub fn root_object(&self) -> MarkupValue {
        if let Some(root) = self.root_object.borrow().as_ref() {
            return Some(root.clone());
        }
        let parent = self.parent_service_provider.as_ref()?;
        self.services.get_parent_root_object(parent).flatten()
    }

    /// The `IntermediateRoot` field: the root of the deferred content being
    /// built (the root object, outside deferred content).
    pub fn intermediate_root_object(&self) -> MarkupValue {
        self.intermediate_root.borrow().clone()
    }

    pub fn set_intermediate_root_object(&self, value: MarkupValue) {
        *self.intermediate_root.borrow_mut() = value;
    }

    /// `PushParent`: adds an object to the parent stack and makes it the
    /// provide-value target object.
    pub fn push_parent(&self, value: MarkupValue) {
        self.parents.borrow_mut().push(value.clone());
        if self.definition.provide_value_target {
            *self.target_object.borrow_mut() = value;
        }
    }

    /// `PopParent`: removes the last object from the parent stack; the
    /// provide-value target object becomes the new last object (or null).
    pub fn pop_parent(&self) {
        let last = {
            let mut parents = self.parents.borrow_mut();
            parents.pop();
            parents.last().cloned().flatten()
        };
        if self.definition.provide_value_target {
            *self.target_object.borrow_mut() = last;
        }
    }

    /// The parent stack of this context only, outermost first (the
    /// `ParentsStack` field).
    pub fn parents_stack(&self) -> Vec<MarkupValue> {
        self.parents.borrow().clone()
    }

    /// The parents as the parent stack provider enumerates them: the stack
    /// of this context from the nearest parent outwards, followed by the
    /// parents of the parent stack provider of the parent service provider.
    pub fn parents(&self) -> Vec<MarkupValue> {
        let mut parents: Vec<MarkupValue> = self.parents.borrow().iter().rev().cloned().collect();
        if let Some(parent) = &self.parent_service_provider {
            if let Some(outer) = self.services.get_parent_stack(parent) {
                parents.extend(outer);
            }
        }
        parents
    }

    /// The parent service provider (the `_sp` field).
    pub fn parent_service_provider(&self) -> Option<Rc<dyn IServiceProvider>> {
        self.parent_service_provider.clone()
    }

    /// The inner service provider (the `_innerSp` field): the provider the
    /// language's factory method created for this context; asked first.
    pub fn inner_service_provider(&self) -> Option<Rc<dyn IServiceProvider>> {
        self.inner_service_provider.borrow().clone()
    }

    pub fn set_inner_service_provider(&self, value: Option<Rc<dyn IServiceProvider>>) {
        *self.inner_service_provider.borrow_mut() = value;
    }

    /// The `ProvideTargetObject` field.
    pub fn target_object(&self) -> MarkupValue {
        self.target_object.borrow().clone()
    }

    pub fn set_target_object(&self, value: MarkupValue) {
        *self.target_object.borrow_mut() = value;
    }

    /// The `ProvideTargetProperty` field.
    pub fn target_property(&self) -> MarkupValue {
        self.target_property.borrow().clone()
    }

    pub fn set_target_property(&self, value: MarkupValue) {
        *self.target_property.borrow_mut() = value;
    }

    pub fn base_uri(&self) -> Option<Uri> {
        self.base_uri.borrow().clone()
    }

    pub fn set_base_uri(&self, value: Option<Uri>) {
        *self.base_uri.borrow_mut() = value;
    }

    /// State a language attaches to the context (the fields its context
    /// type builder callback adds to the generated class).
    pub fn extension<T: 'static>(&self) -> Option<Rc<T>> {
        self.extensions.borrow().get(&TypeId::of::<T>()).cloned().and_then(|e| e.downcast::<T>().ok())
    }

    pub fn set_extension<T: 'static>(&self, extension: Rc<T>) {
        self.extensions.borrow_mut().insert(TypeId::of::<T>(), extension);
    }

    /// This context as a service provider handle.
    pub fn as_service_provider(&self) -> Option<Rc<dyn IServiceProvider>> {
        self.this.upgrade().map(|this| this as Rc<dyn IServiceProvider>)
    }
}

impl RuntimeContext {
    /// The services of the context without the ones of its inner service
    /// provider: what the inner service provider itself is built on.
    pub(crate) fn get_service_below_inner(&self, service_type: TypeId) -> Option<Rc<dyn Any>> {
        self.get_service_after_inner(service_type)
    }
}

impl IServiceProvider for RuntimeContext {
    fn get_service(&self, service_type: TypeId) -> Option<Rc<dyn Any>> {
        let inner = self.inner_service_provider.borrow().clone();
        if let Some(inner) = inner {
            if let Some(service) = inner.get_service(service_type) {
                return Some(service);
            }
        }
        self.get_service_after_inner(service_type)
    }
}

impl RuntimeContext {
    fn get_service_after_inner(&self, service_type: TypeId) -> Option<Rc<dyn Any>> {
        if let Some(this) = self.this.upgrade() {
            for own in self.definition.own_services() {
                if let Some(service) = self.services.get_own_service(&this, own, service_type) {
                    return Some(service);
                }
            }
        }
        for provider in self.static_providers.iter() {
            if let Some(service) = provider.get_static_service(service_type) {
                return Some(service);
            }
        }
        self.parent_service_provider.as_ref()?.get_service(service_type)
    }
}

impl ITypeDescriptorContext for RuntimeContext {}
