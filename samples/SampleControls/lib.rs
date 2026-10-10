//! control-samples
//!
//! Port of the sample library `SampleControls` (the assembly `ControlSamples`): the shared
//! controls of the samples. One file per upstream file: the hamburger menu
//! ([`HamburgerMenu`], `HamburgerMenu/hamburger_menu.rs`) and its control theme
//! (`HamburgerMenu/HamburgerMenu.xaml`, converted from the upstream document by
//! `scripts/convert_catalog_xaml.py`, names only), which a sample merges into the resources
//! of its application:
//!
//! ```xml
//! <ResourceInclude Source="ferres://ControlSamples/HamburgerMenu/HamburgerMenu.xaml" />
//! ```
//!
//! The document is compiled by the build of the crate (`build.rs`): [`register_types`]
//! registers the loader table of the compiled markup, which answers a load of the document by
//! its URI, and the build of a sample links an include of the document to its compiled markup.

// The compiled markup names the types of the crate by the name of the crate, as the markup of
// any other crate does.
extern crate self as control_samples;

// `compiled_xaml` (the compiled markup of the theme document) and `compiled_markup` (the
// loader table of the crate and `register()`), which the build script generates.
ferroui_markup_xaml::include_compiled_xaml!();

#[path = "HamburgerMenu/hamburger_menu.rs"]
mod hamburger_menu;

pub use hamburger_menu::HamburgerMenu;

#[cfg(feature = "document-assets")]
mod document_assets {
    include!(concat!(env!("OUT_DIR"), "/assets.rs"));
}

use ferroui_base::metadata::MarkupAssembly;
use ferroui_base::TypeInfo;

/// The dotted namespaces of the modules of this crate.
const NAMESPACES: &[(&str, &str)] = &[("control_samples", "ControlSamples")];

/// What this crate states about itself for markup.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "ControlSamples",
    crate_name: "control_samples",
    xmlns_definitions: &[],
    xmlns_prefixes: &[],
    metadata: &[],
};

const TYPES: &[&TypeInfo] = &[HamburgerMenu::TYPE];

/// Registers the namespaces, the types, the assembly and the compiled markup of this crate
/// (and of the crates it is built on). Cheap and idempotent.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        // The runtime library of compiled markup, which registers the controls.
        ferroui_markup_xaml::register_types();
        TypeInfo::register_namespaces(NAMESPACES);
        TypeInfo::register_all(TYPES);
        MarkupAssembly::register(&ASSEMBLY);
        // The theme document as an asset: what lets a document the run-time loader loads
        // include it (the tests of the samples do).
        #[cfg(feature = "document-assets")]
        ferroui_base::platform::register_assets(ASSEMBLY.name, document_assets::ASSETS);
        // The loader table of the compiled markup, which the build of the crate generates: a
        // load of the theme document by its URI builds it from its compiled markup.
        compiled_markup::register();
    });
}
