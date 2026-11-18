//! Test fixture of the XAML compiler (#849): an application, the assembly `GeneratedRust.Consumer`,
//! whose compiled documents construct classes of other crates and whose documents give generated
//! items one established name. Not a port of an upstream project.
//!
//! The build script (`build.rs`, `ferroui-build`) generates the compiled markup into `OUT_DIR` and
//! rustc compiles it with the crate, so the fixture is the generated Rust itself:
//!
//! - a class of another crate is constructed with `with_parts` only when its type model says it can
//!   be constructed from selected units; an application class whose markup is partitioned into
//!   units, and a theme whose constructor from units has another name, are constructed with
//!   `new()`;
//! - two themes whose full names differ in case and punctuation only get parts tables of two
//!   names, and every theme of the framework keeps the established name of its table;
//! - two public documents of a class whose established build functions are one name
//!   (`Styles/A-B.xaml` and `Styles/A_B.xaml`) get functions of two names.

// Generated code names the types of this crate by their absolute paths.
extern crate self as generated_rust_fixture_consumer;

mod collision_styles;
mod rust_paths;

// `compiled_xaml` (the documents of `documents::DOCUMENTS`, compiled as one group),
// `compiled_collision_styles` (the document of [`CollisionStyles`]) and `compiled_markup` (the
// loader table of the crate, the parts tables of the themes and `register()`).
ferroui_markup_xaml::include_compiled_xaml!();

pub use collision_styles::CollisionStyles;

use ferroui_base::metadata::MarkupAssembly;
use ferroui_base::TypeInfo;

/// The dotted namespaces of the modules of this crate.
const NAMESPACES: &[(&str, &str)] = &[("generated_rust_fixture_consumer", "GeneratedRust.Consumer")];

/// What this crate states about itself for markup. The build script reads it from this file.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "GeneratedRust.Consumer",
    crate_name: "generated_rust_fixture_consumer",
    xmlns_definitions: &[],
    xmlns_prefixes: &[],
    metadata: &[],
};

const TYPES: &[&TypeInfo] = &[CollisionStyles::TYPE];

/// Registers the namespaces, the types, the assembly and the compiled markup of this crate (and of
/// the crates it is built on). Cheap and idempotent.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        ferroui_markup_xaml::register_types();
        generated_rust_fixture_library::register_types();
        TypeInfo::register_namespaces(NAMESPACES);
        TypeInfo::register_all(TYPES);
        rust_paths::register_rust_paths();
        MarkupAssembly::register(&ASSEMBLY);
        compiled_markup::register();
    });
}
