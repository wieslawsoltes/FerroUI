//! The run-time context of the framework language: the interpreter's
//! context mapped onto the service contracts of the XAML runtime library,
//! and the members the language adds to the context
//! (`FerroXamlIlContextNameScopeField`,
//! `FerroXamlIlContextEagerParentStackProvider`).
//!
//! The IL back end of the managed original generates a context class that
//! implements the contracts; here the context of the runtime library
//! ([`RuntimeContext`], `XamlIlContext`) implements them, and
//! [`FerroRuntimeContextServices`] hands it out under the handle type of each
//! contract (through `FrameworkContextServices`, which generated code uses
//! too) and adds the namespace information of a parsed document.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::rc::Rc;

use ferroui_base::controls::INameScope;
use ferroui_base::metadata::{service, IServiceProvider, MarkupValue};
use ferroui_base::BoxedValue;
use ferroui_markup_xaml::converters::ITypeDescriptorContext;
use ferroui_markup_xaml::xaml_il::runtime::{
    FerroXamlIlXmlNamespaceInfo, FrameworkContextServices, IFerroXamlIlXmlNamespaceInfoProvider, IXamlIlContextServices,
    XmlNamespaces,
};
use ferroui_markup_xaml::ServiceProviderExtensions;
use xamlx::exceptions::XamlResult;
use xamlx::type_system::IXamlType;

use crate::runtime::interpreter::{
    IRuntimeContextServices, RuntimeContext, RuntimeContextService, XmlNamespaceInfoProvider,
};

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

impl IXamlIlContextServices for FerroRuntimeContextServices {
    fn get_own_service(
        &self,
        context: &Rc<RuntimeContext>,
        own: RuntimeContextService,
        service_type: TypeId,
    ) -> Option<Rc<dyn Any>> {
        FrameworkContextServices.get_own_service(context, own, service_type)
    }

    fn get_parent_root_object(&self, parent: &Rc<dyn IServiceProvider>) -> Option<MarkupValue> {
        FrameworkContextServices.get_parent_root_object(parent)
    }

    fn get_parent_stack(&self, parent: &Rc<dyn IServiceProvider>) -> Option<Vec<MarkupValue>> {
        FrameworkContextServices.get_parent_stack(parent)
    }
}

impl IRuntimeContextServices for FerroRuntimeContextServices {
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
