//! Compiles the markup of the crate and embeds its assets
//! (docs/porting/xaml.md, 9.5.13 and 9.6).
//!
//! The counterpart of the resource items of the upstream project file and of
//! its markup compiler: `AboutFerroDialog.xaml`, the document of the class
//! [`AboutFerroDialog`](../about_ferro_dialog_xaml.rs), is compiled to
//! `$OUT_DIR/xaml/compiled_about_ferro_dialog.rs`, and everything under
//! `Assets/` is embedded as an asset of the assembly. A compiled document is
//! not an asset: the loader table of the compiled markup answers a load by its
//! URI, as upstream's compiler removes every compiled resource from the
//! assembly.
//!
//! The compiler reads the types from the type models: the one of this crate,
//! scanned from its sources, and the ones of the crates it is built on, which
//! Cargo hands to this script as `DEP_<CRATE>_XAML_XAMLMETA`. The script links
//! the compiler and no crate for its types. The build also writes the
//! `.xamlmeta` of the crate, with its type model and its compiled document,
//! and Cargo hands the path of the file to the build scripts of the crates
//! that depend on this one (`DEP_FERROUI_DIALOGS_XAML_XAMLMETA`): the themes,
//! whose documents name the types of the dialogs.

use std::env;
use std::fs;
use std::path::PathBuf;

use ferroui_build::{Build, TypeSystem, XamlGroup};

/// The document of the class `AboutFerroDialog`, by its path below the crate directory.
const ABOUT_FERRO_DIALOG: &str = "AboutFerroDialog.xaml";

fn main() {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let path = root.join(ABOUT_FERRO_DIALOG);
    println!("cargo::rerun-if-changed={}", path.display());
    let about_ferro_dialog = fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));

    Build::from_env()
        .type_system(TypeSystem::Model)
        // The upstream project is built with compiled bindings as the default of its documents.
        .default_compile_bindings(true)
        .embed_assets(&["Assets"])
        .compile_group(
            XamlGroup::new("compiled_about_ferro_dialog")
                .documents(&[(ABOUT_FERRO_DIALOG, about_ferro_dialog.as_str())])
                .class_document(ABOUT_FERRO_DIALOG),
        )
        .run();
}
