//! WinUI XAML compatibility of FerroUI: an opt-in importer that migrates WinUI XAML to FerroUI's
//! XAML, and the loading entry points that run it before the run-time loader (docs/porting/winui-controls.md
//! and the plan of #286, W9).
//!
//! Nothing here changes the default loader or the ordinary bindings: an application opts in by
//! calling [`migrate_winui_xaml`] (to feed a build-time compiler) or [`load_winui`],
//! [`parse_winui`] and [`parse_winui_as`] (run time). WinUI's `{x:Bind}` is kept as written and
//! compiled by the x:Bind core of the XAML compiler (`XBindExtension` and `XBindings` of
//! `ferroui_markup_xaml::markup_extensions`, docs/porting/x-bind.md); [`WinUIXBindPolicy`]
//! decides whether it is compiled or reported.

// Generated code names a crate by its name; this one too, also in the tests of the crate itself.
extern crate self as ferroui_markup_xaml_winui;

mod importer;
mod load;
mod register_types;
mod rust_paths;

pub use importer::{
    migrate_winui_xaml, WinUIThemeResourcePolicy, WinUIXBindPolicy, WinUIXamlDiagnostic, WinUIXamlDiagnosticSeverity,
    WinUIXamlEdit, WinUIXamlMemberMapping, WinUIXamlMigration, WinUIXamlMigrationError, WinUIXamlNamespaceMapping,
    WinUIXamlOptions, WinUIXamlSourceSpan, WinUIXamlTypeMapping, WINUI_PRESENTATION_NAMESPACE,
};
pub use load::{load_winui, parse_winui, parse_winui_as};
pub use register_types::{register_types, ASSEMBLY};

#[cfg(test)]
mod compiled_tests;
#[cfg(test)]
mod importer_tests;
#[cfg(test)]
mod x_bind_tests;
