//! Test fixture of the XAML compiler (#849): a library, the assembly `GeneratedRust.Library`, whose
//! classes the compiled documents of `generated-rust-fixture-consumer` construct. Not a port of an
//! upstream project.
//!
//! The document of every class merges a dictionary of its own, so the compiled markup of every
//! class is partitioned into units (#568, docs/porting/theme-trimming.md). Only some of the classes
//! can be constructed from selected units:
//!
//! - [`App`], an application class with `new()` only, and [`RenamedTheme`], a theme whose
//!   constructor from units has another name (`from_parts`): another crate constructs them with
//!   `new()`, whatever their markup is partitioned into.
//! - [`PartsTheme`] (`GeneratedRust.Library.A_B.PartsTheme`) and [`Partstheme`]
//!   (`GeneratedRust.Library.A.B.Partstheme`), themes with `with_parts`: another crate that trims
//!   its themes constructs them from the units it keeps. Their full names differ in case and
//!   punctuation only, so the established names of their parts tables are the same.
//!
//! The build script (`build.rs`, `ferroui-build`) generates the compiled markup into `OUT_DIR`; the
//! consumer compiles what its own build script generates from the `.xamlmeta` of this crate.

// Generated code names the types of this crate by their absolute paths.
extern crate self as generated_rust_fixture_library;

mod app;
mod rust_paths;
mod themes;

// `compiled_app`, `compiled_parts_theme`, `compiled_parts_theme_lower`, `compiled_renamed_theme`
// (the document of each class) and `compiled_markup` (the loader table of the crate and
// `register()`).
ferroui_markup_xaml::include_compiled_xaml!();

pub use app::App;
pub use themes::{PartsTheme, Partstheme, RenamedTheme};

use ferroui_base::metadata::MarkupAssembly;
use ferroui_base::TypeInfo;

/// The dotted namespaces of the modules of this crate.
const NAMESPACES: &[(&str, &str)] = &[("generated_rust_fixture_library", "GeneratedRust.Library")];

/// What this crate states about itself for markup. The build script reads it from this file.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "GeneratedRust.Library",
    crate_name: "generated_rust_fixture_library",
    xmlns_definitions: &[],
    xmlns_prefixes: &[],
    metadata: &[],
};

const TYPES: &[&TypeInfo] = &[App::TYPE, PartsTheme::TYPE, Partstheme::TYPE, RenamedTheme::TYPE];

/// Registers the namespaces, the types, the assembly and the compiled markup of this crate (and of
/// the crates it is built on). Cheap and idempotent.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        ferroui_markup_xaml::register_types();
        TypeInfo::register_namespaces(NAMESPACES);
        TypeInfo::register_all(TYPES);
        rust_paths::register_rust_paths();
        MarkupAssembly::register(&ASSEMBLY);
        compiled_markup::register();
    });
}
