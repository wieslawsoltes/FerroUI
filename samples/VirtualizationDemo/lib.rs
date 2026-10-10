//! virtualization-demo
//!
//! Port of the sample application `VirtualizationDemo`: three pages of list boxes over many
//! items in a hamburger menu (`control-samples`): a playground of a thousand items with the
//! selection modes, scrolling to an item, adding and removing; a chat whose messages have
//! varying heights; and a hundred expanders. The entry point is `program.rs`.
//!
//! The sample is markup: one document per view, which populates the class the document
//! names. The documents are converted from the upstream sample by
//! `scripts/convert_catalog_xaml.py` (names only) and compiled by the build of the crate
//! (`build.rs`, docs/porting/xaml.md 9.5.22): a class is populated by the compiled markup of
//! its document.
//!
//! The directories and files mirror the upstream sample: the document
//! `Views/ChatPageView.xaml` and its class in `Views/chat_page_view.rs`. What the framework
//! lacks for the sample is listed in `GAPS.md`.

// The compiled markup of a document names the types of this crate by the name of the crate.
extern crate self as virtualization_demo;

mod app;
mod main_window;
pub mod markup;
mod register_types;

#[path = "Models/mod.rs"]
pub mod models;
#[path = "ViewModels/mod.rs"]
pub mod view_models;
#[path = "Views/mod.rs"]
pub mod views;

// One module per compiled document and `compiled_markup` (the loader table of the compiled
// markup of the crate and `register()`), which the build script of the crate generates.
ferroui_markup_xaml::include_compiled_xaml!();

pub use app::App;
pub use main_window::MainWindow;
pub use markup::SAMPLE;
pub use register_types::{register_types, ASSEMBLY};

#[cfg(test)]
mod tests;
