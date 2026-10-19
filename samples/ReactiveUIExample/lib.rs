//! reactiveui-example
//!
//! Port of the example application of the ReactiveUI integration with the reference UI framework
//! (`ReactiveUI.<framework>.Example`): a shell with a sidebar and a `RoutedViewHost`, whose routed
//! pages show an overview of the features (an `ItemsControl` whose rows the automatic data-template
//! hook resolves), live measurements of this process (metric cards resolved by
//! `ViewModelViewHost` with view contracts, a computed label, the observation of control
//! properties), and a command lab (can-execute, asynchronous execution, failures reported through
//! an interaction). The input of the command lab and the threshold of the metrics are saved on exit
//! and restored on launch with `AutoSuspendHelper`.
//!
//! The DynamicData page is an addition of the port with no counterpart in the reference: a
//! `SourceCache` filtered by a throttled search, sorted, transformed into item view models and bound
//! with `sort_and_bind`, with `auto_refresh` on a favourite flag.
//!
//! The entry point is `program.rs`. Each view is a markup document with its code (`Views/`),
//! compiled by the build of the crate (`build.rs`, docs/porting/xaml.md 9.5.22).
//!
//! The Android, iOS and browser heads of the reference are not ported: FerroUI builds its own
//! Android, iOS and browser applications, and the shared library runs in them (the single-view
//! lifetime gets a [`MainView`](views::MainView), and the browser gets
//! [`BrowserMachineMetricsService`](services::BrowserMachineMetricsService)).

// The compiled markup of a document names the types of this crate by the name of the crate.
extern crate self as reactiveui_example;

mod app;
mod example_app_builder_extensions;
pub mod markup;
mod register_types;

#[path = "Models/mod.rs"]
pub mod models;
#[path = "Services/mod.rs"]
pub mod services;
#[path = "ViewModels/mod.rs"]
pub mod view_models;
#[path = "Views/mod.rs"]
pub mod views;

// One module per compiled document and `compiled_markup` (the loader table of the compiled
// markup of the crate and `register()`), which the build script of the crate generates.
ferroui_markup_xaml::include_compiled_xaml!();

pub use app::App;
pub use example_app_builder_extensions::{ExampleAppBuilderExt, ExampleAppBuilderExtensions};
pub use markup::SAMPLE;
pub use register_types::{register_types, ASSEMBLY};

#[cfg(test)]
mod tests;
