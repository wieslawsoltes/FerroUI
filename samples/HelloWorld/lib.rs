//! hello-world
//!
//! The smallest application of FerroUI in the WinUI theme: `App.xaml` (the application, with
//! `<WinUITheme />`) and `MainWindow.xaml` (a window with "Hello, World!" in its middle), each
//! of which populates the class it names, compiled by the build of the crate (`build.rs`,
//! docs/porting/xaml.md 9.5.22). It depends on nothing it does not use, so its executable is the
//! size floor of an application in that theme (docs/binary-size.md). The entry point is
//! `program.rs`.

// The compiled markup of a document names the types of this crate by the name of the crate.
extern crate self as hello_world;

mod app;
mod main_window;
pub mod markup;
mod register_types;

// One module per compiled document and `compiled_markup` (the loader table of the compiled
// markup of the crate and `register()`), which the build script of the crate generates.
ferroui_markup_xaml::include_compiled_xaml!();

pub use app::App;
pub use main_window::MainWindow;
pub use markup::SAMPLE;
pub use register_types::{register_types, ASSEMBLY};

#[cfg(test)]
mod tests;
