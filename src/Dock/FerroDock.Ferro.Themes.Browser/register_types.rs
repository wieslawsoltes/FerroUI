//! Theme assembly and compiled markup registration.
use crate::BrowserTabTheme;
use ferroui_base::metadata::{MarkupAssembly, XmlnsDefinition, FERRO_XML_NAMESPACE};
use ferroui_base::TypeInfo;
const NAMESPACES: &[(&str, &str)] = &[(
    "ferrodock_ferro_themes_browser::browser_tab_theme",
    "FerroDock.Ferro.Themes.Browser",
)];
/// Source theme assembly identity and default framework XML namespace.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroDock.Ferro.Themes.Browser",
    crate_name: "ferrodock_ferro_themes_browser",
    xmlns_definitions: &[
        XmlnsDefinition {
            xml_namespace: FERRO_XML_NAMESPACE,
            namespace: "FerroDock.Ferro.Themes.Browser",
        },
        XmlnsDefinition {
            xml_namespace: FERRO_XML_NAMESPACE,
            namespace: "FerroUI.Markup.Xaml.MarkupExtensions",
        },
    ],
    xmlns_prefixes: &[],
    metadata: &[],
};
ferroui_base::ferro_rust_paths! {classes:[crate::BrowserTabTheme],types:[crate::OnPlatformThicknessMetadata,crate::OnPlatformDoubleMetadata],contracts:[],generics:[],generic_contracts:[]}
/// Registers native docking types, this theme and its compiled resources.
pub fn register_types() {
    ferroui_markup_xaml::register_types();
    ferrodock_ferro::register_types();
    ferrodock_ferro_themes_fluent::register_types();
    TypeInfo::register_namespaces(NAMESPACES);
    TypeInfo::register_all(&[BrowserTabTheme::TYPE]);
    register_rust_paths();
    use ferroui_base::{
        data::core::ValueTypes,
        metadata::{MarkupType, MarkupTyped},
    };
    type NativePlatform =
        ferroui_markup_xaml::markup_extensions::OnPlatformExtensionOf<ferroui_base::Thickness>;
    ValueTypes::register_reference::<NativePlatform>();
    MarkupType::register_all(&[
        crate::OnPlatformThicknessMetadata::MARKUP,
        crate::OnPlatformDoubleMetadata::MARKUP,
    ]);
    ValueTypes::register_reference::<
        ferroui_markup_xaml::markup_extensions::OnPlatformExtensionOf<f64>,
    >();
    ferroui_base::__ferro_compiler_metadata! {
        ferroui_base::metadata::register_type_rust_paths(&[(||std::any::TypeId::of::<std::rc::Rc<NativePlatform>>(),"::std::rc::Rc<::ferroui_markup_xaml::markup_extensions::OnPlatformExtensionOf<::ferroui_base::Thickness>>"),(||std::any::TypeId::of::<std::rc::Rc<ferroui_markup_xaml::markup_extensions::OnPlatformExtensionOf<f64>>>(),"::std::rc::Rc<::ferroui_markup_xaml::markup_extensions::OnPlatformExtensionOf<f64>>")]);
    }

    MarkupAssembly::register(&ASSEMBLY);
    crate::compiled_markup::register();
}
