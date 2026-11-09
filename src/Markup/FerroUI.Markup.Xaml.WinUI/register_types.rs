use ferroui_base::metadata::MarkupAssembly;
use ferroui_base::TypeInfo;

/// The namespace of the markup types of the crate (`using:FerroUI.Markup.Xaml.WinUI`; not in the
/// default XML namespace). The crate declares none since the x:Bind adapter was removed (#525,
/// X6): an imported x:Bind is the `XBindExtension` of `ferroui-markup-xaml`.
const NAMESPACES: &[(&str, &str)] = &[("ferroui_markup_xaml_winui", "FerroUI.Markup.Xaml.WinUI")];

/// The markup assembly of the crate.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroUI.Markup.Xaml.WinUI",
    crate_name: "ferroui_markup_xaml_winui",
    xmlns_definitions: &[],
    xmlns_prefixes: &[],
    metadata: &[],
};

/// The classes of the object model of the crate: none.
const TYPES: &[&TypeInfo] = &[];

/// Registers the crate for markup, after the XAML runtime library, whose types (the x:Bind core
/// among them) a migrated document names.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        ferroui_markup_xaml::register_types();
        TypeInfo::register_namespaces(NAMESPACES);
        TypeInfo::register_table(TYPES);
        crate::rust_paths::register_rust_paths();
        MarkupAssembly::register_table(&ASSEMBLY);
    });
}
