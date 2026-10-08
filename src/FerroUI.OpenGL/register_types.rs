//! The type table of this crate: its namespaces, its classes and what it
//! states about itself for markup.

use crate::controls::OpenGlControlBase;
use ferroui_base::metadata::MarkupAssembly;
use ferroui_base::TypeInfo;

/// The dotted namespaces of the modules of this crate.
const NAMESPACES: &[(&str, &str)] =
    &[("ferroui_opengl", "FerroUI.OpenGL"), ("ferroui_opengl::controls", "FerroUI.OpenGL.Controls")];

/// What this crate states about itself for markup: its assembly name. The
/// upstream project maps no XML namespace; documents name the namespace of
/// the control with `using:`.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroUI.OpenGL",
    crate_name: "ferroui_opengl",
    xmlns_definitions: &[],
    xmlns_prefixes: &[],
    metadata: &[],
};

const TYPES: &[&TypeInfo] = &[
    // FerroUI.OpenGL.Controls
    OpenGlControlBase::TYPE,
];

/// Registers the namespaces, the types and the assembly of this crate (and
/// of the crates it is built on). Cheap and idempotent.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        ferroui_controls::register_types();
        TypeInfo::register_namespaces(NAMESPACES);
        TypeInfo::register_all(TYPES);
        crate::rust_paths::register_rust_paths();
        MarkupAssembly::register(&ASSEMBLY);
    });
}
