use crate::icons::ICON_SOURCE_TYPES;
use crate::{AnimatedIcon, AnimatedIconSource, AnimatedVisualPlayer, AnimatedVisualSource, LottieVisualSource};
use ferroui_base::metadata::{MarkupAssembly, XmlnsDefinition, FERRO_XML_NAMESPACE};
use ferroui_base::TypeInfo;

pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroUI.AnimatedVisuals",
    crate_name: "ferroui_animated_visuals",
    xmlns_definitions: &[XmlnsDefinition {
        xml_namespace: FERRO_XML_NAMESPACE,
        namespace: "FerroUI.AnimatedVisuals",
    }],
    xmlns_prefixes: &[],
    metadata: &[],
};

ferroui_base::ferro_rust_paths! {
    classes: [
        crate::AnimatedIcon,
        crate::AnimatedIconSource,
        crate::AnimatedVisualPlayer,
        crate::AnimatedVisualSource,
        crate::LottieVisualSource,
        crate::AnimatedAcceptVisualSource,
        crate::AnimatedBackVisualSource,
        crate::AnimatedChevronDownSmallVisualSource,
        crate::AnimatedChevronRightDownSmallVisualSource,
        crate::AnimatedChevronUpDownSmallVisualSource,
        crate::AnimatedFindVisualSource,
        crate::AnimatedGlobalNavigationButtonVisualSource,
        crate::AnimatedSettingsVisualSource,
    ],
    types: [], contracts: [], generics: [], generic_contracts: [],
}

/// Registers the namespace and the types of animated visuals for the runtime XAML loader.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        ferroui_controls::register_types();
        TypeInfo::register_namespaces(&[("ferroui_animated_visuals", "FerroUI.AnimatedVisuals")]);
        TypeInfo::register_table(&[
            AnimatedIcon::TYPE,
            AnimatedIconSource::TYPE,
            AnimatedVisualPlayer::TYPE,
            AnimatedVisualSource::TYPE,
            LottieVisualSource::TYPE,
        ]);
        TypeInfo::register_table(ICON_SOURCE_TYPES);
        MarkupAssembly::register_table(&ASSEMBLY);
        register_rust_paths();
        crate::ring::install();
    });
}
