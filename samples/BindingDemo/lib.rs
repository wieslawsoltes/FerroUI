//! binding-demo
//!
//! Port of the sample application `BindingDemo`: one window with a tab per kind of binding
//! (the modes of a binding, bindings into a collection, negation, numbers, element names and
//! resources as sources, a property set by a background thread, the stream operator, list
//! boxes that share a selection model, validation by exceptions, by error notifications and
//! by annotations, commands, and generic types named in markup). The entry point is
//! `program.rs`.
//!
//! The sample is markup: one document per class, which populates the class the document
//! names. The documents are converted from the upstream sample by
//! `scripts/convert_catalog_xaml.py` (names only) and compiled by the build of the crate
//! (`build.rs`, docs/porting/xaml.md 9.5.22): a class is populated by the compiled markup of
//! its document.
//!
//! The directories and files mirror the upstream sample: the document `TestItemView.xaml`
//! and its class in `test_item_view.rs`. What the framework lacks for the sample is listed in
//! `GAPS.md`.

// The compiled markup of a document names the types of this crate by the name of the crate.
extern crate self as binding_demo;

mod app;
mod generic_markup_extension;
mod generic_value_converter;
mod main_window;
pub mod markup;
mod register_types;
mod test_item_view;

#[path = "ViewModels/mod.rs"]
pub mod view_models;

// One module per compiled document and `compiled_markup` (the loader table of the compiled
// markup of the crate and `register()`), which the build script of the crate generates.
ferroui_markup_xaml::include_compiled_xaml!();

pub use app::App;
pub use generic_markup_extension::{GenericMarkupExtension, GenericMarkupExtensionOfColor};
pub use generic_value_converter::{GenericValueConverter, GenericValueConverterOfSolidColorBrush};
pub use main_window::MainWindow;
pub use markup::SAMPLE;
pub use register_types::{register_types, ASSEMBLY};
pub use test_item_view::TestItemView;

#[cfg(test)]
mod tests;
