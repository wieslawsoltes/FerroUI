//! WinUI XAML compatibility of FerroUI: an opt-in importer that migrates WinUI XAML to FerroUI's
//! XAML, the loading entry points that run it before the run-time loader, and the x:Bind
//! property-path adapter that the migrated documents use (docs/porting/winui-controls.md and
//! the plan of #286, W9).
//!
//! Nothing here changes the default loader or the ordinary bindings: an application opts in by
//! calling [`migrate_winui_xaml`] (to feed a build-time compiler) or [`load_winui`],
//! [`parse_winui`] and [`parse_winui_as`] (run time), and by calling [`register_types`] so that
//! markup can name the x:Bind types (namespace `FerroUI.Markup.Xaml.WinUI`, reached through the
//! `using:` prefix the importer writes).

// Generated code names the crate by its name, also in the tests of the crate itself.
extern crate self as ferroui_markup_xaml_winui;

mod bindings;
mod importer;
mod load;
mod register_types;
mod rust_paths;

pub use bindings::{
    WinUIBindExtension, WinUIBinding, WinUIBindingDiagnostic, WinUIBindingErrors, WinUIBindingScope, WinUIBindings,
};
pub use importer::{
    migrate_winui_xaml, WinUIThemeResourcePolicy, WinUIXBindPolicy, WinUIXamlDiagnostic, WinUIXamlDiagnosticSeverity,
    WinUIXamlEdit, WinUIXamlMemberMapping, WinUIXamlMigration, WinUIXamlMigrationError, WinUIXamlNamespaceMapping,
    WinUIXamlOptions, WinUIXamlSourceSpan, WinUIXamlTypeMapping, WINUI_PRESENTATION_NAMESPACE,
};
pub use load::{load_winui, parse_winui, parse_winui_as};
pub use register_types::{register_types, ASSEMBLY};

/// Handles of the crate compare by identity (the reference equality of the managed originals).
macro_rules! identity_eq {
    ($($type_:ty),* $(,)?) => {$(
        impl PartialEq for $type_ {
            fn eq(&self, other: &Self) -> bool {
                ::std::ptr::addr_eq(self, other)
            }
        }
    )*};
}
pub(crate) use identity_eq;

/// The error of a fallible member called through metadata: thrown as the runtime of
/// `ferroui-markup-xaml` throws (into a scope of `xaml_error_scope::with_xaml_errors`, or as a
/// panic with its message).
pub(crate) fn throw<T>(result: Result<T, ferroui_markup_xaml::XamlLoadException>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => ferroui_markup_xaml::xaml_error_scope::throw(error),
    }
}

#[cfg(test)]
mod bindings_tests;
#[cfg(test)]
mod compiled_tests;
#[cfg(test)]
mod importer_tests;
