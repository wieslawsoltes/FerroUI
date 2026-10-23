//! What the crate states about itself for markup, and the loader table of its compiled markup:
//! `ferres://ViewInterop/Details.xaml` loads the Rust code the build compiled the document to.

use ferroui_base::metadata::MarkupAssembly;
use ferroui_base::TypeInfo;

/// The dotted namespace of the modules of this crate.
const NAMESPACES: &[(&str, &str)] = &[("view_interop", "ViewInterop")];

/// What this crate states about itself for markup.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "ViewInterop",
    crate_name: "view_interop",
    xmlns_definitions: &[],
    xmlns_prefixes: &[],
    metadata: &[],
};

/// Registers the types of the markup runtime and of `ferroui-view`, the namespace and the
/// assembly of the crate, and the compiled markup of its document. Cheap and idempotent.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        ferroui_markup_xaml::register_types();
        ferroui_view::register_types();
        TypeInfo::register_namespaces(NAMESPACES);
        MarkupAssembly::register(&ASSEMBLY);
        crate::compiled_markup::register();
    });
}
