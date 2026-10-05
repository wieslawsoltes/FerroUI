//! The runtime helpers compiled (and interpreted) markup calls, and the
//! contracts of the runtime context of a document.

mod i_ferro_xaml_il_control_template_provider;
mod i_ferro_xaml_il_parent_stack_provider;
mod i_ferro_xaml_il_xml_namespace_info_provider_v1;
mod xaml_il_parent_stack_provider_wrapper;
mod xaml_il_runtime_helpers;

pub use i_ferro_xaml_il_control_template_provider::IFerroXamlIlControlTemplateProvider;
pub use i_ferro_xaml_il_parent_stack_provider::{
    IFerroXamlIlEagerParentStackProvider, IFerroXamlIlParentStackProvider,
};
pub use i_ferro_xaml_il_xml_namespace_info_provider_v1::{
    FerroXamlIlXmlNamespaceInfo, IFerroXamlIlXmlNamespaceInfoProvider, XmlNamespaces,
};
pub(crate) use xaml_il_parent_stack_provider_wrapper::XamlIlParentStackProviderWrapper;
pub use xaml_il_runtime_helpers::{
    DeferredContent, DeferredContentBuilder, DeferredResult, RuntimePlatformNotRegistered, XamlIlRuntimeHelpers,
};

#[cfg(test)]
mod xaml_il_runtime_helpers_tests;
