//! The type table of this crate: its namespace, its classes and what it
//! states about itself for markup, so that XAML names `ComponentHost`
//! (`using:FerroUI.View`), at run time and to the markup compiler.

use crate::ComponentHost;
use ferroui_base::metadata::MarkupAssembly;
use ferroui_base::TypeInfo;

/// The dotted namespace of the modules of this crate.
const NAMESPACES: &[(&str, &str)] = &[("ferroui_view", "FerroUI.View")];

/// What this crate states about itself for markup: its assembly name. It
/// maps no XML namespace; documents name it with `using:FerroUI.View`.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroUI.View",
    crate_name: "ferroui_view",
    xmlns_definitions: &[],
    xmlns_prefixes: &[],
    metadata: &[],
};

const TYPES: &[&TypeInfo] = &[ComponentHost::TYPE];

/// Registers the types of this crate for markup: XAML then names
/// `ComponentHost` as `using:FerroUI.View` (or
/// `clr-namespace:FerroUI.View;assembly=FerroUI.View`). Cheap and
/// idempotent.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        TypeInfo::register_namespaces(NAMESPACES);
        TypeInfo::register_all(TYPES);
        MarkupAssembly::register(&ASSEMBLY);
    });
}
