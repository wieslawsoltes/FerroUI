mod dockable_options_view;
/// Original document view.
pub mod documents;
/// Original tool views.
pub mod tools;
pub use dockable_options_view::DockableOptionsView;
/// Original navigation view namespace.
#[allow(clippy::module_inception)] // Preserves the original Views.Views namespace.
pub mod views;

mod main_view;
pub use main_view::MainView;
mod main_window;
pub use main_window::MainWindow;
