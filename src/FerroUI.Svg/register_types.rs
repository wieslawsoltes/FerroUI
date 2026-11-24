use crate::SvgImageExtension;
use ferroui_base::metadata::{MarkupAssembly, MarkupType, MarkupTyped, XmlnsDefinition, FERRO_XML_NAMESPACE};

/// The markup of the crate, in the default XML namespace.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroUI.Svg",
    crate_name: "ferroui_svg",
    xmlns_definitions: &[XmlnsDefinition { xml_namespace: FERRO_XML_NAMESPACE, namespace: "FerroUI.Svg" }],
    xmlns_prefixes: &[],
    metadata: &[],
};

ferroui_base::ferro_rust_paths! {
    classes: [],
    types: [crate::SvgImageExtension],
    contracts: [], generics: [], generic_contracts: [],
}

/// Registers `{SvgImage}` for the run-time XAML loader.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        ferroui_markup_xaml::register_types();
        register_rust_paths();
        MarkupType::register_table(&[SvgImageExtension::MARKUP]);
        MarkupAssembly::register_table(&ASSEMBLY);
    });
}
