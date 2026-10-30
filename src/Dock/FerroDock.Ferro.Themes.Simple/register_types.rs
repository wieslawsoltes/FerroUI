//! Theme assembly and compiled markup registration.
use crate::DockSimpleTheme;
use ferroui_base::metadata::{MarkupAssembly, XmlnsDefinition, FERRO_XML_NAMESPACE};
use ferroui_base::TypeInfo;
const NAMESPACES: &[(&str, &str)] = &[(
    "ferrodock_ferro_themes_simple::dock_simple_theme",
    "FerroDock.Ferro.Themes.Simple",
)];
/// Source theme assembly identity and default framework XML namespace.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroDock.Ferro.Themes.Simple",
    crate_name: "ferrodock_ferro_themes_simple",
    xmlns_definitions: &[XmlnsDefinition {
        xml_namespace: FERRO_XML_NAMESPACE,
        namespace: "FerroDock.Ferro.Themes.Simple",
    }],
    xmlns_prefixes: &[],
    metadata: &[],
};
ferroui_base::ferro_rust_paths! {classes:[crate::DockSimpleTheme],types:[],contracts:[],generics:[],generic_contracts:[]}
/// Registers native docking types, this theme and its compiled resources.
pub fn register_types() {
    ferroui_markup_xaml::register_types();
    ferrodock_ferro::register_types();
    TypeInfo::register_namespaces(NAMESPACES);
    TypeInfo::register_all(&[DockSimpleTheme::TYPE]);
    register_rust_paths();
    MarkupAssembly::register(&ASSEMBLY);
    crate::compiled_markup::register();
}
