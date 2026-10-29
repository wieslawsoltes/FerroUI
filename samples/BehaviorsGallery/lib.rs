//! behaviors-gallery
//!
//! Port of the sample application of Xaml.Behaviors (`samples/BehaviorsTestApplication` of the
//! reference): a window with a searchable list of pages, one page per behavior, action or trigger
//! of the behavior crates (`src/Behaviors`), each page a view of its own with its document.
//!
//! The library is the gallery: the application ([`App`], `App.xaml`), the window and its view
//! ([`views`]: `MainWindow`, `MainView`, `SampleView`), what the pages share (the view models
//! [`view_models`], the models [`models`], the controls [`controls`], the converters
//! [`converters`], the dialogs [`dialogs`], the behaviors of the sample [`behaviors`], the
//! animations [`animations`], the renderers [`renderers`] and the triggers [`triggers`]) and the
//! pages ([`pages`]: one module and one document per page, listed in the table [`pages::PAGES`]
//! in the order of the reference; a page not ported yet shows a placeholder). The documents are
//! compiled by the build of the crate (`build.rs`, docs/porting/xaml.md 9.5.22). The entry point
//! is `program.rs`; `PAGES.md` lists the pages and how a page is ported.

// The compiled markup of a document names the types of this crate by the name of the crate.
extern crate self as behaviors_gallery;

mod app;
pub mod markup;
mod register_types;
pub mod smoke;

#[path = "Animations/mod.rs"]
pub mod animations;
#[path = "Behaviors/mod.rs"]
pub mod behaviors;
#[path = "Controls/mod.rs"]
pub mod controls;
#[path = "Converters/mod.rs"]
pub mod converters;
#[path = "Dialogs/mod.rs"]
pub mod dialogs;
#[path = "Models/mod.rs"]
pub mod models;
pub mod pages;
#[path = "Renderers/mod.rs"]
pub mod renderers;
#[path = "Triggers/mod.rs"]
pub mod triggers;
#[path = "ViewModels/mod.rs"]
pub mod view_models;
#[path = "Views/mod.rs"]
pub mod views;

// One module per compiled document and `compiled_markup` (the loader table of the compiled
// markup of the crate and `register()`), which the build script of the crate generates.
ferroui_markup_xaml::include_compiled_xaml!();

pub use app::App;
pub use markup::SAMPLE;
pub use pages::{PageInfo, PAGES};
pub use register_types::{register_types, ASSEMBLY};

#[cfg(test)]
mod tests;
