//! The run-time context of the framework language: the interpreter's
//! context mapped onto the service contracts of the XAML runtime library,
//! and the members the language adds to the context
//! (`FerroXamlIlContextNameScopeField`,
//! `FerroXamlIlContextEagerParentStackProvider`).
//!
//! The IL back end of the managed original generates a context class that
//! implements the contracts; here the interpreter's [`RuntimeContext`]
//! implements them and [`FerroRuntimeContextServices`] hands it out under
//! the handle type of each contract.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::rc::Rc;

use ferroui_base::controls::INameScope;
use ferroui_base::metadata::{service, IServiceProvider, MarkupValue};
use ferroui_base::utilities::Uri;
use ferroui_base::BoxedValue;
use ferroui_markup_xaml::converters::ITypeDescriptorContext;
use ferroui_markup_xaml::xaml_il::runtime::{
    FerroXamlIlXmlNamespaceInfo, IFerroXamlIlEagerParentStackProvider, IFerroXamlIlParentStackProvider,
    IFerroXamlIlXmlNamespaceInfoProvider, XmlNamespaces,
};
use ferroui_markup_xaml::{IProvideValueTarget, IRootObjectProvider, IUriContext, ServiceProviderExtensions};
use xamlx::exceptions::XamlResult;
use xamlx::type_system::IXamlType;

use crate::runtime::interpreter::{
    IRuntimeContextServices, RuntimeContext, RuntimeContextService, XmlNamespaceInfoProvider,
};

impl IRootObjectProvider for RuntimeContext {
    fn root_object(&self) -> Option<BoxedValue> {
        RuntimeContext::root_object(self)
    }

    fn intermediate_root_object(&self) -> Option<BoxedValue> {
        RuntimeContext::intermediate_root_object(self)
    }
}

impl IProvideValueTarget for RuntimeContext {
    fn target_object(&self) -> Option<BoxedValue> {
        RuntimeContext::target_object(self)
    }

    fn target_property(&self) -> Option<BoxedValue> {
        RuntimeContext::target_property(self)
    }
}

impl IUriContext for RuntimeContext {
    fn base_uri(&self) -> Option<Uri> {
        RuntimeContext::base_uri(self)
    }

    fn set_base_uri(&self, value: Option<Uri>) {
        RuntimeContext::set_base_uri(self, value)
    }
}

impl ITypeDescriptorContext for RuntimeContext {}

/// The parent stack provider of the context: the lazily enumerated form.
impl IFerroXamlIlParentStackProvider for RuntimeContext {
    fn parents(&self) -> Vec<BoxedValue> {
        RuntimeContext::parents(self).into_iter().flatten().collect()
    }

    fn as_eager_parent_stack_provider(self: Rc<Self>) -> Option<Rc<dyn IFerroXamlIlEagerParentStackProvider>> {
        Some(self)
    }
}

/// The eager parent stack provider of the context
/// (`FerroXamlIlContextEagerParentStackProvider`).
impl IFerroXamlIlEagerParentStackProvider for RuntimeContext {
    /// `DirectParentsStack => (IReadOnlyList<object>) ParentsStack`.
    fn direct_parents_stack(&self) -> Rc<Vec<BoxedValue>> {
        Rc::new(self.parents_stack().into_iter().flatten().collect())
    }

    /// `ParentProvider => XamlIlRuntimeHelpers.AsEagerParentStackProvider(
    /// _serviceProvider.GetService(typeof(IFerroXamlIlParentStackProvider)))`.
    fn parent_provider(&self) -> Option<Rc<dyn IFerroXamlIlEagerParentStackProvider>> {
        let parent = self.parent_service_provider()?;
        let provider = parent.get_service_of::<Rc<dyn IFerroXamlIlParentStackProvider>>()?;
        Some(ferroui_markup_xaml::xaml_il::runtime::XamlIlRuntimeHelpers::as_eager_parent_stack_provider(provider))
    }
}

/// The name scope field of the context (`FerroXamlIlContextNameScopeField`):
/// filled in the constructor of the context from the parent service
/// provider; null when the parent has no name scope.
pub struct FerroNameScopeField(pub Option<Rc<dyn INameScope>>);

/// The context initialiser that fills the name scope field: the context
/// type builder callback of the language.
pub fn initialize_name_scope_field(context: &Rc<RuntimeContext>) -> XamlResult<()> {
    let scope = context.parent_service_provider().and_then(|parent| parent.get_name_scope());
    context.set_extension(Rc::new(FerroNameScopeField(scope)));
    Ok(())
}

/// The value of the name scope field of a context.
pub fn name_scope_of(context: &RuntimeContext) -> Option<Rc<dyn INameScope>> {
    context.extension::<FerroNameScopeField>().and_then(|field| field.0.clone())
}

struct NamespaceInfoProvider {
    provider: Rc<XmlNamespaceInfoProvider>,
    namespaces: std::cell::OnceCell<XmlNamespaces>,
}

impl IFerroXamlIlXmlNamespaceInfoProvider for NamespaceInfoProvider {
    fn xml_namespaces(&self) -> XmlNamespaces {
        self.namespaces
            .get_or_init(|| {
                let source = self.provider.xml_namespaces();
                let mut namespaces = HashMap::with_capacity(source.len());
                for (prefix, infos) in source.iter() {
                    let infos = infos
                        .iter()
                        .map(|info| {
                            FerroXamlIlXmlNamespaceInfo::with(
                                &info.clr_namespace,
                                info.clr_assembly_name.as_deref().unwrap_or(""),
                            )
                        })
                        .collect();
                    namespaces.insert(prefix.clone(), infos);
                }
                Rc::new(namespaces)
            })
            .clone()
    }
}

/// Maps the run-time context onto the service contracts of the XAML runtime
/// library. Services are identified by their handle types:
/// `Rc<dyn IRootObjectProvider>`, `Rc<dyn IFerroXamlIlParentStackProvider>`,
/// `Rc<dyn ITypeDescriptorContext>`, `Rc<dyn IProvideValueTarget>`,
/// `Rc<dyn IUriContext>` and `Rc<dyn IFerroXamlIlXmlNamespaceInfoProvider>`.
pub struct FerroRuntimeContextServices;

impl IRuntimeContextServices for FerroRuntimeContextServices {
    fn get_own_service(
        &self,
        context: &Rc<RuntimeContext>,
        own: RuntimeContextService,
        service_type: TypeId,
    ) -> Option<Rc<dyn Any>> {
        match own {
            RuntimeContextService::RootObjectProvider => {
                service(service_type, || context.clone() as Rc<dyn IRootObjectProvider>)
            }
            RuntimeContextService::ParentStackProvider => {
                service(service_type, || context.clone() as Rc<dyn IFerroXamlIlParentStackProvider>)
            }
            RuntimeContextService::TypeDescriptorContext => {
                service(service_type, || context.clone() as Rc<dyn ITypeDescriptorContext>)
            }
            RuntimeContextService::ProvideValueTarget => {
                service(service_type, || context.clone() as Rc<dyn IProvideValueTarget>)
            }
            RuntimeContextService::UriContext => service(service_type, || context.clone() as Rc<dyn IUriContext>),
        }
    }

    fn get_namespace_info_service(
        &self,
        provider: &Rc<XmlNamespaceInfoProvider>,
        service_type: TypeId,
    ) -> Option<Rc<dyn Any>> {
        service(service_type, || {
            Rc::new(NamespaceInfoProvider { provider: provider.clone(), namespaces: std::cell::OnceCell::new() })
                as Rc<dyn IFerroXamlIlXmlNamespaceInfoProvider>
        })
    }

    fn get_parent_root_object(&self, parent: &Rc<dyn IServiceProvider>) -> Option<MarkupValue> {
        parent.get_service_of::<Rc<dyn IRootObjectProvider>>().map(|provider| provider.root_object())
    }

    fn get_parent_stack(&self, parent: &Rc<dyn IServiceProvider>) -> Option<Vec<MarkupValue>> {
        parent
            .get_service_of::<Rc<dyn IFerroXamlIlParentStackProvider>>()
            .map(|provider| provider.parents().into_iter().map(Some).collect())
    }

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
