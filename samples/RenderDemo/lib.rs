//! render-demo
//!
//! Port of the sample application `RenderDemo`: pages of custom drawing through the drawing
//! context, animations, transitions, composition, bitmaps and text, in a hamburger menu
//! (`control-samples`). The entry point is `program.rs`.
//!
//! The sample is markup: one document per page, which populates the class the document names.
//! The documents are converted from the upstream sample by `scripts/convert_catalog_xaml.py`
//! (names only) and compiled by the build of the crate (`build.rs`, docs/porting/xaml.md
//! 9.5.22): a class is populated by the compiled markup of its document.
//!
//! The directories and files mirror the upstream sample: the document `Pages/BrushesPage.xaml`
//! and its class in `Pages/brushes_page.rs`.

// The compiled markup of a document names the types of this crate by the name of the crate.
extern crate self as render_demo;

mod app;
mod main_window;
pub mod markup;
mod register_types;

#[path = "Controls/mod.rs"]
pub mod controls;
#[path = "Pages/mod.rs"]
pub mod pages;
#[path = "ViewModels/mod.rs"]
pub mod view_models;

// One module per compiled document and `compiled_markup` (the loader table of the compiled
// markup of the crate and `register()`), which the build script of the crate generates.
ferroui_markup_xaml::include_compiled_xaml!();

pub use app::App;
pub use main_window::MainWindow;
pub use markup::SAMPLE;
pub use register_types::{register_types, ASSEMBLY};

#[cfg(test)]
mod tests;
