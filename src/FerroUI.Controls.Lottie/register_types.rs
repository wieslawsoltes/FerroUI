use crate::{CompositionAnimatedControl, Lottie};
use ferroui_base::TypeInfo;
use ferroui_base::metadata::{MarkupAssembly, XmlnsDefinition};

pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroUI.Controls.Lottie",
    crate_name: "ferroui_controls_lottie",
    xmlns_definitions: &[XmlnsDefinition {
        xml_namespace: "https://ferroui.dev/lottie",
        namespace: "FerroUI.Controls.Lottie",
    }],
    xmlns_prefixes: &[],
    metadata: &[],
};

ferroui_base::ferro_rust_paths! {
    classes: [crate::CompositionAnimatedControl, crate::Lottie],
    types: [], contracts: [], generics: [], generic_contracts: [],
}

/// Registers the Lottie namespace and types for the runtime XAML loader.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        ferroui_controls::register_types();
        TypeInfo::register_namespaces(&[("ferroui_controls_lottie", "FerroUI.Controls.Lottie")]);
        TypeInfo::register_table(&[CompositionAnimatedControl::TYPE, Lottie::TYPE]);
        MarkupAssembly::register_table(&ASSEMBLY);
        register_rust_paths();
    });
}
