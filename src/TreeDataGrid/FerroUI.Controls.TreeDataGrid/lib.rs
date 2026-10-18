//! ferroui-controls-treedatagrid
//!
//! The TreeDataGrid control, ported from the control project of
//! TreeDataGrid (<https://github.com/wieslawsoltes/TreeDataGrid>): hierarchical and tabular data
//! in one virtualizing view, with column layout, sorting, row and cell selection, editing, drag and
//! drop and automation. The data layer is `ferroui-treedatagrid-core` (sources, columns, rows and
//! selection), which the control presents (docs/porting/treedatagrid.md).
//!
//! The control themes are not part of the base themes: an application includes the styles of the
//! library, as the reference's applications do:
//!
//! ```xml
//! <StyleInclude Source="ferres://FerroUI.Controls.TreeDataGrid/Themes/Fluent.xaml" />
//! ```

// The compiled markup names the types of the crate by the name of the crate, as the markup of any
// other crate does.
extern crate self as ferroui_controls_treedatagrid;

// `compiled_xaml` (the compiled markup of the theme documents) and `compiled_markup` (the loader
// table of the crate and `register()`), which the build script generates.
ferroui_markup_xaml::include_compiled_xaml!();

#[cfg(any(test, feature = "document-assets"))]
mod assets;
pub mod automation;
pub mod converters;
mod markup_types;
pub mod models;
pub mod presentation;
pub mod primitives;
mod register_types;
mod root;
mod rust_paths;
pub mod selection;

pub use register_types::{register_types, ASSEMBLY};
pub use root::*;

/// The data layer of the control.
pub use ferroui_treedatagrid_core as core;

#[cfg(test)]
mod tests;
