//! Theme assembly and compiled markup registration.
use crate::DockFluentTheme;
use ferroui_base::metadata::{MarkupAssembly, XmlnsDefinition, FERRO_XML_NAMESPACE};
use ferroui_base::TypeInfo;
const NAMESPACES: &[(&str, &str)] = &[(
    "ferrodock_ferro_themes_fluent::dock_fluent_theme",
    "FerroDock.Ferro.Themes.Fluent",
)];
/// Source theme assembly identity and default framework XML namespace.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroDock.Ferro.Themes.Fluent",
    crate_name: "ferrodock_ferro_themes_fluent",
    xmlns_definitions: &[XmlnsDefinition {
        xml_namespace: FERRO_XML_NAMESPACE,
        namespace: "FerroDock.Ferro.Themes.Fluent",
    }],
    xmlns_prefixes: &[],
    metadata: &[],
};
ferroui_base::ferro_rust_paths! {classes:[crate::DockFluentTheme],types:[],contracts:[],generics:[],generic_contracts:[]}
/// Registers native docking types, this theme and its compiled resources.
pub fn register_types() {
    ferroui_markup_xaml::register_types();
    ferrodock_ferro::register_types();
    TypeInfo::register_namespaces(NAMESPACES);
    TypeInfo::register_all(&[DockFluentTheme::TYPE]);
    register_rust_paths();
    MarkupAssembly::register(&ASSEMBLY);
    crate::compiled_markup::register();
}
