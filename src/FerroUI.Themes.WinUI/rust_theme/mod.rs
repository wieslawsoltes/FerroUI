//! The WinUI theme written in Rust (#988, a prototype; docs/xaml-themes.md, section 10).
//!
//! One module per document of the theme, converted from the XAML by
//! `scripts/themes/xaml_to_rust.py` over the API of `ferroui_markup_xaml::rust_theme`, and the
//! units of the theme over those modules (`units`). With the feature `rust-theme` the crate is
//! built from these modules and the build script compiles no markup; the checked-in
//! `compiled_xaml.xamlmeta` still describes the units, so the applications that trim the theme
//! are built the same way.

/// What the modules of the documents import.
pub(crate) mod prelude {
    pub use ferroui_markup_xaml::rust_theme::prelude::*;
}

include!("documents.rs");
pub(crate) mod units;
