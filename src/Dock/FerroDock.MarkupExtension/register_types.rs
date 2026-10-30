use crate::{LoadExtension, ReferenceExtension};
use ferroui_base::metadata::{
    MarkupAssembly, MarkupType, MarkupTyped, XmlnsDefinition, FERRO_XML_NAMESPACE,
};
/// Original default-framework XML namespace mapping for this assembly.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroDock.MarkupExtension",
    crate_name: "ferrodock_markup_extension",
    xmlns_definitions: &[XmlnsDefinition {
        xml_namespace: FERRO_XML_NAMESPACE,
        namespace: "FerroUI.MarkupExtension",
    }],
    xmlns_prefixes: &[],
    metadata: &[],
};
ferroui_base::ferro_rust_paths! {classes:[],types:[crate::LoadExtension,crate::ReferenceExtension],contracts:[],generics:[],generic_contracts:[]}
/// Registers both original extensions with the native markup metadata system.
pub fn register_types() {
    ferroui_markup_xaml::register_types();
    register_rust_paths();
    MarkupType::register_all(&[LoadExtension::MARKUP, ReferenceExtension::MARKUP]);
    MarkupAssembly::register(&ASSEMBLY);
}
