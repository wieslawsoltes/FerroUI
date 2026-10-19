//! reactive-demo
//!
//! Port of `ReactiveDemo`, the getting-started sample of ReactiveUI
//! (`ReactiveUI.Samples/wpf/getting-started/ReactiveDemo`): a browser of the crates of crates.io,
//! where the reference browses the packages of NuGet. The search term of a text box is throttled,
//! trimmed, deduplicated and searched; the results are an output property of the view model, shown
//! in a list whose items are the views located for their view models; each item opens the page of
//! its crate with a command.
//!
//! The port is file for file:
//!
//! | Reference | Port |
//! |---|---|
//! | `App.xaml`, `App.xaml.cs` | `App.xaml`, [`app`](App) |
//! | `AppViewModel.cs` | [`app_view_model`](AppViewModel) |
//! | `NugetDetailsViewModel.cs` | [`crate_details_view_model`](CrateDetailsViewModel) |
//! | `MainWindow.xaml`, `MainWindow.xaml.cs` | `MainWindow.xaml`, [`main_window`](MainWindow) |
//! | `NugetDetailsView.xaml`, `NugetDetailsView.xaml.cs` | `CrateDetailsView.xaml`, [`crate_details_view`](CrateDetailsView) |
//!
//! The port adds what the reference takes from libraries: the search backend
//! ([`crate_search`]: NuGet.Protocol upstream, the API of crates.io here), the view of a class of
//! a document as ReactiveUI binds it ([`code_behind_view`]: a class is its own `IViewFor`
//! upstream) and the entry point (`program.rs`).
//!
//! The documents are compiled by the build of the crate (`build.rs`, docs/porting/xaml.md
//! 9.5.22).

// The compiled markup of a document names the types of this crate by the name of the crate.
extern crate self as reactive_demo;

mod app;
mod app_view_model;
pub mod code_behind_view;
mod crate_details_view;
mod crate_details_view_model;
pub mod crate_search;
mod main_window;
pub mod markup;
mod register_types;

// One module per compiled document and `compiled_markup` (the loader table of the compiled
// markup of the crate and `register()`), which the build script of the crate generates.
ferroui_markup_xaml::include_compiled_xaml!();

pub use app::{register_views, view_assembly, App};
pub use app_view_model::{AppViewModel, SearchResults};
pub use crate_details_view::{CrateDetailsView, CrateDetailsViewFor};
pub use crate_details_view_model::{format_thousands, CrateDetailsViewModel, LauncherSource};
pub use main_window::{MainWindow, MainWindowView};
pub use markup::SAMPLE;
pub use register_types::{register_types, ASSEMBLY};

#[cfg(test)]
mod tests;
