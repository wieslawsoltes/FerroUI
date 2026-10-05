//! The run-time context of a document being built: the equivalent of the
//! context class the IL back end of the managed original generates for a
//! document (`IL/RuntimeContext.cs` of the compiler library, with the
//! members the framework language adds: `FerroXamlIlContextNameScopeField`,
//! `FerroXamlIlContextEagerParentStackProvider`).
//!
//! One implementation serves both back ends (docs/porting/xaml.md, 9.12
//! ruling 7): the interpreter of the run-time loader creates one per
//! `Build`, `Populate` and deferred content build, and Rust source generated
//! by the ahead-of-time compiler creates the same.
//!
//! The context is the service provider markup extensions, type converters
//! and constructors receive. It resolves its services in the order of the
//! generated `GetService`: the inner service provider, its own services
//! (root object, parent stack, type descriptor context, provide-value
//! target, URI context), the static providers of the document, and last the
//! parent service provider.

use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

use ferroui_base::metadata::{service, IServiceProvider, MarkupValue};
use ferroui_base::utilities::Uri;
use ferroui_base::BoxedValue;

use crate::converters::ITypeDescriptorContext;
use crate::{IProvideValueTarget, IRootObjectProvider, IUriContext};

use super::{IFerroXamlIlEagerParentStackProvider, IFerroXamlIlParentStackProvider, XamlIlRuntimeHelpers};

/// The services a context provides itself, in the order they are checked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum XamlIlContextService {
    RootObjectProvider,
    ParentStackProvider,
    TypeDescriptorContext,
    ProvideValueTarget,
    UriContext,
}

/// What the language configured the context to be: which services it
/// implements (the type mappings of the language that are set).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct XamlIlContextDefinition {
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

impl XamlIlContextDefinition {
    /// The own services of a context of this definition, in the order they
    /// are checked.
    pub fn own_services(&self) -> impl Iterator<Item = XamlIlContextService> {
        [
            (self.root_object_provider, XamlIlContextService::RootObjectProvider),
            (self.parent_stack_provider, XamlIlContextService::ParentStackProvider),
            (self.type_descriptor_context, XamlIlContextService::TypeDescriptorContext),
            (self.provide_value_target, XamlIlContextService::ProvideValueTarget),
            (self.uri_context_provider, XamlIlContextService::UriContext),
        ]
        .into_iter()
        .filter_map(|(enabled, service)| enabled.then_some(service))
    }
}

/// Maps the context onto service contracts: which handle type identifies
/// each service, and how the services of a parent service provider are
/// read. [`FrameworkContextServices`] maps onto the contracts of this
/// library; the interpreter's tests supply self-contained ones.
pub trait IXamlIlContextServices: 'static {
    /// The context as the service `service`, boxed for
    /// [`IServiceProvider::get_service`], if `service_type` is the handle
    /// type of that service.
    fn get_own_service(
        &self,
        context: &Rc<XamlIlContext>,
        service: XamlIlContextService,
        service_type: TypeId,
    ) -> Option<Rc<dyn Any>>;

    /// The root object of the root object provider of a parent service
    /// provider. `None` if the parent has no root object provider.
    fn get_parent_root_object(&self, parent: &Rc<dyn IServiceProvider>) -> Option<MarkupValue>;

    /// The parents of the parent stack provider of a parent service
    /// provider, nearest first. `None` if the parent has no parent stack
    /// provider.
    fn get_parent_stack(&self, parent: &Rc<dyn IServiceProvider>) -> Option<Vec<MarkupValue>>;
}

/// A provider that belongs to a document rather than to a context: the
/// namespace information provider, and whatever a language adds.
pub trait IStaticServiceProvider: 'static {
    /// The provider as the service `service_type`, if it is one (the
    /// requested type is assignable from the provider).
    fn get_static_service(&self, service_type: TypeId) -> Option<Rc<dyn Any>>;
}

/// The run-time context: the state of one `Build`, `Populate` or deferred
/// content build, and the service provider handed to user code during it.
pub struct XamlIlContext {
    this: Weak<XamlIlContext>,
    definition: XamlIlContextDefinition,
    services: Rc<dyn IXamlIlContextServices>,
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

impl XamlIlContext {
    /// The constructor of the context class: stores the parent service
    /// provider, the static providers and the base URI.
    pub fn new(
        definition: XamlIlContextDefinition,
        services: Rc<dyn IXamlIlContextServices>,
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

    pub fn definition(&self) -> XamlIlContextDefinition {
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

    /// The context as the service provider its inner service provider is
    /// built on (the argument of the language's factory method), without
    /// keeping the context alive: it provides the services below the inner
    /// provider, and nothing once the context is gone.
    pub fn service_provider_below_inner(&self) -> Rc<dyn IServiceProvider> {
        Rc::new(WeakContextServiceProvider(self.this.clone()))
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

    /// The services of the context without the ones of its inner service
    /// provider: what the inner service provider itself is built on.
    pub fn get_service_below_inner(&self, service_type: TypeId) -> Option<Rc<dyn Any>> {
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

impl IServiceProvider for XamlIlContext {
    fn get_service(&self, service_type: TypeId) -> Option<Rc<dyn Any>> {
        let inner = self.inner_service_provider.borrow().clone();
        if let Some(inner) = inner {
            if let Some(service) = inner.get_service(service_type) {
                return Some(service);
            }
        }
        self.get_service_below_inner(service_type)
    }
}

/// The context as the service provider its inner service provider is built
/// on, without keeping the context alive. A context that is gone provides
/// nothing.
struct WeakContextServiceProvider(Weak<XamlIlContext>);

impl IServiceProvider for WeakContextServiceProvider {
    fn get_service(&self, service_type: TypeId) -> Option<Rc<dyn Any>> {
        // Only the services below the inner provider: asking the context itself
        // would ask the inner provider again.
        self.0.upgrade()?.get_service_below_inner(service_type)
    }
}

// The context implements the service contracts of this library.

impl IRootObjectProvider for XamlIlContext {
    fn root_object(&self) -> Option<BoxedValue> {
        XamlIlContext::root_object(self)
    }

    fn intermediate_root_object(&self) -> Option<BoxedValue> {
        XamlIlContext::intermediate_root_object(self)
    }
}

impl IProvideValueTarget for XamlIlContext {
    fn target_object(&self) -> Option<BoxedValue> {
        XamlIlContext::target_object(self)
    }

    fn target_property(&self) -> Option<BoxedValue> {
        XamlIlContext::target_property(self)
    }
}

impl IUriContext for XamlIlContext {
    fn base_uri(&self) -> Option<Uri> {
        XamlIlContext::base_uri(self)
    }

    fn set_base_uri(&self, value: Option<Uri>) {
        XamlIlContext::set_base_uri(self, value)
    }
}

impl ITypeDescriptorContext for XamlIlContext {}

/// The parent stack provider of the context: the lazily enumerated form.
impl IFerroXamlIlParentStackProvider for XamlIlContext {
    fn parents(&self) -> Vec<BoxedValue> {
        XamlIlContext::parents(self).into_iter().flatten().collect()
    }

    fn as_eager_parent_stack_provider(self: Rc<Self>) -> Option<Rc<dyn IFerroXamlIlEagerParentStackProvider>> {
        Some(self)
    }
}

/// The eager parent stack provider of the context
/// (`FerroXamlIlContextEagerParentStackProvider`).
impl IFerroXamlIlEagerParentStackProvider for XamlIlContext {
    /// `DirectParentsStack => (IReadOnlyList<object>) ParentsStack`.
    fn direct_parents_stack(&self) -> Rc<Vec<BoxedValue>> {
        Rc::new(self.parents_stack().into_iter().flatten().collect())
    }

    /// `ParentProvider => XamlIlRuntimeHelpers.AsEagerParentStackProvider(
    /// _serviceProvider.GetService(typeof(IFerroXamlIlParentStackProvider)))`.
    fn parent_provider(&self) -> Option<Rc<dyn IFerroXamlIlEagerParentStackProvider>> {
        let parent = self.parent_service_provider()?;
        let provider = parent.get_service_of::<Rc<dyn IFerroXamlIlParentStackProvider>>()?;
        Some(XamlIlRuntimeHelpers::as_eager_parent_stack_provider(provider))
    }
}

/// Maps the context onto the service contracts of this library. Services
/// are identified by their handle types: `Rc<dyn IRootObjectProvider>`,
/// `Rc<dyn IFerroXamlIlParentStackProvider>`, `Rc<dyn ITypeDescriptorContext>`,
/// `Rc<dyn IProvideValueTarget>` and `Rc<dyn IUriContext>`.
pub struct FrameworkContextServices;

impl IXamlIlContextServices for FrameworkContextServices {
    fn get_own_service(
        &self,
        context: &Rc<XamlIlContext>,
        own: XamlIlContextService,
        service_type: TypeId,
    ) -> Option<Rc<dyn Any>> {
        match own {
            XamlIlContextService::RootObjectProvider => {
                service(service_type, || context.clone() as Rc<dyn IRootObjectProvider>)
            }
            XamlIlContextService::ParentStackProvider => {
                service(service_type, || context.clone() as Rc<dyn IFerroXamlIlParentStackProvider>)
            }
            XamlIlContextService::TypeDescriptorContext => {
                service(service_type, || context.clone() as Rc<dyn ITypeDescriptorContext>)
            }
            XamlIlContextService::ProvideValueTarget => {
                service(service_type, || context.clone() as Rc<dyn IProvideValueTarget>)
            }
            XamlIlContextService::UriContext => service(service_type, || context.clone() as Rc<dyn IUriContext>),
        }
    }

    fn get_parent_root_object(&self, parent: &Rc<dyn IServiceProvider>) -> Option<MarkupValue> {
        parent.get_service_of::<Rc<dyn IRootObjectProvider>>().map(|provider| provider.root_object())
    }

    fn get_parent_stack(&self, parent: &Rc<dyn IServiceProvider>) -> Option<Vec<MarkupValue>> {
        parent
            .get_service_of::<Rc<dyn IFerroXamlIlParentStackProvider>>()
            .map(|provider| provider.parents().into_iter().map(Some).collect())
    }
}
