//! Class, namespace and compiled resource registration for this library.
use crate::{ProportionalStackPanel, ProportionalStackPanelSplitter};
use ferroui_base::metadata::{MarkupAssembly, XmlnsDefinition, FERRO_XML_NAMESPACE};
use ferroui_base::{StaticType, TypeInfo};
const NAMESPACES: &[(&str, &str)] = &[(
    "ferrodock_controls_proportional_stack_panel",
    "FerroDock.Controls.ProportionalStackPanel",
)];
/// The source library identity and framework XML namespace mapping.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroDock.Controls.ProportionalStackPanel",
    crate_name: "ferrodock_controls_proportional_stack_panel",
    xmlns_definitions: &[XmlnsDefinition {
        xml_namespace: FERRO_XML_NAMESPACE,
        namespace: "FerroDock.Controls.ProportionalStackPanel",
    }],
    xmlns_prefixes: &[],
    metadata: &[("FerroUseCompiledBindingsByDefault", "True")],
};
ferroui_base::ferro_rust_paths! {classes:[crate::ProportionalStackPanel,crate::ProportionalStackPanelSplitter],types:[],contracts:[],generics:[],generic_contracts:[]}
/// Registers the native classes and the library's compiled splitter resource.
pub fn register_types() {
    ferroui_controls::register_types();
    ferroui_markup_xaml::register_types();
    TypeInfo::register_namespaces(NAMESPACES);
    TypeInfo::register_all(&[
        <ProportionalStackPanel as StaticType>::TYPE,
        <ProportionalStackPanelSplitter as StaticType>::TYPE,
    ]);
    register_rust_paths();
    MarkupAssembly::register(&ASSEMBLY);
    crate::compiled_markup::register();
}
