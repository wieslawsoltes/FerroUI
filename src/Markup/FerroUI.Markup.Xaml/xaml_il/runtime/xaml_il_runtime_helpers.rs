//! Port of `XamlIl/Runtime/XamlIlRuntimeHelpers.cs`: the helpers compiled
//! (and interpreted) markup calls at run time.

use super::{
    FerroXamlIlXmlNamespaceInfo, IFerroXamlIlControlTemplateProvider, IFerroXamlIlEagerParentStackProvider,
    IFerroXamlIlParentStackProvider, IFerroXamlIlXmlNamespaceInfoProvider, XamlIlParentStackProviderWrapper,
};
use crate::object_casts::{as_object, rc_of};
use crate::{
    EagerParentStackEnumerator, IRootObjectProvider, IXamlTypeResolver, ServiceProviderExtensions, XamlLoadException,
    XamlResourceNode,
};
use ferroui_base::controls::{ChildNameScope, IDeferredContent, INameScope, NameScope, NameScopeRef};
use ferroui_base::data::core::expression_nodes::CastTarget;
use ferroui_base::data::core::{ValueType, ValueTypes, WeakValue};
use ferroui_base::data::{BindingBase, BindingPriority};
use ferroui_base::metadata::{service, IServiceProvider, MarkupAssembly, MarkupType};
use ferroui_base::{
    ferro_markup_type, BoxedValue, FerroProperty, TypeInfo, UnsetValueType, WeakRef,
};
use ferroui_base::platform::IRuntimePlatform;
use ferroui_base::{FerroLocator, LocatorExtensions};
use ferroui_controls::Application;
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// The function that builds deferred content: the compiled (or interpreted)
/// body of a template or of a deferred resource. It receives the service
/// provider of the instantiation and returns the object it built.
///
/// (The function pointer and delegate forms of the managed original.) A
/// builder can fail: its error is the exception the managed builder would
/// throw, and is the error of the instantiation.
#[derive(Clone)]
pub struct DeferredContentBuilder(
    Rc<dyn Fn(&Rc<dyn IServiceProvider>) -> Result<Option<BoxedValue>, XamlLoadException>>,
);

impl DeferredContentBuilder {
    /// A builder that cannot fail.
    pub fn new(builder: impl Fn(&Rc<dyn IServiceProvider>) -> Option<BoxedValue> + 'static) -> Self {
        Self(Rc::new(move |service_provider| Ok(builder(service_provider))))
    }

    /// A builder that can fail.
    pub fn try_new(
        builder: impl Fn(&Rc<dyn IServiceProvider>) -> Result<Option<BoxedValue>, XamlLoadException> + 'static,
    ) -> Self {
        Self(Rc::new(builder))
    }

    /// Wraps a shared builder function.
    pub fn from_rc(
        builder: Rc<dyn Fn(&Rc<dyn IServiceProvider>) -> Result<Option<BoxedValue>, XamlLoadException>>,
    ) -> Self {
        Self(builder)
    }

    /// Calls the builder.
    pub fn invoke(&self, service_provider: &Rc<dyn IServiceProvider>) -> Result<Option<BoxedValue>, XamlLoadException> {
        (self.0)(service_provider)
    }
}

/// Builders compare by identity, so that they can be held in untyped values.
impl PartialEq for DeferredContentBuilder {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

/// What one instantiation of deferred content produced: the object and the
/// name scope its named elements were registered in (the template result of
/// the managed original, before the object is cast to the result type).
#[derive(Clone)]
pub struct DeferredResult {
    pub result: Option<BoxedValue>,
    pub name_scope: NameScopeRef,
}

/// Deferred content: the body of a template (or of a deferred resource)
/// together with what it captured where it was declared: the resource nodes
/// above it, the root object and the name scope.
///
/// Every instantiation ([`build_with`](Self::build_with)) runs the builder
/// with a fresh name scope (a child of the captured one) and a service
/// provider that answers for the captured context.
///
/// As [`IDeferredContent`] it builds the object alone; this is what a
/// resource dictionary stores for a deferred resource. That contract has no
/// error channel: a failing builder surfaces there as a panic carrying the
/// message, at the place the resource is first asked for (where the managed
/// exception would propagate from).
/// [`TemplateContent::load`](crate::templates::TemplateContent::load)
/// consumes it through `build_with` to also obtain the name scope.
///
/// What it captured is held weakly (DEVIATIONS.md, Markup): the content
/// belongs to a template or a resource declared under those very objects
/// (the root of the document, the elements and dictionaries above the
/// declaration, the elements the name scope names), which own it. Held
/// strongly, as the managed original holds them, the content and they keep
/// each other alive, which the collector resolves there and nothing resolves
/// here. An instantiation sees the ones that are still alive.
pub struct DeferredContent {
    this: Weak<DeferredContent>,
    parent_name_scope: Option<Weak<dyn INameScope>>,
    root_object: Option<WeakValue>,
    parent_resource_nodes: Rc<CapturedResourceNodes>,
    builder: DeferredContentBuilder,
    result_type: ValueType,
}

/// The resource nodes above a declaration, held weakly, shared by the
/// deferred content declared under the same parents.
struct CapturedResourceNodes {
    nodes: Vec<WeakValue>,
    /// The stack of the nodes that are alive, while an instantiation (or
    /// whoever asked one for it) holds it: instantiations that overlap share
    /// one stack, as all of them do in the managed original.
    alive: RefCell<Weak<Vec<BoxedValue>>>,
}

impl CapturedResourceNodes {
    fn alive(&self) -> Rc<Vec<BoxedValue>> {
        if let Some(alive) = self.alive.borrow().upgrade() {
            return alive;
        }
        let alive: Rc<Vec<BoxedValue>> = Rc::new(self.nodes.iter().filter_map(WeakValue::upgrade).collect());
        *self.alive.borrow_mut() = Rc::downgrade(&alive);
        alive
    }
}

impl DeferredContent {
    fn new(
        parent_resource_nodes: Rc<CapturedResourceNodes>,
        root_object: Option<BoxedValue>,
        parent_name_scope: Option<NameScopeRef>,
        builder: DeferredContentBuilder,
        result_type: ValueType,
    ) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            parent_name_scope: parent_name_scope.map(|scope| Rc::downgrade(&scope.0)),
            root_object: root_object.as_ref().map(WeakValue::new),
            parent_resource_nodes,
            builder,
            result_type,
        })
    }

    /// The type the built object is expected to be (the type argument of the
    /// factory that created the content).
    pub fn result_type(&self) -> ValueType {
        self.result_type
    }

    /// Builds the content: `Build(IServiceProvider?)`.
    ///
    /// # Panics
    /// Panics if the builder fails, or if the built object is not of the
    /// result type (the invalid cast of the managed original). Use
    /// [`try_build_with`](Self::try_build_with) to handle both.
    pub fn build_with(&self, service_provider: Option<&Rc<dyn IServiceProvider>>) -> DeferredResult {
        crate::throw(self.try_build_with(service_provider))
    }

    /// [`build_with`](Self::build_with) without the panic: the error of the
    /// builder, and a built object that is not of the result type, are
    /// errors. (The name scope of a failed instantiation is not completed,
    /// as in the original.)
    pub fn try_build_with(
        &self,
        service_provider: Option<&Rc<dyn IServiceProvider>>,
    ) -> Result<DeferredResult, XamlLoadException> {
        let scope = match self.parent_name_scope.as_ref().and_then(Weak::upgrade) {
            None => NameScopeRef::new(NameScope::new()),
            Some(parent) => NameScopeRef::new(ChildNameScope::new(NameScopeRef(parent))),
        };
        // The captured objects that are still alive, for the time of the
        // instantiation.
        let provider: Rc<dyn IServiceProvider> = DeferredParentServiceProvider::new(
            service_provider.cloned(),
            self.parent_resource_nodes.alive(),
            self.root_object.as_ref().and_then(WeakValue::upgrade),
            scope.clone(),
        );
        let obj = self.builder.invoke(&provider)?;
        scope.complete();

        if let Some(obj) = &obj {
            if !self.result_type.is_object() && !ValueTypes::is_assignable(ValueType::of_value(&**obj), self.result_type) {
                return Err(XamlLoadException::with_message(format!(
                    "Unable to cast object of type '{}' to type '{}'.",
                    (**obj).type_name(),
                    self.result_type
                )));
            }
        }

        Ok(DeferredResult { result: obj, name_scope: scope })
    }

    /// The content as the deferred content contract.
    pub fn as_deferred_content(&self) -> Rc<dyn IDeferredContent> {
        self.this.upgrade().expect("the deferred content is alive")
    }
}

impl IDeferredContent for DeferredContent {
    fn build(&self, service_provider: Option<&Rc<dyn IServiceProvider>>) -> Option<BoxedValue> {
        self.build_with(service_provider).result
    }
}

/// Deferred content compares by identity.
impl PartialEq for DeferredContent {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

/// The service provider of one instantiation of deferred content.
struct DeferredParentServiceProvider {
    this: Weak<DeferredParentServiceProvider>,
    parent_provider: Option<Rc<dyn IServiceProvider>>,
    parent_resource_nodes: Rc<Vec<BoxedValue>>,
    name_scope: NameScopeRef,
    root_object: Option<BoxedValue>,
    runtime_platform: RefCell<Option<Rc<dyn IRuntimePlatform>>>,
    parent_stack_provider: RefCell<Option<Option<Rc<dyn IFerroXamlIlEagerParentStackProvider>>>>,
}

impl DeferredParentServiceProvider {
    fn new(
        parent_provider: Option<Rc<dyn IServiceProvider>>,
        parent_resource_nodes: Rc<Vec<BoxedValue>>,
        root_object: Option<BoxedValue>,
        name_scope: NameScopeRef,
    ) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            parent_provider,
            parent_resource_nodes,
            name_scope,
            root_object,
            runtime_platform: RefCell::new(None),
            parent_stack_provider: RefCell::new(None),
        })
    }

    fn this(&self) -> Rc<Self> {
        self.this.upgrade().expect("the service provider is alive while it is used")
    }

    fn get_parent_stack_provider_from_services(&self) -> Option<Rc<dyn IFerroXamlIlEagerParentStackProvider>> {
        self.parent_provider
            .as_ref()?
            .get_service_of::<Rc<dyn IFerroXamlIlParentStackProvider>>()?
            .as_eager_parent_stack_provider()
    }
}

impl IRootObjectProvider for DeferredParentServiceProvider {
    fn root_object(&self) -> Option<BoxedValue> {
        self.root_object.clone()
    }

    fn intermediate_root_object(&self) -> Option<BoxedValue> {
        self.root_object.clone()
    }
}

impl IFerroXamlIlControlTemplateProvider for DeferredParentServiceProvider {}

impl IFerroXamlIlParentStackProvider for DeferredParentServiceProvider {
    fn parents(&self) -> Vec<BoxedValue> {
        self.parent_resource_nodes.iter().rev().cloned().collect()
    }

    fn as_eager_parent_stack_provider(self: Rc<Self>) -> Option<Rc<dyn IFerroXamlIlEagerParentStackProvider>> {
        Some(self)
    }
}

impl IFerroXamlIlEagerParentStackProvider for DeferredParentServiceProvider {
    fn direct_parents_stack(&self) -> Rc<Vec<BoxedValue>> {
        self.parent_resource_nodes.clone()
    }

    fn parent_provider(&self) -> Option<Rc<dyn IFerroXamlIlEagerParentStackProvider>> {
        if let Some(provider) = &*self.parent_stack_provider.borrow() {
            return provider.clone();
        }
        let provider = self.get_parent_stack_provider_from_services();
        *self.parent_stack_provider.borrow_mut() = Some(provider.clone());
        provider
    }
}

impl IServiceProvider for DeferredParentServiceProvider {
    fn get_service(&self, service_type: TypeId) -> Option<Rc<dyn Any>> {
        if let Some(scope) = name_scope_service(service_type, &self.name_scope) {
            return Some(scope);
        }
        if let Some(this) = service(service_type, || -> Rc<dyn IFerroXamlIlParentStackProvider> { self.this() }) {
            return Some(this);
        }
        if let Some(this) = service(service_type, || -> Rc<dyn IRootObjectProvider> { self.this() }) {
            return Some(this);
        }
        if let Some(this) = service(service_type, || -> Rc<dyn IFerroXamlIlControlTemplateProvider> { self.this() }) {
            return Some(this);
        }
        if service_type == TypeId::of::<Rc<dyn IRuntimePlatform>>() {
            if self.runtime_platform.borrow().is_none() {
                *self.runtime_platform.borrow_mut() = FerroLocator::current().get_service::<dyn IRuntimePlatform>();
            }
            let platform = self.runtime_platform.borrow().clone()?;
            return Some(Rc::new(platform));
        }
        self.parent_provider.as_ref()?.get_service(service_type)
    }
}

/// The name scope service: asked for as the contract handle or as the
/// property-value handle.
fn name_scope_service(service_type: TypeId, scope: &NameScopeRef) -> Option<Rc<dyn Any>> {
    service(service_type, || -> Rc<dyn INameScope> { scope.0.clone() }).or_else(|| service(service_type, || scope.clone()))
}

/// Parent resource nodes are often the same (e.g. most values in a
/// resource dictionary), cache the last ones.
struct LastParentStack {
    parent_stack_provider: Weak<dyn IFerroXamlIlParentStackProvider>,
    resource_nodes: Weak<CapturedResourceNodes>,
}

impl LastParentStack {
    fn is_equivalent_to(
        &self,
        parent_stack_provider: &Rc<dyn IFerroXamlIlParentStackProvider>,
        resource_nodes: &[BoxedValue],
    ) -> Option<Rc<CapturedResourceNodes>> {
        let last_parent_stack_provider = self.parent_stack_provider.upgrade()?;
        let last_resource_nodes = self.resource_nodes.upgrade()?;
        if !std::ptr::addr_eq(Rc::as_ptr(parent_stack_provider), Rc::as_ptr(&last_parent_stack_provider))
            || resource_nodes.len() != last_resource_nodes.nodes.len()
        {
            return None;
        }

        let same = resource_nodes
            .iter()
            .zip(last_resource_nodes.nodes.iter())
            .all(|(a, b)| b.upgrade().is_some_and(|b| ValueTypes::identity_equals(Some(a), Some(&b))));
        same.then_some(last_resource_nodes)
    }
}

thread_local! {
    static RESOURCE_NODE_BUFFER: RefCell<Vec<BoxedValue>> = RefCell::new(Vec::with_capacity(8));
    static LAST_PARENT_STACK: RefCell<Option<LastParentStack>> = const { RefCell::new(None) };
    static LAST_APPLICATION_PROVIDER: RefCell<Option<Rc<ApplicationParentStackProvider>>> = const { RefCell::new(None) };
    static EMPTY_PARENT_STACK_PROVIDER: Rc<EmptyParentStackProvider> =
        Rc::new(EmptyParentStackProvider { stack: Rc::new(Vec::new()) });
}

/// The marker a root service provider answers with when it is asked for
/// the runtime platform and none was registered when it was created: the
/// state in which the managed provider throws "IRuntimePlatform was not
/// registered".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimePlatformNotRegistered;

/// The helpers compiled markup calls at run time. The names carry the
/// version of the contract between the compiler and the runtime.
pub struct XamlIlRuntimeHelpers;

impl XamlIlRuntimeHelpers {
    /// `serviceProvider.GetService<IRuntimePlatform>()` with the behaviour
    /// of the service providers of this type: the platform, `None` from a
    /// provider that does not know the service, and the error of the root
    /// service provider when no platform was registered.
    pub fn get_runtime_platform(
        service_provider: &Rc<dyn IServiceProvider>,
    ) -> Result<Option<Rc<dyn IRuntimePlatform>>, XamlLoadException> {
        if let Some(platform) = service_provider.get_service_of::<Rc<dyn IRuntimePlatform>>() {
            return Ok(Some(platform));
        }
        if service_provider.get_service_of::<RuntimePlatformNotRegistered>().is_some() {
            return Err(XamlLoadException::with_message("IRuntimePlatform was not registered"));
        }
        Ok(None)
    }

    /// Creates the deferred content of a template whose result is a control.
    pub fn deferred_transformation_factory_v1(
        builder: DeferredContentBuilder,
        provider: &Rc<dyn IServiceProvider>,
    ) -> Rc<DeferredContent> {
        Self::deferred_transformation_factory_v2::<ferroui_base::Ref<ferroui_controls::Control>>(builder, provider)
    }

    /// Creates deferred content whose result is a `T`.
    ///
    /// The managed original returns the build function as a delegate; a
    /// function cannot be an untyped value here, so the deferred content
    /// object stands for it.
    pub fn deferred_transformation_factory_v2<T: 'static>(
        builder: DeferredContentBuilder,
        provider: &Rc<dyn IServiceProvider>,
    ) -> Rc<DeferredContent> {
        Self::deferred_transformation_factory_for(ValueType::of::<T>(), builder, provider)
    }

    /// Creates deferred content whose result is a `T`: captures the
    /// resource nodes above the declaration, the root object and the name
    /// scope from `provider`.
    ///
    /// # Panics
    /// Panics if `provider` has no parent stack or root object service.
    pub fn deferred_transformation_factory_v3<T: 'static>(
        builder: DeferredContentBuilder,
        provider: &Rc<dyn IServiceProvider>,
    ) -> Rc<DeferredContent> {
        Self::deferred_transformation_factory_for(ValueType::of::<T>(), builder, provider)
    }

    /// The factory with the result type given as a value: what untyped
    /// callers use in place of the type argument. The "any value" type
    /// ([`ValueType::object`]) accepts every result.
    pub fn deferred_transformation_factory_for(
        result_type: ValueType,
        builder: DeferredContentBuilder,
        provider: &Rc<dyn IServiceProvider>,
    ) -> Rc<DeferredContent> {
        crate::throw(Self::try_deferred_transformation_factory_for(result_type, builder, provider))
    }

    /// [`deferred_transformation_factory_for`](Self::deferred_transformation_factory_for)
    /// without panics: a provider without a parent stack or root object
    /// service is an error. This is what untyped (metadata) callers use.
    pub fn try_deferred_transformation_factory_for(
        result_type: ValueType,
        builder: DeferredContentBuilder,
        provider: &Rc<dyn IServiceProvider>,
    ) -> Result<Rc<DeferredContent>, XamlLoadException> {
        let missing = |service: &str| XamlLoadException::with_message(format!("Service {service} hasn't been registered"));
        let parent_stack = provider
            .get_service_of::<Rc<dyn IFerroXamlIlParentStackProvider>>()
            .ok_or_else(|| missing("IFerroXamlIlParentStackProvider"))?;
        let resource_nodes = Self::as_resource_nodes_stack(&parent_stack);
        let root_object = provider
            .get_service_of::<Rc<dyn IRootObjectProvider>>()
            .ok_or_else(|| missing("IRootObjectProvider"))?
            .root_object();
        let parent_scope = provider.get_name_scope().map(NameScopeRef);

        Ok(DeferredContent::new(resource_nodes, root_object, parent_scope, builder, result_type))
    }

    fn as_resource_nodes_stack(provider: &Rc<dyn IFerroXamlIlParentStackProvider>) -> Rc<CapturedResourceNodes> {
        let mut buffer = RESOURCE_NODE_BUFFER.with(|buffer| std::mem::take(&mut *buffer.borrow_mut()));
        buffer.clear();

        match provider.clone().as_eager_parent_stack_provider() {
            Some(eager_provider) => {
                let mut enumerator = EagerParentStackEnumerator::new(Some(eager_provider));

                while let Some(parent) = enumerator.try_get_next() {
                    if is_resource_node(&parent) {
                        buffer.push(parent);
                    }
                }
            }
            None => {
                for item in provider.parents() {
                    if is_resource_node(&item) {
                        buffer.push(item);
                    }
                }
            }
        }

        // The immediate parent should be last in the stack.
        buffer.reverse();

        let cached = LAST_PARENT_STACK
            .with(|last| last.borrow().as_ref().and_then(|last| last.is_equivalent_to(provider, &buffer)));
        let resource_nodes = match cached {
            Some(resource_nodes) => resource_nodes,
            None => {
                let resource_nodes = Rc::new(CapturedResourceNodes {
                    nodes: buffer.iter().map(WeakValue::new).collect(),
                    alive: RefCell::new(Weak::new()),
                });
                LAST_PARENT_STACK.with(|last| {
                    *last.borrow_mut() = Some(LastParentStack {
                        parent_stack_provider: Rc::downgrade(provider),
                        resource_nodes: Rc::downgrade(&resource_nodes),
                    })
                });
                resource_nodes
            }
        };

        buffer.clear();
        RESOURCE_NODE_BUFFER.with(|slot| *slot.borrow_mut() = buffer);
        resource_nodes
    }

    /// The provider as an eager provider: itself if it is one, a wrapper
    /// that snapshots its parents otherwise.
    pub fn as_eager_parent_stack_provider(
        provider: Rc<dyn IFerroXamlIlParentStackProvider>,
    ) -> Rc<dyn IFerroXamlIlEagerParentStackProvider> {
        match provider.clone().as_eager_parent_stack_provider() {
            Some(eager) => eager,
            None => XamlIlParentStackProviderWrapper::new(provider),
        }
    }

    /// Applies the value of a markup extension that is not assignable to
    /// the property it was provided for: a binding is bound to the property,
    /// the unset value marker is set on it.
    ///
    /// `property` is the registered property (`&'static FerroProperty`) or
    /// whatever else identifies the target member.
    pub fn apply_non_matching_markup_extension_v1(
        target: Option<&BoxedValue>,
        property: Option<&BoxedValue>,
        _prov: &Rc<dyn IServiceProvider>,
        value: Option<&BoxedValue>,
    ) -> Result<(), XamlLoadException> {
        let ferro_property = property.and_then(|p| p.downcast_ref::<&'static FerroProperty>().copied());
        let describe = |value: Option<&BoxedValue>| match value {
            Some(value) => ValueTypes::try_to_string(&**value).unwrap_or_else(|| (**value).type_name().to_string()),
            None => "(null)".to_string(),
        };
        let target_object = || {
            target.and_then(as_object).ok_or_else(|| {
                XamlLoadException::with_message(format!(
                    "Unable to cast object of type '{}' to type 'FerroObject'.",
                    target.map_or("(null)", |t| (**t).type_name())
                ))
            })
        };

        if let Some(binding) = value.and_then(binding_of) {
            match ferro_property {
                Some(p) => {
                    target_object()?.bind_binding(p, &*binding);
                    Ok(())
                }
                None => Err(XamlLoadException::with_message(format!(
                    "Attempt to apply binding to non-ferro property {}",
                    describe(property)
                ))),
            }
        } else if let Some(unset) = value.and_then(|v| v.downcast_ref::<UnsetValueType>()) {
            if let Some(p) = ferro_property {
                target_object()?.set_value_untyped(p, unset, BindingPriority::LocalValue);
            }
            Ok(())
        } else {
            Err(XamlLoadException::with_message(format!(
                "Don't know what to do with {}",
                value.map_or("(null)", |v| (**v).type_name())
            )))
        }
    }

    /// Creates the service provider that offers the services built on a
    /// compiled context (`compiled`): the type resolver.
    pub fn create_inner_service_provider_v1(compiled: Rc<dyn IServiceProvider>) -> Rc<dyn IServiceProvider> {
        Rc::new(InnerServiceProvider { compiled_provider: compiled, resolver: RefCell::new(None) })
    }

    /// Creates the service provider a document is built with when it is
    /// not loaded into another one.
    pub fn create_root_service_provider_v2() -> Rc<dyn IServiceProvider> {
        RootServiceProvider::new(NameScopeRef::new(NameScope::new()), None)
    }

    /// Creates the service provider a document is built with:
    /// `parent_service_provider` is the one of the place the document is
    /// loaded into (an include, a template being instantiated).
    pub fn create_root_service_provider_v3(
        parent_service_provider: Option<Rc<dyn IServiceProvider>>,
    ) -> Rc<dyn IServiceProvider> {
        RootServiceProvider::new(NameScopeRef::new(NameScope::new()), parent_service_provider)
    }
}

fn is_resource_node(value: &BoxedValue) -> bool {
    <XamlResourceNode as crate::FromXamlObject>::from_xaml_object(value).is_some()
}

/// `value is BindingBase`: the binding an untyped value holds, if it holds
/// one. A binding is boxed as its concrete reference object; the binding
/// classes of the framework are recognised directly, any other one through
/// the registered cast to the binding contract.
pub(crate) fn binding_of(value: &BoxedValue) -> Option<Rc<dyn BindingBase>> {
    use crate::markup_extensions::{CompiledBindingExtension, DynamicResourceExtension, ReflectionBindingExtension};
    use ferroui_base::data::{CompiledBinding, MultiBinding, ReflectionBinding, TemplateBinding};
    use ferroui_markup::data::Binding;

    macro_rules! concrete {
        ($($type_:ty),*) => {$(
            if let Some(binding) = rc_of::<$type_>(value) {
                return Some(binding);
            }
        )*};
    }
    concrete!(
        CompiledBinding,
        ReflectionBinding,
        Binding,
        TemplateBinding,
        MultiBinding,
        DynamicResourceExtension,
        CompiledBindingExtension,
        ReflectionBindingExtension
    );
    ferroui_base::metadata::from_markup_value::<Rc<dyn BindingBase>>(&Some(value.clone()))
}

struct InnerServiceProvider {
    compiled_provider: Rc<dyn IServiceProvider>,
    resolver: RefCell<Option<Rc<dyn IXamlTypeResolver>>>,
}

impl IServiceProvider for InnerServiceProvider {
    fn get_service(&self, service_type: TypeId) -> Option<Rc<dyn Any>> {
        if service_type != TypeId::of::<Rc<dyn IXamlTypeResolver>>() {
            return None;
        }
        if self.resolver.borrow().is_none() {
            // Without the namespaces of the document there is no resolver
            // (the managed original fails here; a missing service lets the
            // caller report it).
            let ns_info = self.compiled_provider.get_service_of::<Rc<dyn IFerroXamlIlXmlNamespaceInfoProvider>>()?;
            let resolver: Rc<dyn IXamlTypeResolver> = Rc::new(XamlTypeResolver { ns_info });
            *self.resolver.borrow_mut() = Some(resolver);
        }
        let resolver = self.resolver.borrow().clone()?;
        Some(Rc::new(resolver))
    }
}

struct XamlTypeResolver {
    ns_info: Rc<dyn IFerroXamlIlXmlNamespaceInfoProvider>,
}

impl XamlTypeResolver {
    /// `Assembly.Load(name).GetType(namespace + "." + name)`: the type
    /// named `name` in `namespace` that the assembly declares.
    fn find(entry: &FerroXamlIlXmlNamespaceInfo, name: &str) -> Option<CastTarget> {
        let assembly = MarkupAssembly::find(&entry.clr_assembly_name())?;
        let namespace = entry.clr_namespace();
        let in_assembly =
            |module_path: &str| MarkupAssembly::of_module(module_path).is_some_and(|a| std::ptr::eq(a, assembly));

        if let Some(class) = TypeInfo::find(&namespace, name).filter(|t| in_assembly(t.module_path())) {
            return Some(CastTarget::Class(class));
        }
        let markup = MarkupType::find(&namespace, name).filter(|t| in_assembly(t.module_path))?;
        match markup.type_info {
            Some(class) => Some(CastTarget::Class(class())),
            None => markup.handle().map(CastTarget::Value),
        }
    }
}

impl IXamlTypeResolver for XamlTypeResolver {
    fn resolve(&self, qualified_type_name: &str) -> Result<CastTarget, XamlLoadException> {
        let (ns, name) = match qualified_type_name.split_once(':') {
            Some((ns, name)) => (ns, name),
            None => ("", qualified_type_name),
        };
        let namespaces = self.ns_info.xml_namespaces();
        let Some(lst) = namespaces.get(ns) else {
            return Err(XamlLoadException::with_message(format!(
                "Unable to resolve namespace for type {qualified_type_name}"
            )));
        };
        let resolvable = lst.iter().filter(|e| !e.clr_assembly_name().is_empty());
        for entry in resolvable.clone() {
            if let Some(resolved) = Self::find(entry, name) {
                return Ok(resolved);
            }
        }

        let locations: Vec<String> = resolvable
            .map(|e| format!("`clr-namespace:{};assembly={}`", e.clr_namespace(), e.clr_assembly_name()))
            .collect();
        Err(XamlLoadException::with_message(format!(
            "Unable to resolve type {qualified_type_name} from any of the following locations: {}",
            locations.join(",")
        )))
    }
}

struct RootServiceProvider {
    name_scope: NameScopeRef,
    parent_service_provider: Option<Rc<dyn IServiceProvider>>,
    runtime_platform: Option<Rc<dyn IRuntimePlatform>>,
    parent_stack_provider: RefCell<Option<Rc<dyn IFerroXamlIlParentStackProvider>>>,
}

impl RootServiceProvider {
    fn new(name_scope: NameScopeRef, parent_service_provider: Option<Rc<dyn IServiceProvider>>) -> Rc<dyn IServiceProvider> {
        Rc::new(Self {
            name_scope,
            parent_service_provider,
            runtime_platform: FerroLocator::current().get_service::<dyn IRuntimePlatform>(),
            parent_stack_provider: RefCell::new(None),
        })
    }

    fn create_parent_stack_provider(&self) -> Rc<dyn IFerroXamlIlParentStackProvider> {
        self.parent_service_provider
            .as_ref()
            .and_then(|parent| parent.get_service_of::<Rc<dyn IFerroXamlIlParentStackProvider>>())
            .unwrap_or_else(|| Self::get_parent_stack_provider_for_application(Application::current()))
    }

    fn get_parent_stack_provider_for_application(
        application: Option<ferroui_base::Ref<Application>>,
    ) -> Rc<dyn IFerroXamlIlParentStackProvider> {
        match application {
            None => EMPTY_PARENT_STACK_PROVIDER.with(|empty| empty.clone()),
            Some(application) => ApplicationParentStackProvider::get_for_application(&application),
        }
    }
}

impl IServiceProvider for RootServiceProvider {
    fn get_service(&self, service_type: TypeId) -> Option<Rc<dyn Any>> {
        if let Some(scope) = name_scope_service(service_type, &self.name_scope) {
            return Some(scope);
        }
        if service_type == TypeId::of::<Rc<dyn IRuntimePlatform>>() {
            // The managed original throws here when the platform was not
            // registered. A service lookup has no error channel, so the
            // service is absent and the provider answers for the marker
            // below instead: callers that need the platform turn it into
            // the error (see `XamlIlRuntimeHelpers::get_runtime_platform`).
            let platform = self.runtime_platform.clone()?;
            return Some(Rc::new(platform));
        }
        if service_type == TypeId::of::<RuntimePlatformNotRegistered>() && self.runtime_platform.is_none() {
            return Some(Rc::new(RuntimePlatformNotRegistered));
        }
        service(service_type, || -> Rc<dyn IFerroXamlIlParentStackProvider> {
            if let Some(provider) = &*self.parent_stack_provider.borrow() {
                return provider.clone();
            }
            let provider = self.create_parent_stack_provider();
            *self.parent_stack_provider.borrow_mut() = Some(provider.clone());
            provider
        })
    }
}

/// The parent stack of a document that is loaded on its own: the
/// application object, whose resources and styles apply to everything.
struct ApplicationParentStackProvider {
    application: WeakRef<Application>,
}

impl ApplicationParentStackProvider {
    fn get_for_application(application: &ferroui_base::Ref<Application>) -> Rc<ApplicationParentStackProvider> {
        LAST_APPLICATION_PROVIDER.with(|last| {
            let mut last = last.borrow_mut();
            match &*last {
                Some(provider) if provider.application.points_to(application) => provider.clone(),
                _ => {
                    let provider = Rc::new(ApplicationParentStackProvider { application: application.downgrade() });
                    *last = Some(provider.clone());
                    provider
                }
            }
        })
    }
}

impl IFerroXamlIlParentStackProvider for ApplicationParentStackProvider {
    fn parents(&self) -> Vec<BoxedValue> {
        match self.application.upgrade() {
            Some(application) => vec![Rc::new(application) as BoxedValue],
            None => Vec::new(),
        }
    }

    fn as_eager_parent_stack_provider(self: Rc<Self>) -> Option<Rc<dyn IFerroXamlIlEagerParentStackProvider>> {
        Some(self)
    }
}

impl IFerroXamlIlEagerParentStackProvider for ApplicationParentStackProvider {
    fn direct_parents_stack(&self) -> Rc<Vec<BoxedValue>> {
        Rc::new(self.parents())
    }

    fn parent_provider(&self) -> Option<Rc<dyn IFerroXamlIlEagerParentStackProvider>> {
        None
    }
}

struct EmptyParentStackProvider {
    stack: Rc<Vec<BoxedValue>>,
}

impl IFerroXamlIlParentStackProvider for EmptyParentStackProvider {
    fn parents(&self) -> Vec<BoxedValue> {
        Vec::new()
    }

    fn as_eager_parent_stack_provider(self: Rc<Self>) -> Option<Rc<dyn IFerroXamlIlEagerParentStackProvider>> {
        Some(self)
    }
}

impl IFerroXamlIlEagerParentStackProvider for EmptyParentStackProvider {
    fn direct_parents_stack(&self) -> Rc<Vec<BoxedValue>> {
        self.stack.clone()
    }

    fn parent_provider(&self) -> Option<Rc<dyn IFerroXamlIlEagerParentStackProvider>> {
        None
    }
}

ferro_markup_type!(class DeferredContentBuilder {
    handles: [DeferredContentBuilder],
});

ferro_markup_type!(class DeferredContent {
    this: Rc<DeferredContent>,
    handles: [DeferredContent, Rc<DeferredContent>, Option<Rc<DeferredContent>>],
    interfaces: [Rc<dyn IDeferredContent>],
    methods: [
        try fn Build(Option<Rc<dyn IServiceProvider>>) -> Option<BoxedValue> =>
            |this: &Rc<DeferredContent>, service_provider: Option<Rc<dyn IServiceProvider>>| {
                this.try_build_with(service_provider.as_ref()).map(|built| built.result)
            },
    ],
});

ferro_markup_type!(static XamlIlRuntimeHelpers {
    methods: [
        static try fn DeferredTransformationFactoryV1(DeferredContentBuilder, Rc<dyn IServiceProvider>) -> Rc<DeferredContent> =>
            |builder: DeferredContentBuilder, provider: Rc<dyn IServiceProvider>| {
                XamlIlRuntimeHelpers::try_deferred_transformation_factory_for(
                    ValueType::of::<ferroui_base::Ref<ferroui_controls::Control>>(),
                    builder,
                    &provider,
                )
            },
        static try fn DeferredTransformationFactoryV2(DeferredContentBuilder, Rc<dyn IServiceProvider>) -> Rc<DeferredContent> =>
            |builder: DeferredContentBuilder, provider: Rc<dyn IServiceProvider>| {
                XamlIlRuntimeHelpers::try_deferred_transformation_factory_for(ValueType::object(), builder, &provider)
            },
        static try fn DeferredTransformationFactoryV3(DeferredContentBuilder, Rc<dyn IServiceProvider>) -> Rc<DeferredContent> =>
            |builder: DeferredContentBuilder, provider: Rc<dyn IServiceProvider>| {
                XamlIlRuntimeHelpers::try_deferred_transformation_factory_for(ValueType::object(), builder, &provider)
            },
        static fn AsEagerParentStackProvider(Rc<dyn IFerroXamlIlParentStackProvider>) -> Rc<dyn IFerroXamlIlEagerParentStackProvider> =>
            XamlIlRuntimeHelpers::as_eager_parent_stack_provider,
        static try fn ApplyNonMatchingMarkupExtensionV1(Option<BoxedValue>, Option<BoxedValue>, Rc<dyn IServiceProvider>, Option<BoxedValue>) =>
            |target: Option<BoxedValue>, property: Option<BoxedValue>, prov: Rc<dyn IServiceProvider>, value: Option<BoxedValue>| {
                XamlIlRuntimeHelpers::apply_non_matching_markup_extension_v1(
                    target.as_ref(),
                    property.as_ref(),
                    &prov,
                    value.as_ref(),
                )
            },
        static fn CreateInnerServiceProviderV1(Rc<dyn IServiceProvider>) -> Rc<dyn IServiceProvider> =>
            XamlIlRuntimeHelpers::create_inner_service_provider_v1,
        static fn CreateRootServiceProviderV2() -> Rc<dyn IServiceProvider> =>
            XamlIlRuntimeHelpers::create_root_service_provider_v2,
        static fn CreateRootServiceProviderV3(Option<Rc<dyn IServiceProvider>>) -> Rc<dyn IServiceProvider> =>
            XamlIlRuntimeHelpers::create_root_service_provider_v3,
    ],
});
