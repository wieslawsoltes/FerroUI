use crate::{ControlRecycling, ControlRecyclingDataTemplate};
use ferroui_base::metadata::{MarkupAssembly, XmlnsDefinition, FERRO_XML_NAMESPACE};
use ferroui_base::{StaticType, TypeInfo};
const NAMESPACES: &[(&str, &str)] =
    &[("ferrodock_controls_recycling", "FerroUI.Controls.Recycling")];
/// The native library identity and source framework namespace mapping.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroDock.Controls.Recycling",
    crate_name: "ferrodock_controls_recycling",
    xmlns_definitions: &[XmlnsDefinition {
        xml_namespace: FERRO_XML_NAMESPACE,
        namespace: "FerroUI.Controls.Recycling",
    }],
    xmlns_prefixes: &[],
    metadata: &[],
};
ferroui_base::ferro_rust_paths! {classes:[crate::ControlRecycling,crate::ControlRecyclingDataTemplate],types:[],contracts:[],generics:[],generic_contracts:[]}
/// Registers native recycling classes and their metadata with the shared framework.
pub fn register_types() {
    ferroui_controls::register_types();
    ferrodock_controls_recycling_model_ferro::register_types();
    ferrodock_runtime::register_value_type::<ferroui_base::Ref<ferroui_controls::Control>>();
    TypeInfo::register_namespaces(NAMESPACES);
    TypeInfo::register_all(&[
        <ControlRecycling as StaticType>::TYPE,
        <ControlRecyclingDataTemplate as StaticType>::TYPE,
    ]);
    register_rust_paths();
    MarkupAssembly::register(&ASSEMBLY);
}
