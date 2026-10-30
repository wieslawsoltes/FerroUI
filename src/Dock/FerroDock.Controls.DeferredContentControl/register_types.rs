//! Source assembly, namespace and native metadata registration.
use crate::{DeferredContentControl, DeferredContentPresenter, DeferredContentScheduling};
use crate::{DeferredContentPresentationBudgetMode, DeferredContentPresentationTimeline};
use ferroui_base::metadata::{
    MarkupAssembly, MarkupType, MarkupTyped, XmlnsDefinition, FERRO_XML_NAMESPACE,
};
use ferroui_base::{StaticType, TypeInfo};
const NAMESPACES: &[(&str, &str)] = &[(
    "ferrodock_controls_deferred_content_control",
    "FerroDock.Controls.DeferredContentControl",
)];
/// The deferred-content library identity and original framework XML namespace mapping.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroDock.Controls.DeferredContentControl",
    crate_name: "ferrodock_controls_deferred_content_control",
    xmlns_definitions: &[XmlnsDefinition {
        xml_namespace: FERRO_XML_NAMESPACE,
        namespace: "FerroDock.Controls.DeferredContentControl",
    }],
    xmlns_prefixes: &[],
    metadata: &[],
};
ferroui_base::ferro_rust_paths! {classes:[crate::DeferredContentControl,crate::DeferredContentPresenter],types:[crate::DeferredContentPresentationSettings,crate::DeferredContentScheduling,crate::DeferredContentPresentationBudgetMode,crate::DeferredContentPresentationTimeline],contracts:[crate::IDeferredContentPresentation],generics:[],generic_contracts:[]}
/// Registers native presentation metadata in the shared framework type system.
pub fn register_types() {
    ferroui_controls::register_types();
    ferroui_base::data::core::ValueTypes::register_nullable::<
        std::rc::Rc<DeferredContentPresentationTimeline>,
    >();
    TypeInfo::register_namespaces(NAMESPACES);
    TypeInfo::register_all(&[
        <DeferredContentControl as StaticType>::TYPE,
        <DeferredContentPresenter as StaticType>::TYPE,
        <DeferredContentScheduling as StaticType>::TYPE,
    ]);
    register_rust_paths();
    MarkupType::register_all(&[
        <crate::DeferredContentPresentationSettings as MarkupTyped>::MARKUP,
        <dyn crate::IDeferredContentPresentation as MarkupTyped>::MARKUP,
        <DeferredContentPresentationBudgetMode as MarkupTyped>::MARKUP,
        <DeferredContentPresentationTimeline as MarkupTyped>::MARKUP,
    ]);
    MarkupAssembly::register(&ASSEMBLY);
}
