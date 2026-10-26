use crate::{
    FExtension, FormatExtension, TExtension, TranslateExtension, Translation,
    TranslationBindingSource,
};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::{
    MarkupAssembly, MarkupType, MarkupTyped, XmlnsDefinition, FERRO_XML_NAMESPACE,
};
use ferroui_base::TypeInfo;
use std::rc::Rc;

const NAMESPACES: &[(&str, &str)] = &[("ferroui_protranslate", "FerroUI.ProTranslate")];

/// ProTranslate's shared XML namespace, also available in FerroUI's default namespace.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroUI.ProTranslate",
    crate_name: "ferroui_protranslate",
    xmlns_definitions: &[
        XmlnsDefinition {
            xml_namespace: "https://github.com/protranslate/xaml",
            namespace: "FerroUI.ProTranslate",
        },
        XmlnsDefinition {
            xml_namespace: FERRO_XML_NAMESPACE,
            namespace: "FerroUI.ProTranslate",
        },
    ],
    xmlns_prefixes: &[],
    metadata: &[],
};

const TYPES: &[&TypeInfo] = &[Translation::TYPE];
const MARKUP_TYPES: &[&MarkupType] = &[
    <Translation as MarkupTyped>::MARKUP,
    <TranslateExtension as MarkupTyped>::MARKUP,
    <TExtension as MarkupTyped>::MARKUP,
    <FormatExtension as MarkupTyped>::MARKUP,
    <FExtension as MarkupTyped>::MARKUP,
    <TranslationBindingSource as MarkupTyped>::MARKUP,
];

/// Registers native property metadata and markup extensions for both runtime
/// and compiled XAML. Registration is idempotent and needs no loader modifications.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        ferroui_markup_xaml::register_types();
        TypeInfo::register_namespaces(NAMESPACES);
        TypeInfo::register_all(TYPES);
        MarkupType::register_all(MARKUP_TYPES);
        MarkupAssembly::register(&ASSEMBLY);
        crate::rust_paths::register_rust_paths();
        ValueTypes::register_global(register_value_types);
    });
}

fn register_value_types() {
    ValueTypes::register_nullable::<TranslationBindingSource>();
    ValueTypes::register_reference::<TranslateExtension>();
    ValueTypes::register_reference::<TExtension>();
    ValueTypes::register_reference::<FormatExtension>();
    ValueTypes::register_reference::<FExtension>();
    ValueTypes::register_cast::<Rc<TExtension>, Rc<TranslateExtension>>(|value| value.base());
    ValueTypes::register_cast::<Rc<FormatExtension>, Rc<TranslateExtension>>(|value| value.base());
    ValueTypes::register_cast::<Rc<FExtension>, Rc<FormatExtension>>(|value| value.base());
}
