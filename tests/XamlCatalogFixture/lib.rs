//! Test fixture of the XAML compiler (docs/porting/xaml.md, 9.5.17, 9.5.18 and 9.10.1,
//! item 2): the ControlCatalog with its documents compiled by a build script and its
//! classes populated by their compiled markup. Not a port of an upstream project, and not a
//! copy of the sample: the modules of this crate are the source files of
//! `samples/ControlCatalog`, read where they are (`#[path]`), and the documents are its
//! files. The sample itself is not changed and still loads its documents with the
//! run-time loader.
//!
//! The assembly is the assembly of the sample (`ControlCatalog`), so that the documents
//! name their classes and each other as they do there. What is the fixture's own:
//!
//! - [`markup`]: the module of the sample with `load_component` (what
//!   `initialize_component()` of a class calls) populating a class from the compiled markup
//!   of its document when the build compiled it;
//! - `register_types`: the table of the sample with the modules of this crate;
//! - `assets`: the module of the sample over the tables the build script of this crate
//!   writes (the documents only);
//! - [`fixture_documents`]: which documents the build compiles (every one the compiler
//!   does not refuse with the feature `catalog`, seven pages without it).
//!
//! `tests` compares each class populated by its compiled markup with the same class
//! populated by the run-time loader from the same document.

// The compiled markup of a document without a class (`CustomThemes.xaml`) names the types of
// this crate by the name of the crate, as the compiled markup of another crate would; the
// compiled markup of a class names them through `crate`.
extern crate self as xaml_catalog_fixture;

use crate::markup::XamlClass;
use ferroui_base::metadata::MarkupType;
use ferroui_base::TypeInfo;

#[path = "../../samples/ControlCatalog/app.rs"]
mod app;
mod assets;
#[path = "../../samples/ControlCatalog/decorated_window.rs"]
mod decorated_window;
#[path = "documents.rs"]
pub mod fixture_documents;
#[path = "../../samples/ControlCatalog/icons.rs"]
mod icons;
#[path = "../../samples/ControlCatalog/main_view.rs"]
mod main_view;
#[path = "../../samples/ControlCatalog/main_window.rs"]
mod main_window;
pub mod markup;
#[path = "../../samples/ControlCatalog/page_assets.rs"]
mod page_assets;
mod register_types;
#[path = "../../samples/ControlCatalog/smoke.rs"]
mod smoke;
#[path = "../../samples/ControlCatalog/transparent_styles.rs"]
mod transparent_styles;

#[path = "../../samples/ControlCatalog/Controls/mod.rs"]
pub mod controls;
#[path = "../../samples/ControlCatalog/Converter/mod.rs"]
pub mod converter;
#[path = "../../samples/ControlCatalog/Models/mod.rs"]
pub mod models;
#[path = "../../samples/ControlCatalog/Pages/mod.rs"]
pub mod pages;
#[path = "../../samples/ControlCatalog/ViewModels/mod.rs"]
pub mod view_models;
#[path = "../../samples/ControlCatalog/Views/mod.rs"]
pub mod views;

// One module per compiled document (`compiled_pages_check_box_page`, ..: the function
// `populate` of the document of a class) and `compiled_markup` (the loader table of the
// compiled markup of the crate).
ferroui_markup_xaml::include_compiled_xaml!();

pub use app::App;
pub use assets::{documents, excluded, excluded_documents, ExcludedDocument};
pub use decorated_window::DecoratedWindow;
pub use main_view::MainView;
pub use main_window::MainWindow;
pub use page_assets::{IPageAssets, PageAssets, PageAssetsFuture};
pub use register_types::{register_types, ASSEMBLY};
pub use smoke::show_every_page;
pub use transparent_styles::TransparentStyles;

/// The classes of the root namespace `ControlCatalog` (`X::TYPE`).
pub(crate) const ROOT_TYPES: &[&TypeInfo] = &[App::TYPE, DecoratedWindow::TYPE, MainView::TYPE, MainWindow::TYPE, TransparentStyles::TYPE];

/// The classes of the root namespace that have a document (`&X::XAML_CLASS`).
pub(crate) const ROOT_CLASSES: &[&XamlClass] =
    &[&App::XAML_CLASS, &DecoratedWindow::XAML_CLASS, &MainView::XAML_CLASS, &MainWindow::XAML_CLASS, &TransparentStyles::XAML_CLASS];

/// The types of the root namespace declared with `ferro_markup_type!` / `ferro_markup_enum!`.
pub(crate) const ROOT_MARKUP_TYPES: &[&MarkupType] = &[];

/// What the untyped value conversions must know about the types of the root namespace.
pub(crate) fn register_root_value_types() {}

#[cfg(test)]
mod tests;
