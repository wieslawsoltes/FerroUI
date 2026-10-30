use ferroui_base::metadata::MarkupAssembly;
use ferroui_base::TypeInfo;

/// The namespace of the markup types of the crate (`using:FerroUI.Markup.Xaml.WinUI`, which the
/// importer writes; not in the default XML namespace).
const NAMESPACES: &[(&str, &str)] = &[("ferroui_markup_xaml_winui", "FerroUI.Markup.Xaml.WinUI")];

/// The markup assembly of the crate.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroUI.Markup.Xaml.WinUI",
    crate_name: "ferroui_markup_xaml_winui",
    xmlns_definitions: &[],
    xmlns_prefixes: &[],
    metadata: &[],
};

/// The classes of the object model of the crate: none (its markup types are in `bindings`).
const TYPES: &[&TypeInfo] = &[];

/// Registers the x:Bind types for markup, after the XAML runtime library they build on.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        ferroui_markup_xaml::register_types();
        TypeInfo::register_namespaces(NAMESPACES);
        TypeInfo::register_all(TYPES);
        crate::rust_paths::register_rust_paths();
        MarkupAssembly::register(&ASSEMBLY);
        crate::bindings::register_markup_types();
    });
}
