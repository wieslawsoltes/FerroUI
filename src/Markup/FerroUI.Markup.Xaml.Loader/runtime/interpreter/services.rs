//! A self-contained set of the service contracts of the run-time context
//! (the counterparts of the runtime library's `IProvideValueTarget`,
//! `IRootObjectProvider`, `IUriContext` and of the parent stack and
//! namespace information providers of `XamlX.Runtime/Interfaces.cs`), and
//! the adapter that maps the context onto them.
//!
//! The markup runtime crate declares the contracts user code is written
//! against; an integration supplies its own [`IRuntimeContextServices`] for
//! those. These are what the interpreter uses when none is supplied.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::rc::Rc;

use ferroui_base::metadata::{service, IServiceProvider, MarkupType, MarkupTyped, MarkupValue};
use ferroui_base::utilities::Uri;
use ferroui_base::{ferro_markup_type, BoxedValue};

use crate::runtime::type_system::ITypeDescriptorContext;

use super::runtime_context::{
    IRuntimeContextServices, RuntimeContext, RuntimeContextService, XamlXmlNamespaceInfo, XmlNamespaceInfoProvider,
};

/// Provides the root object of the document being built.
pub trait IRootObjectProvider {
    /// The root object of the document.
    fn root_object(&self) -> MarkupValue;
    /// The root of the deferred content being built.
    fn intermediate_root_object(&self) -> MarkupValue;
}

/// Provides the object and property a markup extension provides its value
/// for.
pub trait IProvideValueTarget {
    fn target_object(&self) -> MarkupValue;
    /// The property: its name, or what the language's target property hook
    /// evaluates to.
    fn target_property(&self) -> MarkupValue;
}

/// Provides the base URI of the document being built.
pub trait IUriContext {
    fn base_uri(&self) -> Option<Uri>;
    fn set_base_uri(&self, value: Option<Uri>);
}

/// Provides the objects being initialised around the current one
/// (`IXamlParentStackProviderV1`).
pub trait IXamlParentStackProvider {
    /// The parents, nearest first.
    fn parents(&self) -> Vec<MarkupValue>;
}

/// Provides what the XML namespace prefixes of the document resolve to
/// (`IXamlXmlNamespaceInfoProviderV1`).
pub trait IXamlXmlNamespaceInfoProvider {
    fn xml_namespaces(&self) -> Rc<HashMap<String, Vec<XamlXmlNamespaceInfo>>>;
}

impl IRootObjectProvider for RuntimeContext {
    fn root_object(&self) -> MarkupValue {
        RuntimeContext::root_object(self)
    }
    fn intermediate_root_object(&self) -> MarkupValue {
        RuntimeContext::intermediate_root_object(self)
    }
}

impl IProvideValueTarget for RuntimeContext {
    fn target_object(&self) -> MarkupValue {
        RuntimeContext::target_object(self)
    }
    fn target_property(&self) -> MarkupValue {
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

impl IXamlParentStackProvider for RuntimeContext {
    fn parents(&self) -> Vec<MarkupValue> {
        RuntimeContext::parents(self)
    }
}

impl IXamlXmlNamespaceInfoProvider for XmlNamespaceInfoProvider {
    fn xml_namespaces(&self) -> Rc<HashMap<String, Vec<XamlXmlNamespaceInfo>>> {
        XmlNamespaceInfoProvider::xml_namespaces(self)
    }
}

/// Maps the run-time context onto the contracts of this module. Services
/// are identified by their handle types: `Rc<dyn IRootObjectProvider>`,
/// `Rc<dyn IXamlParentStackProvider>`, `Rc<dyn ITypeDescriptorContext>`,
/// `Rc<dyn IProvideValueTarget>`, `Rc<dyn IUriContext>` and
/// `Rc<dyn IXamlXmlNamespaceInfoProvider>`.
pub struct DefaultRuntimeContextServices;

impl IRuntimeContextServices for DefaultRuntimeContextServices {
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
                service(service_type, || context.clone() as Rc<dyn IXamlParentStackProvider>)
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
        service(service_type, || provider.clone() as Rc<dyn IXamlXmlNamespaceInfoProvider>)
    }

    fn get_parent_root_object(&self, parent: &Rc<dyn IServiceProvider>) -> Option<MarkupValue> {
        parent.get_service_of::<Rc<dyn IRootObjectProvider>>().map(|provider| provider.root_object())
    }

    fn get_parent_stack(&self, parent: &Rc<dyn IServiceProvider>) -> Option<Vec<MarkupValue>> {
        parent.get_service_of::<Rc<dyn IXamlParentStackProvider>>().map(|provider| provider.parents())
    }
}

/// The namespace the contracts of this module are known by in markup.
pub const SERVICES_NAMESPACE: &str = "FerroUI.Markup.Xaml.Loader.Runtime";

ferro_markup_type!(interface dyn IRootObjectProvider as "IRootObjectProvider" {
    handles: [Rc<dyn IRootObjectProvider>, Option<Rc<dyn IRootObjectProvider>>],
    this: Rc<dyn IRootObjectProvider>,
    namespace: "FerroUI.Markup.Xaml.Loader.Runtime",
    properties: [
        RootObject: Option<BoxedValue> { get: |p: &Rc<dyn IRootObjectProvider>| p.root_object() },
        IntermediateRootObject: Option<BoxedValue> {
            get: |p: &Rc<dyn IRootObjectProvider>| p.intermediate_root_object()
        },
    ],
});

ferro_markup_type!(interface dyn IProvideValueTarget as "IProvideValueTarget" {
    handles: [Rc<dyn IProvideValueTarget>, Option<Rc<dyn IProvideValueTarget>>],
    this: Rc<dyn IProvideValueTarget>,
    namespace: "FerroUI.Markup.Xaml.Loader.Runtime",
    properties: [
        TargetObject: Option<BoxedValue> { get: |p: &Rc<dyn IProvideValueTarget>| p.target_object() },
        TargetProperty: Option<BoxedValue> { get: |p: &Rc<dyn IProvideValueTarget>| p.target_property() },
    ],
});

ferro_markup_type!(interface dyn IUriContext as "IUriContext" {
    handles: [Rc<dyn IUriContext>, Option<Rc<dyn IUriContext>>],
    this: Rc<dyn IUriContext>,
    namespace: "FerroUI.Markup.Xaml.Loader.Runtime",
    properties: [
        BaseUri: Option<Uri> {
            get: |p: &Rc<dyn IUriContext>| p.base_uri(),
            set: |p: &Rc<dyn IUriContext>, value: Option<Uri>| p.set_base_uri(value)
        },
    ],
});

ferro_markup_type!(interface dyn IXamlParentStackProvider as "IXamlParentStackProvider" {
    handles: [Rc<dyn IXamlParentStackProvider>, Option<Rc<dyn IXamlParentStackProvider>>],
    this: Rc<dyn IXamlParentStackProvider>,
    namespace: "FerroUI.Markup.Xaml.Loader.Runtime",
});

ferro_markup_type!(interface dyn IXamlXmlNamespaceInfoProvider as "IXamlXmlNamespaceInfoProvider" {
    handles: [Rc<dyn IXamlXmlNamespaceInfoProvider>, Option<Rc<dyn IXamlXmlNamespaceInfoProvider>>],
    this: Rc<dyn IXamlXmlNamespaceInfoProvider>,
    namespace: "FerroUI.Markup.Xaml.Loader.Runtime",
});

/// Makes the contracts of this module known to the type system, so that a
/// language can map them (`RootObjectProvider`, `ProvideValueTarget`, ...).
/// Cheap and idempotent.
pub fn register_service_types() {
    MarkupType::register_all(&[
        <dyn IRootObjectProvider as MarkupTyped>::MARKUP,
        <dyn IProvideValueTarget as MarkupTyped>::MARKUP,
        <dyn IUriContext as MarkupTyped>::MARKUP,
        <dyn IXamlParentStackProvider as MarkupTyped>::MARKUP,
        <dyn IXamlXmlNamespaceInfoProvider as MarkupTyped>::MARKUP,
    ]);
}
